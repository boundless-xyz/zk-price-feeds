//! ZK Price Oracle -- Boundless integration.
//!
//! Submits a proof request to the Boundless Market to prove DEX prices,
//! waits for fulfillment, then submits the proof on-chain to update the oracle.
//!
//! Usage:
//!   cargo run --release --bin boundless -- \
//!     --rpc-url $RPC_URL \
//!     --private-key $PRIVATE_KEY \
//!     --pool 0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc \
//!     --oracle-address 0x... \
//!     --dex-type uniswap-v2 \
//!     --num-blocks 10

use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use anyhow::{Context, Result, ensure};
use boundless_market::{Client, Deployment, StorageUploaderConfig};
use clap::{Parser, ValueEnum};
use risc0_steel::{
    Contract, EvmBlockHeader,
    alloy::{
        network::EthereumWallet,
        providers::{Provider, ProviderBuilder},
        signers::local::PrivateKeySigner,
    },
    ethereum::{EthChainSpec, EthEvmEnv},
};
use risc0_zkvm::sha::Digestible;
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
}

/// ZK Price Oracle -- request a proof from Boundless and submit it on-chain.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Ethereum RPC URL (works with any EVM chain).
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,

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

    /// Sample every Nth block.
    #[arg(short = 's', long, default_value = "1", env = "BLOCK_STRIDE")]
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

    // ─── 1. Build Boundless client ─────────────────────────────────────

    let client = Client::builder()
        .with_rpc_url(args.rpc_url.clone())
        .with_deployment(args.deployment)
        .with_uploader_config(&args.storage_config)
        .await?
        .with_private_key(args.private_key.clone())
        .build()
        .await
        .context("failed to build Boundless client")?;

    // ─── 2. Build Steel inputs ─────────────────────────────────────────

    let provider = ProviderBuilder::new()
        .wallet(EthereumWallet::from(args.private_key.clone()))
        .connect_http(args.rpc_url.clone());

    let chain_id = provider.get_chain_id().await?;
    let chain_spec = EthChainSpec::from_chain_id(chain_id)
        .with_context(|| format!("Unsupported chain ID: {chain_id}"))?;

    let latest_env = EthEvmEnv::builder()
        .provider(provider.clone())
        .chain_spec(chain_spec.clone())
        .build()
        .await?;
    let latest_block = latest_env.header().number();
    println!("Chain ID: {chain_id}");
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
    };

    let mut stdin_bytes: Vec<u8> = Vec::new();
    stdin_bytes.extend_from_slice(&risc0_zkvm::serde::to_vec(&config)?.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>());
    stdin_bytes.extend_from_slice(&risc0_zkvm::serde::to_vec(&args.num_blocks)?.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>());

    for &block_num in &block_numbers {
        let mut env = EthEvmEnv::builder()
            .provider(provider.clone())
            .block_number(block_num)
            .chain_spec(chain_spec.clone())
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
                let call = IUniswapV3Pool::slot0Call {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let _r = contract.call_builder(&call).call().await?;
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
                let _r0 = contract.call_builder(&call0).call().await?;
                let _r1 = contract.call_builder(&call1).call().await?;
            }
            DexType::BalancerV2 => {
                let vault = args.aux_address.context("--aux-address required for Balancer V2")?;
                let pool_id = aux_bytes32.context("--aux-bytes32 required for Balancer V2")?;
                let vault_call = IBalancerVault::getPoolTokensCall { poolId: pool_id.into() };
                let mut vault_contract = Contract::preflight(vault, &mut env);
                let _vr = vault_contract.call_builder(&vault_call).call().await?;
                let weights_call = IBalancerWeightedPool::getNormalizedWeightsCall {};
                let mut pool_contract = Contract::preflight(args.pool, &mut env);
                let _wr = pool_contract.call_builder(&weights_call).call().await?;
            }
            DexType::TraderJoeLB => {
                let id_call = ILBPair::getActiveIdCall {};
                let step_call = ILBPair::getBinStepCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let _id = contract.call_builder(&id_call).call().await?;
                let _step = contract.call_builder(&step_call).call().await?;
            }
            DexType::DodoPMM => {
                let call = IDODO::getMidPriceCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let _r = contract.call_builder(&call).call().await?;
            }
        }

        println!("  Block {block_num}: preflight OK");

        let input = env.into_input().await?;
        let input_bytes = risc0_zkvm::serde::to_vec(&input)?.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>();
        stdin_bytes.extend_from_slice(&input_bytes);
    }

    // ─── 3. Submit proof request to Boundless ──────────────────────────

    println!("\nSubmitting proof request to Boundless Market...");

    let request = if let Some(program_url) = args.program_url {
        client
            .new_request()
            .with_program_url(program_url)?
            .with_stdin(&stdin_bytes)
    } else {
        client
            .new_request()
            .with_program(ZK_ORACLE_GUEST_ELF)
            .with_stdin(&stdin_bytes)
    };

    let (request_id, expires_at) = client.submit(request).await?;
    println!("  Request ID: {:x}", request_id);

    // ─── 4. Wait for fulfillment ───────────────────────────────────────

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
    let journal_bytes = fulfillment_data
        .journal()
        .context("fulfillment missing journal")?
        .bytes
        .clone();
    let seal = fulfillment.seal.clone();

    let journal = OracleJournal::abi_decode(&journal_bytes)
        .context("failed to decode journal")?;

    println!("\nProof received!");
    println!("  Pool:         {:#}", journal.pool);
    println!("  DEX:          {dex_name}");
    println!("  Median Price: {} ({}dp)", journal.medianPrice, journal.decimals);
    println!("  Samples:      {}", journal.numSamples);
    println!("  Block Range:  {} - {}", journal.startBlock, journal.endBlock);

    // ─── 5. Submit proof on-chain ──────────────────────────────────────

    println!("\nSubmitting proof to ZkOracle contract...");

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

    let round_id = oracle.currentRoundId().call().await?;
    let latest = oracle.latestRoundData().call().await?;

    println!("\nPrice updated on-chain!");
    println!("  Round:   {}", round_id.0);
    println!("  Price:   {}", latest.answer);
    println!("  Updated: {}", latest.updatedAt);
    println!("  Tx:      {tx_hash:#}");

    Ok(())
}
