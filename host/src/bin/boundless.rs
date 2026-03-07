//! ZK Price Oracle -- Boundless integration.
//!
//! Submits a proof request to the Boundless Market to prove DEX prices,
//! waits for fulfillment, then submits the proof on-chain to update the oracle.

use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use anyhow::{Context, Result, ensure};
use boundless_market::{Client, Deployment, StorageUploaderConfig, request_builder::RequirementParams};
use clap::{Parser, ValueEnum};
use risc0_steel::{
    Contract, EvmBlockHeader,
    alloy::{
        network::EthereumWallet,
        providers::ProviderBuilder,
        signers::local::PrivateKeySigner,
    },
    ethereum::{EthChainSpec, EthEvmEnv},
    config::ChainSpec,
};
use std::time::Duration;
use tracing_subscriber::EnvFilter;
use url::Url;
use zk_oracle_methods::ZK_ORACLE_GUEST_ELF;

// ─── DEX interfaces ──────────────────────────────────────────────────────────

sol! {
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }

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

    interface IDODO {
        function getMidPrice() external view returns (uint256);
    }
}

sol! {
    struct OracleJournal {
        bytes steelCommitment;
        address pool;
        uint256 medianPrice;
        uint8 decimals;
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

/// ZK Price Oracle -- request a proof from Boundless and submit it on-chain.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Ethereum RPC URL for reading pool state (e.g. Base mainnet).
    #[arg(short, long, env = "BASE_MAINNET_RPC")]
    rpc_url: Url,

    /// RPC URL for the chain where the oracle contract is deployed (e.g. Base Sepolia).
    #[arg(long, env = "RPC_URL")]
    oracle_rpc_url: Url,

    /// Private key for Boundless market interactions and on-chain submission.
    #[arg(long, env = "PRIVATE_KEY")]
    private_key: PrivateKeySigner,

    /// DEX pool / pair address.
    #[arg(short, long, env = "POOL_ADDRESS")]
    pool: Address,

    /// DEX type.
    #[arg(long, default_value = "uniswap-v2", env = "DEX_TYPE")]
    dex_type: DexType,

    /// Deployed ZkOracle contract address.
    #[arg(long, env = "ORACLE_ADDRESS")]
    oracle_address: Address,

    /// Number of blocks to sample.
    #[arg(short, long, default_value = "10", env = "NUM_BLOCKS")]
    num_blocks: u32,

    /// Sample every Nth block (wider stride = wider sampling window).
    #[arg(short = 's', long, default_value = "10", env = "BLOCK_STRIDE")]
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

    /// Lock timeout in seconds (time a prover has to fulfill after locking).
    #[arg(long, default_value = "120", env = "LOCK_TIMEOUT")]
    lock_timeout: u32,

    /// Callback address for Boundless Market to call on proof fulfillment.
    /// When set, the proof is delivered via callback and no on-chain submission is needed.
    #[arg(long, env = "CALLBACK_ADDRESS")]
    callback_address: Option<Address>,

    /// Gas limit for the callback (default 500000).
    #[arg(long, default_value = "500000", env = "CALLBACK_GAS_LIMIT")]
    callback_gas_limit: u64,

    /// Pre-uploaded program URL (skips uploading the ELF).
    #[arg(long, env = "PROGRAM_URL")]
    program_url: Option<Url>,

    /// Storage provider config for uploading the ELF to Boundless.
    #[clap(flatten, next_help_heading = "Storage Uploader")]
    storage_config: StorageUploaderConfig,

    /// Boundless Market deployment config.
    #[clap(flatten, next_help_heading = "Boundless Market Deployment")]
    deployment: Option<Deployment>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

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

    println!("ZK Price Oracle (Boundless)");
    println!("  Pool:       {:#}", args.pool);
    println!("  DEX:        {dex_name}");
    println!("  Oracle:     {:#}", args.oracle_address);
    println!("  Samples:    {}", args.num_blocks);
    println!("  Stride:     {}", args.stride);
    println!("  Decimals:   {}", args.decimals);
    println!();

    // ─── 1. Build chain spec for source chain ─────────────────────────

    use risc0_steel::revm::primitives::hardfork::SpecId;
    let chain_spec: EthChainSpec = match args.source_chain_id {
        1 => risc0_steel::ethereum::ETH_MAINNET_CHAIN_SPEC.clone(),
        11155111 => risc0_steel::ethereum::ETH_SEPOLIA_CHAIN_SPEC.clone(),
        cid => ChainSpec::new_single(cid, SpecId::CANCUN),
    };

    // ─── 2. Build Boundless client ─────────────────────────────────────

    let client = Client::builder()
        .with_rpc_url(args.oracle_rpc_url.clone())
        .with_deployment(args.deployment)
        .with_uploader_config(&args.storage_config)
        .await?
        .with_private_key(args.private_key.clone())
        .config_offer_layer(|cfg| cfg.lock_timeout(args.lock_timeout).ramp_up_period(args.lock_timeout.min(60)))
        .build()
        .await
        .context("failed to build Boundless client")?;

    // ─── 3. Build Steel inputs from source chain ──────────────────────

    let latest_env = EthEvmEnv::builder()
        .rpc(args.rpc_url.clone())
        .chain_spec(&chain_spec)
        .build()
        .await?;
    let latest_block = latest_env.header().number();
    println!("Source chain ID: {}", args.source_chain_id);
    println!("Latest block: {latest_block}");

    let block_numbers: Vec<u64> = (0..args.num_blocks as u64)
        .map(|i| latest_block - (i * args.stride))
        .collect();

    println!(
        "Sampling blocks {} to {} (stride {})",
        block_numbers.last().unwrap(),
        block_numbers[0],
        args.stride
    );

    let aux_bytes32: Option<[u8; 32]> = args.aux_bytes32.as_ref().map(|hex_str| {
        let hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex).expect("invalid hex for --aux-bytes32");
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        arr
    });

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
        let mut env = EthEvmEnv::builder()
            .rpc(args.rpc_url.clone())
            .block_number(block_num)
            .chain_spec(&chain_spec)
            .build()
            .await
            .with_context(|| format!("failed to build env for block {block_num}"))?;

        // Preflight the appropriate DEX calls.
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

        println!("  Block {block_num}: preflight OK");

        let input = env.into_input().await?;
        let input_bytes = risc0_zkvm::serde::to_vec(&input)?.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>();
        stdin_bytes.extend_from_slice(&input_bytes);
    }

    // ─── 4. Submit proof request to Boundless ──────────────────────────

    println!("\nSubmitting proof request to Boundless Market...");

    let mut request = if let Some(program_url) = args.program_url {
        client
            .new_request()
            .with_program_url(program_url)?
            .with_stdin(stdin_bytes)
    } else {
        client
            .new_request()
            .with_program(ZK_ORACLE_GUEST_ELF)
            .with_stdin(stdin_bytes)
    };

    let use_callback = args.callback_address.is_some();
    if let Some(callback_addr) = args.callback_address {
        println!("  Callback:   {callback_addr:#} (gas limit: {})", args.callback_gas_limit);
        request = request.with_requirements(
            RequirementParams::builder()
                .callback_address(callback_addr)
                .callback_gas_limit(args.callback_gas_limit),
        );
    }

    let (request_id, expires_at) = client.submit(request).await?;
    println!("  Request ID: {:x}", request_id);

    // ─── 5. Wait for fulfillment ───────────────────────────────────────

    println!("Waiting for proof fulfillment...");
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

    let journal = OracleJournal::abi_decode(&journal_bytes)
        .context("failed to decode journal")?;

    println!("\nProof received!");
    println!("  Pool:         {:#}", journal.pool);
    println!("  DEX:          {dex_name}");
    println!("  Median Price: {} ({}dp)", journal.medianPrice, journal.decimals);
    println!("  Samples:      {}", journal.numSamples);
    println!("  Block Range:  {} - {}", journal.startBlock, journal.endBlock);

    // ─── 6. Submit proof on-chain (skip if callback handles it) ────────

    if use_callback {
        println!("\nCallback mode: proof delivered via Boundless Market callback.");
        println!("The oracle contract was updated automatically by the market.");
    } else {
        println!("\nSubmitting proof to ZkOracle contract...");

        let provider = ProviderBuilder::new()
            .wallet(EthereumWallet::from(args.private_key.clone()))
            .connect_http(args.oracle_rpc_url.clone());

        let oracle = IZkOracle::new(args.oracle_address, &provider);
        let call_builder = oracle.updatePrice(journal_bytes.into(), seal);
        let pending_tx = call_builder.send().await.context("failed to send tx")?;
        let tx_hash = *pending_tx.tx_hash();
        println!("  Tx sent: {tx_hash:#}");

        let receipt = pending_tx
            .get_receipt()
            .await
            .with_context(|| format!("tx did not confirm: {tx_hash}"))?;
        ensure!(receipt.status(), "tx reverted: {tx_hash}");

        println!("\nPrice updated on-chain!");
        println!("  Tx:      {tx_hash:#}");
    }

    // Read final state regardless of submission path.
    let provider = ProviderBuilder::new()
        .wallet(EthereumWallet::from(args.private_key.clone()))
        .connect_http(args.oracle_rpc_url.clone());
    let oracle = IZkOracle::new(args.oracle_address, &provider);
    let round_id = oracle.currentRoundId().call().await?;
    let latest = oracle.latestRoundData().call().await?;

    println!("  Round:   {}", round_id);
    println!("  Price:   {}", latest.answer);
    println!("  Updated: {}", latest.updatedAt);

    Ok(())
}
