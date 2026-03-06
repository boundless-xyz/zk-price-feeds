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
//!     --num-blocks 10

use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use anyhow::{Context, Result, ensure};
use boundless_market::{Client, Deployment, StorageUploaderConfig};
use clap::Parser;
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

sol! {
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
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

/// Guest input: oracle configuration passed to the guest.
#[derive(serde::Serialize)]
struct OracleConfig {
    pool: Address,
    decimals: u8,
}

/// ZK Price Oracle -- request a proof from Boundless and submit it on-chain.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Ethereum RPC URL.
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,

    /// Private key for Boundless market interactions and on-chain submission.
    #[arg(long, env = "PRIVATE_KEY")]
    private_key: PrivateKeySigner,

    /// Uniswap V2 pair address.
    #[arg(short, long, env = "POOL_ADDRESS")]
    pool: Address,

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

    /// Pre-uploaded program URL (skips uploading the ELF).
    #[arg(long, env = "PROGRAM_URL")]
    program_url: Option<Url>,

    /// Storage provider config for uploading the ELF to Boundless.
    #[clap(flatten, next_help_heading = "Storage Uploader")]
    storage_config: StorageUploaderConfig,

    /// Boundless Market deployment config (optional, uses defaults if omitted).
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

    println!("ZK Price Oracle (Boundless)");
    println!("  Pool:       {:#}", args.pool);
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

    // Preflight each block and serialize the Steel inputs.
    let call = IUniswapV2Pair::getReservesCall {};
    let config = OracleConfig {
        pool: args.pool,
        decimals: args.decimals,
    };

    // Build the stdin bytes for the guest.
    let mut stdin_bytes: Vec<u8> = Vec::new();
    // Write config and num_blocks via bincode (matching risc0 env::read format).
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

        let mut contract = Contract::preflight(args.pool, &mut env);
        let returns = contract.call_builder(&call).call().await?;

        let reserve0 = U256::from(returns.reserve0);
        let reserve1 = U256::from(returns.reserve1);
        let scale = U256::from(10u64).pow(U256::from(args.decimals));
        let price = if !reserve0.is_zero() {
            (reserve1 * scale) / reserve0
        } else {
            U256::ZERO
        };
        println!("  Block {block_num}: price={price}");

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

    println!("\n✅ Price updated on-chain!");
    println!("  Round:   {}", round_id.0);
    println!("  Price:   {}", latest.answer);
    println!("  Updated: {}", latest.updatedAt);
    println!("  Tx:      {tx_hash:#}");

    Ok(())
}
