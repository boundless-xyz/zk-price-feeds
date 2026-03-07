//! Submit a ZK Oracle price update on-chain.
//!
//! Runs the guest in execute-only mode, then submits the journal
//! to the oracle contract with an empty seal (requires mock verifier).

use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use anyhow::{Context, Result};
use clap::Parser;
use risc0_steel::{
    Contract, EvmBlockHeader,
    alloy::{
        network::EthereumWallet,
        providers::ProviderBuilder,
        signers::local::PrivateKeySigner,
    },
    ethereum::{EthChainSpec, EthEvmEnv},
};
use risc0_zkvm::{ExecutorEnv, default_executor};
use tracing_subscriber::EnvFilter;
use url::Url;
use zk_oracle_methods::ZK_ORACLE_GUEST_ELF;

sol! {
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
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
        function latestRoundData() external view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);
    }
}

#[derive(serde::Serialize)]
#[repr(u8)]
enum DexType { UniswapV2 = 0 }

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

#[derive(Parser, Debug)]
struct Args {
    /// RPC URL for the source chain (e.g. Base mainnet).
    #[arg(long, env = "BASE_MAINNET_RPC")]
    rpc_url: Url,

    /// RPC URL for the oracle chain (e.g. Base Sepolia).
    #[arg(long, env = "RPC_URL")]
    oracle_rpc_url: Url,

    /// Private key for on-chain submission.
    #[arg(long, env = "PRIVATE_KEY")]
    private_key: PrivateKeySigner,

    /// DEX pool address.
    #[arg(long, env = "POOL_ADDRESS")]
    pool: Address,

    /// Oracle contract address.
    #[arg(long, env = "ORACLE_ADDRESS")]
    oracle_address: Address,

    /// Number of blocks to sample.
    #[arg(long, default_value = "10", env = "NUM_BLOCKS")]
    num_blocks: u32,

    /// Sample every Nth block (wider stride = wider sampling window).
    #[arg(long, default_value = "10", env = "BLOCK_STRIDE")]
    stride: u64,

    /// Number of blocks to lag behind latest (use finalized blocks).
    #[arg(long, default_value = "64", env = "ANCHOR_LAG")]
    anchor_lag: u64,

    /// Price decimals.
    #[arg(long, default_value = "18", env = "PRICE_DECIMALS")]
    decimals: u8,

    /// Token0 decimals.
    #[arg(long, env = "TOKEN0_DECIMALS")]
    token0_decimals: Option<u8>,

    /// Token1 decimals.
    #[arg(long, env = "TOKEN1_DECIMALS")]
    token1_decimals: Option<u8>,

    /// Source chain ID.
    #[arg(long, default_value = "8453", env = "SOURCE_CHAIN_ID")]
    source_chain_id: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    // Build chain spec
    use risc0_steel::revm::primitives::hardfork::SpecId;
    use risc0_steel::config::ChainSpec;
    let chain_spec: EthChainSpec = match args.source_chain_id {
        1 => risc0_steel::ethereum::ETH_MAINNET_CHAIN_SPEC.clone(),
        11155111 => risc0_steel::ethereum::ETH_SEPOLIA_CHAIN_SPEC.clone(),
        cid => ChainSpec::new_single(cid, SpecId::CANCUN),
    };

    println!("ZK Price Oracle — Execute + Submit");
    println!("  Pool:       {:#}", args.pool);
    println!("  Oracle:     {:#}", args.oracle_address);
    println!("  Stride:     {}", args.stride);
    println!("  Anchor lag: {} blocks", args.anchor_lag);
    println!();

    // Build Steel inputs
    let latest_env = EthEvmEnv::builder()
        .rpc(args.rpc_url.clone())
        .chain_spec(&chain_spec)
        .build()
        .await?;
    let latest_block = latest_env.header().number();
    println!("Latest block: {latest_block}");

    // Anchor to finalized blocks
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

    let mut inputs = Vec::new();
    for &block_num in &block_numbers {
        let mut env = EthEvmEnv::builder()
            .rpc(args.rpc_url.clone())
            .block_number(block_num)
            .chain_spec(&chain_spec)
            .build()
            .await?;

        let call = IUniswapV2Pair::getReservesCall {};
        let mut contract = Contract::preflight(args.pool, &mut env);
        let returns = contract.call_builder(&call).call().await?;
        let r0 = U256::from(returns.reserve0);
        let r1 = U256::from(returns.reserve1);
        let scale = U256::from(10u64).pow(U256::from(args.decimals));
        let t0s = U256::from(10u64).pow(U256::from(args.token0_decimals.unwrap_or(18)));
        let t1s = U256::from(10u64).pow(U256::from(args.token1_decimals.unwrap_or(18)));
        let price = if !r0.is_zero() { (r1 * scale * t0s) / (r0 * t1s) } else { U256::ZERO };
        println!("  Block {block_num}: price={price}");

        inputs.push(env.into_input().await?);
    }

    // Execute guest
    let config = OracleConfig {
        pool: args.pool,
        decimals: args.decimals,
        dex_type: DexType::UniswapV2,
        aux_address: None,
        aux_bytes32: None,
        token0_decimals: args.token0_decimals,
        token1_decimals: args.token1_decimals,
        twap_window: 0,
        chain_spec: chain_spec.clone(),
    };

    let num_blocks = args.num_blocks;
    let mut builder = ExecutorEnv::builder();
    builder.write(&config)?;
    builder.write(&num_blocks)?;
    for input in inputs {
        builder.write(&input)?;
    }
    let executor_env = builder.build()?;

    println!("\nExecuting guest program...");
    let session = default_executor()
        .execute(executor_env, ZK_ORACLE_GUEST_ELF)
        .context("guest execution failed")?;

    let journal_bytes = session.journal.bytes.clone();
    let journal = OracleJournal::abi_decode(&journal_bytes)
        .context("failed to decode journal")?;

    println!("\nResult:");
    println!("  Median Price: {} ({}dp)", journal.medianPrice, journal.journalDecimals);
    println!("  Samples:      {} valid", journal.numSamples);
    println!("  Block Range:  {} - {}", journal.startBlock, journal.endBlock);
    println!("  Timestamp:    {}", journal.timestamp);

    // Submit on-chain with empty seal (mock verifier)
    println!("\nSubmitting to oracle contract...");
    let provider = ProviderBuilder::new()
        .wallet(EthereumWallet::from(args.private_key.clone()))
        .connect_http(args.oracle_rpc_url.clone());

    let oracle = IZkOracle::new(args.oracle_address, &provider);
    let empty_seal: Vec<u8> = vec![];
    let tx = oracle.updatePrice(journal_bytes.into(), empty_seal.into());
    let pending = tx.send().await.context("failed to send tx")?;
    let tx_hash = *pending.tx_hash();
    println!("  Tx: {tx_hash:#}");

    let receipt = pending.get_receipt().await?;
    if !receipt.status() {
        anyhow::bail!("Transaction reverted: {tx_hash}");
    }

    let latest = oracle.latestRoundData().call().await?;
    println!("\nPrice updated on-chain!");
    println!("  Round:   {}", latest.roundId);
    println!("  Price:   {}", latest.answer);
    println!("  Updated: {}", latest.updatedAt);

    Ok(())
}
