use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use risc0_steel::{
    Contract, EvmBlockHeader,
    ethereum::{EthChainSpec, EthEvmEnv},
};
use risc0_zkvm::{ExecutorEnv, default_executor};
use tracing_subscriber::EnvFilter;
use url::Url;
use zk_oracle_methods::{ZK_ORACLE_GUEST_ELF, ZK_ORACLE_GUEST_ID};

// ─── DEX interfaces (for preflight) ──────────────────────────────────────────

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

/// Supported DEX types.
#[derive(Clone, Copy, Debug, ValueEnum, serde::Serialize)]
#[repr(u8)]
enum DexType {
    /// Uniswap V2, SushiSwap, PancakeSwap V2, and all V2 forks
    UniswapV2 = 0,
    /// Uniswap V3, PancakeSwap V3, SushiSwap V3, Aerodrome CL, Velodrome CL
    UniswapV3 = 1,
    /// Curve StableSwap and CryptoSwap pools
    CurveStableSwap = 2,
    /// Balancer V2 weighted pools
    BalancerV2 = 3,
    /// Aerodrome / Velodrome stable pairs (x^3*y + y^3*x = k)
    SolidlyStable = 4,
    /// Trader Joe Liquidity Book V2.1
    TraderJoeLB = 5,
    /// DODO Proactive Market Maker
    DodoPMM = 6,
    /// Algebra V3 (Camelot, QuickSwap V3)
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

/// ZK Price Oracle -- proves DEX prices across multiple blocks using Steel.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Ethereum RPC URL (works with any EVM chain).
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,

    /// DEX pool / pair address.
    #[arg(short, long, env = "POOL_ADDRESS")]
    pool: Address,

    /// DEX type.
    #[arg(long, default_value = "uniswap-v2", env = "DEX_TYPE")]
    dex_type: DexType,

    /// Number of blocks to sample.
    #[arg(short, long, default_value = "10", env = "NUM_BLOCKS")]
    num_blocks: u32,

    /// Sample every Nth block (wider stride = wider sampling window).
    #[arg(short = 's', long, default_value = "10", env = "BLOCK_STRIDE")]
    stride: u64,

    /// Number of blocks to lag behind latest (use finalized blocks).
    #[arg(long, default_value = "64", env = "ANCHOR_LAG")]
    anchor_lag: u64,

    /// Output price decimals.
    #[arg(short, long, default_value = "18", env = "PRICE_DECIMALS")]
    decimals: u8,

    /// Token0 decimals (required for correct price normalization).
    #[arg(long, env = "TOKEN0_DECIMALS")]
    token0_decimals: Option<u8>,

    /// Token1 decimals (required for correct price normalization).
    #[arg(long, env = "TOKEN1_DECIMALS")]
    token1_decimals: Option<u8>,

    /// Auxiliary address (e.g. Balancer Vault address).
    #[arg(long, env = "AUX_ADDRESS")]
    aux_address: Option<Address>,

    /// Auxiliary bytes32 (e.g. Balancer pool ID, as hex).
    #[arg(long, env = "AUX_BYTES32")]
    aux_bytes32: Option<String>,

    /// TWAP window in seconds for V3 observe() (0 = use spot price).
    #[arg(long, default_value = "1800", env = "TWAP_WINDOW")]
    twap_window: u32,

    /// Chain ID for the source chain (auto-detected if not specified).
    #[arg(long, env = "SOURCE_CHAIN_ID")]
    chain_id: Option<u64>,
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

    println!("ZK Price Oracle");
    println!("  Pool:       {:#}", args.pool);
    println!("  DEX:        {dex_name}");
    println!("  Samples:    {}", args.num_blocks);
    println!("  Stride:     {}", args.stride);
    println!("  Anchor lag: {} blocks", args.anchor_lag);
    println!("  Decimals:   {}", args.decimals);
    if args.twap_window > 0 {
        println!("  TWAP:       {}s window", args.twap_window);
    }
    println!();

    // Build chain spec.
    use risc0_steel::revm::primitives::hardfork::SpecId;
    use risc0_steel::config::ChainSpec;
    let chain_spec: EthChainSpec = if let Some(cid) = args.chain_id {
        match cid {
            1 => risc0_steel::ethereum::ETH_MAINNET_CHAIN_SPEC.clone(),
            11155111 => risc0_steel::ethereum::ETH_SEPOLIA_CHAIN_SPEC.clone(),
            _ => ChainSpec::new_single(cid, SpecId::CANCUN),
        }
    } else {
        ChainSpec::new_single(8453, SpecId::CANCUN)
    };

    // Build initial env to get latest block number.
    let latest_env = EthEvmEnv::builder()
        .rpc(args.rpc_url.clone())
        .chain_spec(&chain_spec)
        .build()
        .await?;
    let latest_block = latest_env.header().number();
    println!("Latest block: {latest_block}");

    // Anchor to finalized blocks (lag behind latest).
    let anchor_block = latest_block.saturating_sub(args.anchor_lag);
    let block_numbers: Vec<u64> = (0..args.num_blocks as u64)
        .map(|i| anchor_block.saturating_sub(i * args.stride))
        .collect();

    println!(
        "Sampling blocks {} to {} (stride {}, anchor lag {})",
        block_numbers.last().unwrap(),
        block_numbers[0],
        args.stride,
        args.anchor_lag,
    );

    // Parse aux_bytes32 if provided.
    let aux_bytes32: Option<[u8; 32]> = match args.aux_bytes32.as_ref() {
        Some(hex_str) => {
            let hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            let bytes = hex::decode(hex).context("invalid hex for --aux-bytes32")?;
            anyhow::ensure!(bytes.len() == 32, "--aux-bytes32 must be exactly 32 bytes, got {}", bytes.len());
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Some(arr)
        }
        None => None,
    };

    // Build Steel environments and preflight each block.
    let mut inputs = Vec::with_capacity(args.num_blocks as usize);

    for &block_num in &block_numbers {
        let mut env = EthEvmEnv::builder()
            .rpc(args.rpc_url.clone())
            .block_number(block_num)
            .chain_spec(&chain_spec)
            .build()
            .await
            .with_context(|| format!("failed to build env for block {block_num}"))?;

        // Preflight the appropriate DEX calls to prepare storage proofs.
        match args.dex_type {
            DexType::UniswapV2 | DexType::SolidlyStable => {
                let call = IUniswapV2Pair::getReservesCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let returns = contract.call_builder(&call).call().await?;
                let r0 = U256::from(returns.reserve0);
                let r1 = U256::from(returns.reserve1);
                let scale = U256::from(10u64).pow(U256::from(args.decimals));
                let t0s = U256::from(10u64).pow(U256::from(args.token0_decimals.unwrap_or(18)));
                let t1s = U256::from(10u64).pow(U256::from(args.token1_decimals.unwrap_or(18)));
                let price = if !r0.is_zero() { (r1 * scale * t0s) / (r0 * t1s) } else { U256::ZERO };
                println!("  Block {block_num}: reserve0={r0}, reserve1={r1}, price={price}");
            }
            DexType::UniswapV3 => {
                if args.twap_window > 0 {
                    let call = IUniswapV3Pool::observeCall {
                        secondsAgos: vec![args.twap_window, 0],
                    };
                    let mut contract = Contract::preflight(args.pool, &mut env);
                    let returns = contract.call_builder(&call).call().await?;
                    println!("  Block {block_num}: TWAP observe ({}s window), tickCum[0]={}, tickCum[1]={}",
                        args.twap_window, returns.tickCumulatives[0], returns.tickCumulatives[1]);
                } else {
                    let call = IUniswapV3Pool::slot0Call {};
                    let mut contract = Contract::preflight(args.pool, &mut env);
                    let returns = contract.call_builder(&call).call().await?;
                    println!("  Block {block_num}: sqrtPriceX96={}", returns.sqrtPriceX96);
                }
            }
            DexType::AlgebraV3 => {
                let call = IAlgebraPool::globalStateCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let returns = contract.call_builder(&call).call().await?;
                println!("  Block {block_num}: sqrtPrice={}", returns.price);
            }
            DexType::CurveStableSwap => {
                let call0 = ICurvePool::balancesCall { i: U256::ZERO };
                let call1 = ICurvePool::balancesCall { i: U256::from(1u64) };
                let mut contract = Contract::preflight(args.pool, &mut env);
                let b0: U256 = contract.call_builder(&call0).call().await?;
                let b1: U256 = contract.call_builder(&call1).call().await?;
                println!("  Block {block_num}: balance0={b0}, balance1={b1}");
            }
            DexType::BalancerV2 => {
                let vault = args.aux_address.context("--aux-address (Balancer Vault) is required for Balancer V2")?;
                let pool_id = aux_bytes32.context("--aux-bytes32 (Balancer pool ID) is required for Balancer V2")?;
                let vault_call = IBalancerVault::getPoolTokensCall { poolId: pool_id.into() };
                let mut vault_contract = Contract::preflight(vault, &mut env);
                let vault_returns = vault_contract.call_builder(&vault_call).call().await?;
                let weights_call = IBalancerWeightedPool::getNormalizedWeightsCall {};
                let mut pool_contract = Contract::preflight(args.pool, &mut env);
                let _weights: Vec<U256> = pool_contract.call_builder(&weights_call).call().await?;
                println!("  Block {block_num}: balances={:?}", vault_returns.balances);
            }
            DexType::TraderJoeLB => {
                let id_call = ILBPair::getActiveIdCall {};
                let step_call = ILBPair::getBinStepCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let active_id = contract.call_builder(&id_call).call().await?;
                let bin_step: u16 = contract.call_builder(&step_call).call().await?;
                println!("  Block {block_num}: activeId={active_id}, binStep={bin_step}");
            }
            DexType::DodoPMM => {
                let call = IDODO::getMidPriceCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let mid_price: U256 = contract.call_builder(&call).call().await?;
                println!("  Block {block_num}: midPrice={mid_price}");
            }
        }

        let input = env.into_input().await?;
        inputs.push(input);
    }

    // Build the guest executor environment.
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

    let num_blocks = args.num_blocks;
    let mut builder = ExecutorEnv::builder();
    builder.write(&config)?;
    builder.write(&num_blocks)?;
    for input in inputs {
        builder.write(&input)?;
    }
    let executor_env = builder.build().context("failed to build executor env")?;

    println!("\nRunning the guest program (execute only, no proof)...");
    let exec = default_executor();
    let session_info = exec
        .execute(executor_env, ZK_ORACLE_GUEST_ELF)
        .context("failed to run executor")?;

    let journal =
        OracleJournal::abi_decode(session_info.journal.as_ref()).context("failed to decode journal")?;

    println!("\nOracle Result:");
    println!("  Pool:         {:#}", journal.pool);
    println!("  DEX:          {dex_name}");
    println!("  Median Price: {} ({}dp)", journal.medianPrice, journal.journalDecimals);
    println!("  Samples:      {} valid", journal.numSamples);
    println!("  Block Range:  {} - {}", journal.startBlock, journal.endBlock);
    println!("  Timestamp:    {}", journal.timestamp);
    println!("  Segments:     {}", session_info.segments.len());
    println!(
        "  Total Cycles: {}",
        session_info.segments.iter().map(|s| 1 << s.po2).sum::<u64>()
    );
    println!("  Image ID:     0x{}", hex::encode(<[u8; 32]>::from(risc0_zkvm::Digest::from(ZK_ORACLE_GUEST_ID))));

    Ok(())
}
