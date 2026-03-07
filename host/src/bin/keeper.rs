//! ZK Price Oracle Keeper -- automated price feed updates.
//!
//! Runs a loop that checks two triggers:
//! 1. **Heartbeat**: Time elapsed since last update exceeds the configured interval.
//! 2. **Deviation**: Current spot price deviates from the on-chain price by more than a threshold.
//!
//! When triggered, submits a proof request to Boundless Market with a callback so the
//! oracle contract is updated automatically on fulfillment.

use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;
use anyhow::{Context, Result, ensure};
use boundless_market::{Client, Deployment, StorageUploaderConfig, request_builder::RequirementParams};
use clap::{Parser, ValueEnum};
use risc0_steel::{
    Contract, EvmBlockHeader,
    alloy::{
        network::EthereumWallet,
        providers::ProviderBuilder,
    },
    ethereum::{EthChainSpec, EthEvmEnv},
    config::ChainSpec,
};
use std::path::PathBuf;
use std::time::Duration;
use tracing_subscriber::EnvFilter;
use url::Url;
use zk_oracle_methods::ZK_ORACLE_GUEST_ELF;

// ─── DEX interfaces ──────────────────────────────────────────────────────────

sol! {
    #[sol(rpc)]
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }

    #[sol(rpc)]
    interface IUniswapV3Pool {
        function slot0() external view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked);
        function observe(uint32[] secondsAgos) external view returns (int56[] tickCumulatives, uint160[] secondsPerLiquidityCumulativeX128s);
    }

    interface IAlgebraPool {
        function globalState() external view returns (uint160 price, int24 tick, uint16 feeZto, uint16 feeOtz, uint16 timepointIndex, uint8 communityFeeToken0, uint8 communityFeeToken1);
    }

    interface ICurvePool {
        function balances(uint256 i) external view returns (uint256);
    }

    interface IBalancerVault {
        function getPoolTokens(bytes32 poolId) external view returns (address[] tokens, uint256[] balances, uint256 lastChangeBlock);
    }

    interface IBalancerWeightedPool {
        function getNormalizedWeights() external view returns (uint256[]);
    }

    interface ILBPair {
        function getActiveId() external view returns (uint24);
        function getBinStep() external view returns (uint16);
    }

    #[sol(rpc)]
    interface IDODO {
        function getMidPrice() external view returns (uint256);
    }
}

sol! {
    struct OracleJournal {
        bytes[] steelCommitments;
        bytes32 configHash;
        address pool;
        uint256 medianPrice;
        uint8 journalDecimals;
        uint32 numSamples;
        uint64 startBlock;
        uint64 endBlock;
        uint64 timestamp;
    }
}

sol! {
    #[sol(rpc)]
    interface IZkOracle {
        function updatePrice(bytes calldata journalData, bytes calldata seal) external;
        function imageId() external view returns (bytes32);
        function currentRoundId() external view returns (uint80);
        function latestRoundData() external view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);
        function BOUNDLESS_MARKET() external view returns (address);
    }
}

/// Supported DEX types.
#[derive(Clone, Copy, Debug, ValueEnum, serde::Serialize)]
#[repr(u8)]
enum DexType {
    UniswapV2 = 0,
    UniswapV3 = 1,
    CurveStableSwap = 2,
    BalancerV2 = 3,
    SolidlyStable = 4,
    TraderJoeLB = 5,
    DodoPMM = 6,
    AlgebraV3 = 7,
}

/// Guest input.
#[derive(serde::Serialize)]
struct OracleConfig {
    pool: Address,
    decimals: u8,
    dex_type: DexType,
    aux_address: Option<Address>,
    aux_bytes32: Option<[u8; 32]>,
    token0_decimals: Option<u8>,
    token1_decimals: Option<u8>,
    twap_window: u32,
    chain_spec: EthChainSpec,
}

// ─── Persistent state ────────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct KeeperState {
    last_request_id: Option<String>,
    last_request_time: Option<u64>,
    last_success_time: Option<u64>,
    consecutive_failures: u32,
    last_round_id: u64,
}

impl KeeperState {
    fn load(path: &PathBuf) -> Self {
        match std::fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("[keeper] Warning: failed to parse state file {}: {e}", path.display());
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    fn save(&self, path: &PathBuf) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                // Atomic write: write to temp file then rename to avoid partial writes.
                let tmp = path.with_extension("json.tmp");
                if let Err(e) = std::fs::write(&tmp, &json) {
                    eprintln!("[keeper] Warning: failed to write temp state file {}: {e}", tmp.display());
                    return;
                }
                if let Err(e) = std::fs::rename(&tmp, path) {
                    eprintln!("[keeper] Warning: failed to rename state file: {e}");
                    // Fallback: try direct write
                    let _ = std::fs::write(path, json);
                }
            }
            Err(e) => {
                eprintln!("[keeper] Warning: failed to serialize state: {e}");
            }
        }
    }
}

// ─── RPC fallback helper ─────────────────────────────────────────────────────

async fn with_rpc_fallback<T, F, Fut>(rpc_urls: &[Url], f: F) -> Result<T>
where
    F: Fn(Url) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_err = None;
    for url in rpc_urls {
        match f(url.clone()).await {
            Ok(val) => return Ok(val),
            Err(e) => {
                eprintln!("[keeper] RPC {} failed: {e:#}", url);
                last_err = Some(e);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no RPC URLs configured")))
}

// ─── CLI ─────────────────────────────────────────────────────────────────────

/// ZK Price Oracle Keeper -- automated price feed updates via heartbeat and deviation triggers.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Ethereum RPC URLs for reading pool state (comma-separated for fallback).
    #[arg(short, long, env = "BASE_MAINNET_RPC", value_delimiter = ',')]
    rpc_urls: Vec<Url>,

    /// RPC URL for the chain where the oracle contract is deployed.
    #[arg(long, env = "RPC_URL")]
    oracle_rpc_url: Url,

    /// Private key for Boundless market interactions.
    #[arg(long, env = "PRIVATE_KEY")]
    private_key: risc0_steel::alloy::signers::local::PrivateKeySigner,

    /// DEX pool / pair address.
    #[arg(short, long, env = "POOL_ADDRESS")]
    pool: Address,

    /// DEX type.
    #[arg(long, default_value = "uniswap-v2", env = "DEX_TYPE")]
    dex_type: DexType,

    /// Deployed ZkOracle contract address (also the callback target).
    #[arg(long, env = "ORACLE_ADDRESS")]
    oracle_address: Address,

    /// Number of blocks to sample per proof.
    #[arg(short, long, default_value = "5", env = "NUM_BLOCKS")]
    num_blocks: u32,

    /// Sample every Nth block.
    #[arg(short = 's', long, default_value = "20", env = "BLOCK_STRIDE")]
    stride: u64,

    /// Output price decimals.
    #[arg(short, long, default_value = "18", env = "PRICE_DECIMALS")]
    decimals: u8,

    /// Token0 decimals (needed for V3/Algebra price math).
    #[arg(long, env = "TOKEN0_DECIMALS")]
    token0_decimals: Option<u8>,

    /// Token1 decimals (needed for V3/Algebra price math).
    #[arg(long, env = "TOKEN1_DECIMALS")]
    token1_decimals: Option<u8>,

    /// Auxiliary address (e.g. Balancer Vault).
    #[arg(long, env = "AUX_ADDRESS")]
    aux_address: Option<Address>,

    /// Auxiliary bytes32 (e.g. Balancer pool ID, as hex).
    #[arg(long, env = "AUX_BYTES32")]
    aux_bytes32: Option<String>,

    /// TWAP window in seconds for V3 observe() (0 = use spot price).
    #[arg(long, default_value = "1800", env = "TWAP_WINDOW")]
    twap_window: u32,

    /// Source chain ID (default 8453 for Base mainnet).
    #[arg(long, default_value = "8453", env = "SOURCE_CHAIN_ID")]
    source_chain_id: u64,

    /// Lock timeout in seconds. Provers need time to generate the ZK proof, so avoid
    /// setting this too low. 600s (10 min) is a safe default; shorter values risk expiry.
    #[arg(long, default_value = "600", env = "LOCK_TIMEOUT")]
    lock_timeout: u32,

    /// Gas limit for the callback.
    #[arg(long, default_value = "500000", env = "CALLBACK_GAS_LIMIT")]
    callback_gas_limit: u64,

    // ─── Keeper trigger config ───────────────────────────────────────

    /// Heartbeat interval in seconds. Update if this much time passes since last update.
    #[arg(long, default_value = "3600", env = "HEARTBEAT_SECONDS")]
    heartbeat: u64,

    /// Deviation threshold in basis points. Update if spot price deviates this much from on-chain.
    #[arg(long, default_value = "100", env = "DEVIATION_BPS")]
    deviation_bps: u64,

    /// How often to check triggers (seconds).
    #[arg(long, default_value = "60", env = "CHECK_INTERVAL")]
    check_interval: u64,

    /// Pre-uploaded program URL (skips uploading the ELF).
    #[arg(long, env = "PROGRAM_URL")]
    program_url: Option<Url>,

    /// Path to the keeper state file for persistent state across restarts.
    #[arg(long, default_value = "keeper-state.json", env = "KEEPER_STATE_FILE")]
    state_file: PathBuf,

    /// Maximum consecutive failures before logging CRITICAL (keeps retrying at capped interval).
    #[arg(long, default_value = "5", env = "MAX_RETRIES")]
    max_retries: u32,

    /// Force manual submission mode: submit proof to Boundless, wait for fulfillment,
    /// then call updatePrice() directly instead of using the callback mechanism.
    /// Auto-detected when the oracle contract has BOUNDLESS_MARKET set to address(0).
    #[arg(long, default_value = "false", env = "MANUAL_SUBMISSION")]
    manual_submission: bool,

    /// Storage provider config.
    #[clap(flatten, next_help_heading = "Storage Uploader")]
    storage_config: StorageUploaderConfig,

    /// Boundless Market deployment config.
    #[clap(flatten, next_help_heading = "Boundless Market Deployment")]
    deployment: Option<Deployment>,
}

/// Fetch the current spot price from the DEX pool (simple RPC read, no proof).
async fn fetch_spot_price(rpc_urls: &[Url], pool: Address, dex_type: DexType, decimals: u8) -> Result<U256> {
    with_rpc_fallback(rpc_urls, |url| {
        let pool = pool;
        async move {
            let provider = ProviderBuilder::new().connect_http(url);

            match dex_type {
                DexType::UniswapV2 | DexType::SolidlyStable => {
                    let pair = IUniswapV2Pair::new(pool, &provider);
                    let res = pair.getReserves().call().await?;
                    let r0 = U256::from(res.reserve0);
                    let r1 = U256::from(res.reserve1);
                    if r0.is_zero() {
                        anyhow::bail!("reserve0 is zero");
                    }
                    let scale = U256::from(10u64).pow(U256::from(decimals));
                    Ok(r1 * scale / r0)
                }
                DexType::UniswapV3 => {
                    let p = IUniswapV3Pool::new(pool, &provider);
                    let slot = p.slot0().call().await?;
                    let sqrt_price = U256::from(slot.sqrtPriceX96);
                    let scale = U256::from(10u64).pow(U256::from(decimals));
                    // price = (sqrtPriceX96 / 2^96)^2 * scale
                    let q96 = U256::from(1u128 << 96);
                    Ok(sqrt_price * sqrt_price * scale / q96 / q96)
                }
                DexType::DodoPMM => {
                    let d = IDODO::new(pool, &provider);
                    let price = d.getMidPrice().call().await?;
                    Ok(price)
                }
                _ => {
                    anyhow::bail!("Spot price fetch not implemented for this DEX type; use Uniswap V2/V3 or DODO");
                }
            }
        }
    }).await
}

/// Fetch the last on-chain price, updatedAt timestamp, and current round ID from the oracle.
async fn fetch_on_chain_state(oracle_rpc_url: &Url, oracle_address: Address) -> Result<(U256, u64, u64)> {
    let provider = ProviderBuilder::new().connect_http(oracle_rpc_url.clone());
    let oracle = IZkOracle::new(oracle_address, &provider);

    let round_id = oracle.currentRoundId().call().await?;
    let round_id_u64: u64 = round_id.to::<u64>();
    if round_id_u64 == 0 {
        // No data yet -- trigger an update.
        return Ok((U256::ZERO, 0, 0));
    }

    let latest = oracle.latestRoundData().call().await?;
    let price = if !latest.answer.is_negative() {
        latest.answer.unsigned_abs()
    } else {
        U256::ZERO
    };
    let updated_at = latest.updatedAt.try_into().unwrap_or(0u64);
    Ok((price, updated_at, round_id_u64))
}

/// Return the current unix timestamp.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Check if the oracle contract has a Boundless Market configured for callback delivery.
async fn check_oracle_callback_support(oracle_rpc_url: &Url, oracle_address: Address) -> Result<bool> {
    let provider = ProviderBuilder::new().connect_http(oracle_rpc_url.clone());
    let oracle = IZkOracle::new(oracle_address, &provider);
    let market = oracle.BOUNDLESS_MARKET().call().await?;
    Ok(market != Address::ZERO)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    if args.rpc_urls.is_empty() {
        anyhow::bail!("At least one RPC URL is required (--rpc-urls or BASE_MAINNET_RPC)");
    }

    let dex_name = match args.dex_type {
        DexType::UniswapV2 => "Uniswap V2",
        DexType::UniswapV3 => "Uniswap V3",
        DexType::CurveStableSwap => "Curve",
        DexType::BalancerV2 => "Balancer V2",
        DexType::SolidlyStable => "Solidly Stable",
        DexType::TraderJoeLB => "Trader Joe LB",
        DexType::DodoPMM => "DODO PMM",
        DexType::AlgebraV3 => "Algebra V3",
    };

    // Detect callback support before printing config.
    let use_callback = if args.manual_submission {
        false
    } else {
        match check_oracle_callback_support(&args.oracle_rpc_url, args.oracle_address).await {
            Ok(supported) => supported,
            Err(e) => {
                eprintln!("[keeper] Warning: could not check oracle callback support: {e:#}");
                false
            }
        }
    };

    println!("ZK Price Oracle Keeper");
    println!("  Pool:       {:#}", args.pool);
    println!("  DEX:        {dex_name}");
    println!("  Oracle:     {:#}", args.oracle_address);
    println!("  Heartbeat:  {}s", args.heartbeat);
    println!("  Deviation:  {} bps", args.deviation_bps);
    println!("  Check:      every {}s", args.check_interval);
    if use_callback {
        println!("  Mode:       callback (Boundless Market delivers proofs to oracle)");
        println!("  Callback:   {:#} (gas limit: {})", args.oracle_address, args.callback_gas_limit);
    } else {
        println!("  Mode:       manual (keeper waits for proof and calls updatePrice directly)");
    }
    println!("  RPCs:       {} configured", args.rpc_urls.len());
    println!("  State file: {}", args.state_file.display());
    println!("  Max retries: {}", args.max_retries);
    println!();

    use risc0_steel::revm::primitives::hardfork::SpecId;
    let chain_spec: EthChainSpec = match args.source_chain_id {
        1 => risc0_steel::ethereum::ETH_MAINNET_CHAIN_SPEC.clone(),
        11155111 => risc0_steel::ethereum::ETH_SEPOLIA_CHAIN_SPEC.clone(),
        cid => ChainSpec::new_single(cid, SpecId::CANCUN),
    };

    let aux_bytes32: Option<[u8; 32]> = match args.aux_bytes32.as_ref() {
        Some(hex_str) => {
            let hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            let bytes = hex::decode(hex).context("invalid hex for --aux-bytes32")?;
            ensure!(bytes.len() == 32, "--aux-bytes32 must be exactly 32 bytes, got {}", bytes.len());
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Some(arr)
        }
        None => None,
    };

    // Build Boundless client once.
    let client = Client::builder()
        .with_rpc_url(args.oracle_rpc_url.clone())
        .with_deployment(args.deployment.clone())
        .with_uploader_config(&args.storage_config)
        .await?
        .with_private_key(args.private_key.clone())
        .config_offer_layer(|cfg| cfg.lock_timeout(args.lock_timeout).ramp_up_period(args.lock_timeout.min(60)))
        .build()
        .await
        .context("failed to build Boundless client")?;

    let check_interval = Duration::from_secs(args.check_interval);

    // Load persistent state.
    let mut state = KeeperState::load(&args.state_file);
    println!("[keeper] Loaded state: round={}, failures={}, last_request={:?}",
        state.last_round_id, state.consecutive_failures, state.last_request_id);

    // Track in-flight request: (request_id_hex, submission_timestamp)
    let mut in_flight: Option<(String, u64)> = None;

    // Restore in-flight from state if we crashed mid-flight.
    if let (Some(req_id), Some(req_time)) = (&state.last_request_id, state.last_request_time) {
        let elapsed = unix_now().saturating_sub(req_time);
        let expiry = (args.lock_timeout as u64) * 2;
        if elapsed < expiry {
            println!("[keeper] Resuming in-flight tracking for request {req_id} (submitted {elapsed}s ago)");
            in_flight = Some((req_id.clone(), req_time));
        }
    }

    loop {
        let now = unix_now();

        // ─── Check for in-flight request completion ──────────────────
        if let Some((ref req_id, submitted_at)) = in_flight {
            match fetch_on_chain_state(&args.oracle_rpc_url, args.oracle_address).await {
                Ok((price, _updated_at, current_round)) => {
                    if current_round > state.last_round_id {
                        // Callback landed -- round advanced.
                        println!("[keeper] HEALTH status=confirmed round={current_round} price={price}");
                        state.last_round_id = current_round;
                        state.last_success_time = Some(now);
                        state.consecutive_failures = 0;
                        state.save(&args.state_file);
                        in_flight = None;
                    } else {
                        let elapsed = now.saturating_sub(submitted_at);
                        let expiry = (args.lock_timeout as u64) * 2;
                        if elapsed > expiry {
                            eprintln!("[keeper] In-flight request {req_id} expired after {elapsed}s (timeout: {expiry}s)");
                            in_flight = None;
                            // Do not increment failures here -- the proof may still land later.
                        } else {
                            println!("[keeper] In-flight request {req_id} pending ({elapsed}s / {expiry}s)");
                            tokio::time::sleep(check_interval).await;
                            continue;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[keeper] Failed to check in-flight status: {e:#}");
                    tokio::time::sleep(check_interval).await;
                    continue;
                }
            }
        }

        // ─── Check triggers ──────────────────────────────────────────
        let (on_chain_price, updated_at, current_round) = match fetch_on_chain_state(&args.oracle_rpc_url, args.oracle_address).await {
            Ok(v) => v,
            Err(e) => {
                let err_msg = format!("{e:#}");
                state.consecutive_failures += 1;
                println!("[keeper] HEALTH status=error failures={} last_error={}", state.consecutive_failures, err_msg);
                state.save(&args.state_file);
                let backoff = backoff_duration(args.check_interval, state.consecutive_failures);
                tokio::time::sleep(backoff).await;
                continue;
            }
        };

        // Update last_round_id if it advanced (e.g. another keeper submitted).
        if current_round > state.last_round_id {
            state.last_round_id = current_round;
            state.last_success_time = Some(now);
            state.consecutive_failures = 0;
            state.save(&args.state_file);
        }

        let heartbeat_triggered = if updated_at == 0 {
            true // No data yet.
        } else {
            now.saturating_sub(updated_at) >= args.heartbeat
        };

        let (deviation_triggered, spot_price) = if on_chain_price.is_zero() {
            (false, None) // Can't compute deviation without a baseline.
        } else {
            match fetch_spot_price(&args.rpc_urls, args.pool, args.dex_type, args.decimals).await {
                Ok(spot) => {
                    let diff = if spot > on_chain_price {
                        spot - on_chain_price
                    } else {
                        on_chain_price - spot
                    };
                    let threshold = on_chain_price * U256::from(args.deviation_bps) / U256::from(10000u64);
                    let triggered = diff > threshold;
                    if triggered {
                        println!("[keeper] Deviation trigger: spot={spot}, on-chain={on_chain_price}, diff={diff}, threshold={threshold}");
                    }
                    (triggered, Some(spot))
                }
                Err(e) => {
                    eprintln!("[keeper] Failed to fetch spot price: {e:#}");
                    (false, None)
                }
            }
        };

        if heartbeat_triggered {
            let age = now.saturating_sub(updated_at);
            println!("[keeper] Heartbeat trigger: last update {age}s ago (threshold: {}s)", args.heartbeat);
        }

        if !heartbeat_triggered && !deviation_triggered {
            let age = now.saturating_sub(updated_at);
            let price_str = spot_price.map_or_else(|| on_chain_price.to_string(), |s| s.to_string());
            println!("[keeper] HEALTH status=ok round={current_round} price={price_str} age={age}s");
            tokio::time::sleep(check_interval).await;
            continue;
        }

        let trigger = if heartbeat_triggered && deviation_triggered {
            "heartbeat+deviation"
        } else if heartbeat_triggered {
            "heartbeat"
        } else {
            "deviation"
        };

        let price_str = spot_price.map_or_else(|| on_chain_price.to_string(), |s| s.to_string());
        println!("[keeper] HEALTH status=triggered trigger={trigger} round={current_round} price={price_str}");
        println!("[keeper] Trigger: {trigger} -- submitting proof request...");

        // Build Steel inputs and submit proof request.
        if use_callback {
            // Callback path: submit with callback, track in-flight.
            match build_and_submit_callback(&args, &client, &chain_spec, aux_bytes32).await {
                Ok(request_id) => {
                    let request_id_hex = format!("{request_id:x}");
                    println!("[keeper] HEALTH status=submitted request_id={request_id_hex} round={current_round}");
                    println!("[keeper] Proof will be delivered via callback to {:#}", args.oracle_address);

                    state.last_request_id = Some(request_id_hex.clone());
                    state.last_request_time = Some(now);
                    state.consecutive_failures = 0;
                    state.save(&args.state_file);

                    in_flight = Some((request_id_hex, now));
                }
                Err(e) => {
                    handle_submit_error(&mut state, &args, e).await;
                    continue;
                }
            }
        } else {
            // Manual path: submit, wait for proof, call updatePrice() directly.
            match build_and_submit_manual(&args, &client, &chain_spec, aux_bytes32).await {
                Ok(()) => {
                    println!("[keeper] Manual submission succeeded");
                    state.consecutive_failures = 0;
                    state.last_success_time = Some(unix_now());
                    // Re-fetch round to confirm.
                    if let Ok((_, _, round)) = fetch_on_chain_state(&args.oracle_rpc_url, args.oracle_address).await {
                        state.last_round_id = round;
                        println!("[keeper] HEALTH status=confirmed round={round}");
                    }
                    state.save(&args.state_file);
                }
                Err(e) => {
                    handle_submit_error(&mut state, &args, e).await;
                    continue;
                }
            }
        }

        tokio::time::sleep(check_interval).await;
    }
}

/// Compute exponential backoff: min(check_interval * 2^failures, 600) seconds.
fn backoff_duration(check_interval_secs: u64, consecutive_failures: u32) -> Duration {
    let base = check_interval_secs;
    // Use saturating shift to avoid overflow; cap exponent at 30 to stay within u64.
    let exponent = consecutive_failures.min(30);
    let backoff_secs = base.saturating_mul(1u64.wrapping_shl(exponent));
    let capped = backoff_secs.min(600);
    Duration::from_secs(capped)
}

/// Handle submission errors with exponential backoff.
async fn handle_submit_error(state: &mut KeeperState, args: &Args, e: anyhow::Error) {
    let err_msg = format!("{e:#}");
    state.consecutive_failures += 1;
    state.save(&args.state_file);

    if state.consecutive_failures >= args.max_retries {
        eprintln!("[keeper] CRITICAL: {0} consecutive failures (max_retries={1}): {2:#}",
            state.consecutive_failures, args.max_retries, e);
    } else {
        eprintln!("[keeper] Failed to submit request (attempt {}/{}): {e:#}",
            state.consecutive_failures, args.max_retries);
    }

    println!("[keeper] HEALTH status=error failures={} last_error={}", state.consecutive_failures, err_msg);

    let backoff = backoff_duration(args.check_interval, state.consecutive_failures);
    println!("[keeper] Backing off for {}s", backoff.as_secs());
    tokio::time::sleep(backoff).await;
}

/// Build Steel inputs (shared between callback and manual paths).
async fn build_steel_inputs(
    args: &Args,
    chain_spec: &EthChainSpec,
    aux_bytes32: Option<[u8; 32]>,
) -> Result<Vec<u8>> {
    let latest_env = with_rpc_fallback(&args.rpc_urls, |url| {
        let chain_spec = chain_spec.clone();
        async move {
            EthEvmEnv::builder()
                .rpc(url)
                .chain_spec(&chain_spec)
                .build()
                .await
                .context("failed to build latest EVM env")
        }
    }).await?;
    let latest_block = latest_env.header().number();

    let block_numbers: Vec<u64> = (0..args.num_blocks as u64)
        .map(|i| latest_block.saturating_sub(i * args.stride))
        .filter(|&b| b > 0)
        .collect();
    ensure!(!block_numbers.is_empty(), "No valid block numbers (chain too young for configured stride)");

    println!(
        "[keeper] Sampling blocks {} to {} (stride {})",
        block_numbers.last().unwrap(),
        block_numbers[0],
        args.stride
    );

    let config = OracleConfig {
        pool: args.pool,
        decimals: args.decimals,
        dex_type: args.dex_type,
        aux_address: args.aux_address,
        aux_bytes32,
        token0_decimals: args.token0_decimals,
        token1_decimals: args.token1_decimals,
        twap_window: args.twap_window,
        chain_spec: chain_spec.clone(),
    };

    let mut stdin_bytes: Vec<u8> = Vec::new();
    stdin_bytes.extend_from_slice(&risc0_zkvm::serde::to_vec(&config)?.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>());
    stdin_bytes.extend_from_slice(&risc0_zkvm::serde::to_vec(&args.num_blocks)?.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>());

    for &block_num in &block_numbers {
        let mut env = with_rpc_fallback(&args.rpc_urls, |url| {
            let chain_spec = chain_spec.clone();
            async move {
                EthEvmEnv::builder()
                    .rpc(url)
                    .block_number(block_num)
                    .chain_spec(&chain_spec)
                    .build()
                    .await
                    .with_context(|| format!("failed to build env for block {block_num}"))
            }
        }).await?;

        match args.dex_type {
            DexType::UniswapV2 | DexType::SolidlyStable => {
                let call = IUniswapV2Pair::getReservesCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let _r = contract.call_builder(&call).call().await?;
            }
            DexType::UniswapV3 => {
                if args.twap_window > 0 {
                    let call = IUniswapV3Pool::observeCall {
                        secondsAgos: vec![args.twap_window, 0],
                    };
                    let mut contract = Contract::preflight(args.pool, &mut env);
                    let _r = contract.call_builder(&call).call().await?;
                } else {
                    let call = IUniswapV3Pool::slot0Call {};
                    let mut contract = Contract::preflight(args.pool, &mut env);
                    let _r = contract.call_builder(&call).call().await?;
                }
            }
            DexType::AlgebraV3 => {
                let call = IAlgebraPool::globalStateCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let _r = contract.call_builder(&call).call().await?;
            }
            DexType::CurveStableSwap => {
                let call0 = ICurvePool::balancesCall { i: U256::ZERO };
                let call1 = ICurvePool::balancesCall { i: U256::from(1u64) };
                let mut contract = Contract::preflight(args.pool, &mut env);
                let _r0: U256 = contract.call_builder(&call0).call().await?;
                let _r1: U256 = contract.call_builder(&call1).call().await?;
            }
            DexType::BalancerV2 => {
                let vault = args.aux_address.context("--aux-address required for Balancer V2")?;
                let pool_id = aux_bytes32.context("--aux-bytes32 required for Balancer V2")?;
                let vault_call = IBalancerVault::getPoolTokensCall { poolId: pool_id.into() };
                let mut vault_contract = Contract::preflight(vault, &mut env);
                let _vr = vault_contract.call_builder(&vault_call).call().await?;
                let weights_call = IBalancerWeightedPool::getNormalizedWeightsCall {};
                let mut pool_contract = Contract::preflight(args.pool, &mut env);
                let _wr: Vec<U256> = pool_contract.call_builder(&weights_call).call().await?;
            }
            DexType::TraderJoeLB => {
                let id_call = ILBPair::getActiveIdCall {};
                let step_call = ILBPair::getBinStepCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let _id = contract.call_builder(&id_call).call().await?;
                let _step: u16 = contract.call_builder(&step_call).call().await?;
            }
            DexType::DodoPMM => {
                let call = IDODO::getMidPriceCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let _r: U256 = contract.call_builder(&call).call().await?;
            }
        }

        let input = env.into_input().await?;
        let input_bytes = risc0_zkvm::serde::to_vec(&input)?.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>();
        stdin_bytes.extend_from_slice(&input_bytes);
    }

    Ok(stdin_bytes)
}

/// Submit a proof request with callback delivery (fire-and-forget).
async fn build_and_submit_callback(
    args: &Args,
    client: &Client,
    chain_spec: &EthChainSpec,
    aux_bytes32: Option<[u8; 32]>,
) -> Result<U256> {
    let stdin_bytes = build_steel_inputs(args, chain_spec, aux_bytes32).await?;

    let mut request = if let Some(ref program_url) = args.program_url {
        client
            .new_request()
            .with_program_url(program_url.clone())?
            .with_stdin(stdin_bytes)
    } else {
        client
            .new_request()
            .with_program(ZK_ORACLE_GUEST_ELF)
            .with_stdin(stdin_bytes)
    };

    request = request.with_requirements(
        RequirementParams::builder()
            .callback_address(args.oracle_address)
            .callback_gas_limit(args.callback_gas_limit),
    );

    let (request_id, _expires_at) = client.submit(request).await?;
    Ok(request_id)
}

/// Submit a proof request, wait for fulfillment, then call updatePrice() directly.
/// Used when the oracle contract has no Boundless Market configured for callbacks.
async fn build_and_submit_manual(
    args: &Args,
    client: &Client,
    chain_spec: &EthChainSpec,
    aux_bytes32: Option<[u8; 32]>,
) -> Result<()> {
    let stdin_bytes = build_steel_inputs(args, chain_spec, aux_bytes32).await?;

    // Submit without callback.
    let request = if let Some(ref program_url) = args.program_url {
        client
            .new_request()
            .with_program_url(program_url.clone())?
            .with_stdin(stdin_bytes)
    } else {
        client
            .new_request()
            .with_program(ZK_ORACLE_GUEST_ELF)
            .with_stdin(stdin_bytes)
    };

    let (request_id, expires_at) = client.submit(request).await?;
    println!("[keeper] Request submitted: {request_id:x} (waiting for proof...)");

    // Wait for the proof to be generated.
    let fulfillment = client
        .wait_for_request_fulfillment(
            request_id,
            Duration::from_secs(5),
            expires_at,
        )
        .await
        .context("proof request was not fulfilled")?;

    let fulfillment_data = fulfillment.data()?;
    let journal_bytes: Vec<u8> = fulfillment_data
        .journal()
        .context("fulfillment missing journal")?
        .to_vec();
    let seal = fulfillment.seal.clone();

    println!("[keeper] Proof received, submitting updatePrice() transaction...");

    // Call updatePrice() directly on the oracle contract.
    let provider = ProviderBuilder::new()
        .wallet(EthereumWallet::from(args.private_key.clone()))
        .connect_http(args.oracle_rpc_url.clone());

    let oracle = IZkOracle::new(args.oracle_address, &provider);
    let pending_tx = oracle.updatePrice(journal_bytes.into(), seal)
        .send()
        .await
        .context("failed to send updatePrice tx")?;
    let tx_hash = *pending_tx.tx_hash();
    println!("[keeper] Tx sent: {tx_hash:#}");

    let receipt = pending_tx
        .get_receipt()
        .await
        .with_context(|| format!("updatePrice tx did not confirm: {tx_hash}"))?;
    ensure!(receipt.status(), "updatePrice tx reverted: {tx_hash}");

    println!("[keeper] Price updated on-chain via manual submission (tx: {tx_hash:#})");
    Ok(())
}
