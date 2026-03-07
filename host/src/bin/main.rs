use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use anyhow::{Context, Result, ensure};
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
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::{Digest, ExecutorEnv, Prover, ProverOpts, default_executor, default_prover};
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

    /// Auxiliary address (e.g. Balancer Vault address).
    #[arg(long, env = "AUX_ADDRESS")]
    aux_address: Option<Address>,

    /// Auxiliary bytes32 (e.g. Balancer pool ID, as hex).
    #[arg(long, env = "AUX_BYTES32")]
    aux_bytes32: Option<String>,

    /// Deployed ZkOracle contract address (for on-chain submission).
    #[arg(long, env = "ORACLE_ADDRESS")]
    oracle_address: Option<Address>,

    /// Private key for on-chain submission.
    #[arg(long, env = "ETH_WALLET_PRIVATE_KEY")]
    private_key: Option<PrivateKeySigner>,

    /// Execute-only mode (no proof).
    #[arg(long, default_value = "false")]
    execute_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let proving = !args.execute_only;
    let submitting = args.oracle_address.is_some();

    if submitting {
        ensure!(
            args.private_key.is_some(),
            "--private-key is required when --oracle-address is set"
        );
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

    println!("ZK Price Oracle");
    println!("  Pool:       {:#}", args.pool);
    println!("  DEX:        {dex_name}");
    println!("  Samples:    {}", args.num_blocks);
    println!("  Stride:     {}", args.stride);
    println!("  Decimals:   {}", args.decimals);
    println!("  Mode:       {}", if submitting { "prove + submit" } else if proving { "prove only" } else { "execute only" });
    println!();

    // Build provider.
    let provider = if let Some(ref pk) = args.private_key {
        ProviderBuilder::new()
            .wallet(EthereumWallet::from(pk.clone()))
            .connect_http(args.rpc_url.clone())
    } else {
        ProviderBuilder::new().connect_http(args.rpc_url.clone())
    };

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

    // Parse aux_bytes32 if provided.
    let aux_bytes32: Option<[u8; 32]> = args.aux_bytes32.as_ref().map(|hex_str| {
        let hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(hex).expect("invalid hex for --aux-bytes32");
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        arr
    });

    // Build Steel environments and preflight each block.
    let mut inputs = Vec::with_capacity(args.num_blocks as usize);

    for &block_num in &block_numbers {
        let mut env = EthEvmEnv::builder()
            .provider(provider.clone())
            .block_number(block_num)
            .chain_spec(chain_spec.clone())
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
                let price = if !r0.is_zero() { (r1 * scale) / r0 } else { U256::ZERO };
                println!("  Block {block_num}: reserve0={r0}, reserve1={r1}, price={price}");
            }
            DexType::UniswapV3 => {
                let call = IUniswapV3Pool::slot0Call {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let returns = contract.call_builder(&call).call().await?;
                println!("  Block {block_num}: sqrtPriceX96={}", returns.sqrtPriceX96);
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
                let b0 = contract.call_builder(&call0).call().await?._0;
                let b1 = contract.call_builder(&call1).call().await?._0;
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
                let _weights = pool_contract.call_builder(&weights_call).call().await?;
                println!("  Block {block_num}: balances={:?}", vault_returns.balances);
            }
            DexType::TraderJoeLB => {
                let id_call = ILBPair::getActiveIdCall {};
                let step_call = ILBPair::getBinStepCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let active_id = contract.call_builder(&id_call).call().await?._0;
                let bin_step = contract.call_builder(&step_call).call().await?._0;
                println!("  Block {block_num}: activeId={active_id}, binStep={bin_step}");
            }
            DexType::DodoPMM => {
                let call = IDODO::getMidPriceCall {};
                let mut contract = Contract::preflight(args.pool, &mut env);
                let mid_price = contract.call_builder(&call).call().await?._0;
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
    };

    let num_blocks = args.num_blocks;
    let mut builder = ExecutorEnv::builder();
    builder.write(&config)?;
    builder.write(&num_blocks)?;
    for input in inputs {
        builder.write(&input)?;
    }
    let executor_env = builder.build().context("failed to build executor env")?;

    if args.execute_only {
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
        println!("  Median Price: {} ({}dp)", journal.medianPrice, journal.decimals);
        println!("  Samples:      {}", journal.numSamples);
        println!("  Block Range:  {} - {}", journal.startBlock, journal.endBlock);
        println!("  Segments:     {}", session_info.segments.len());
        println!(
            "  Total Cycles: {}",
            session_info.segments.iter().map(|s| 1 << s.po2).sum::<u64>()
        );
    } else {
        println!("\nGenerating ZK proof (Groth16)...");
        let prove_info = tokio::task::spawn_blocking(move || {
            default_prover().prove_with_opts(
                executor_env,
                ZK_ORACLE_GUEST_ELF,
                &ProverOpts::groth16(),
            )
        })
        .await?
        .context("failed to generate proof")?;

        let receipt = prove_info.receipt;
        let journal_bytes = receipt.journal.bytes.clone();

        let journal =
            OracleJournal::abi_decode(&journal_bytes).context("failed to decode journal")?;

        println!("\nOracle Result:");
        println!("  Pool:         {:#}", journal.pool);
        println!("  DEX:          {dex_name}");
        println!("  Median Price: {} ({}dp)", journal.medianPrice, journal.decimals);
        println!("  Samples:      {}", journal.numSamples);
        println!("  Block Range:  {} - {}", journal.startBlock, journal.endBlock);

        let seal = encode_seal(&receipt).context("failed to encode seal")?;
        println!("  Seal size:    {} bytes", seal.len());

        if let Some(oracle_address) = args.oracle_address {
            println!("\nSubmitting proof on-chain...");

            let contract = IZkOracle::new(oracle_address, &provider);

            let on_chain_image_id = contract.imageId().call().await?;
            ensure!(
                on_chain_image_id.0 == <[u8; 32]>::from(Digest::from(ZK_ORACLE_GUEST_ID)),
                "Image ID mismatch between guest program and on-chain contract. Redeploy the contract with the correct image ID."
            );

            let call_builder = contract.updatePrice(journal_bytes.into(), seal.into());
            let pending_tx = call_builder.send().await?;
            let tx_hash = *pending_tx.tx_hash();
            println!("  Tx sent: {tx_hash:#}");

            let tx_receipt = pending_tx
                .get_receipt()
                .await
                .with_context(|| format!("transaction did not confirm: {tx_hash}"))?;
            ensure!(tx_receipt.status(), "transaction reverted: {tx_hash}");

            let round_id = contract.currentRoundId().call().await?;
            let latest = contract.latestRoundData().call().await?;

            println!("\nPrice updated on-chain!");
            println!("  Round:   {}", round_id.0);
            println!("  Price:   {}", latest.answer);
            println!("  Updated: {}", latest.updatedAt);
            println!("  Tx:      {tx_hash:#}");
        } else {
            println!("\nProof generated successfully. Use --oracle-address to submit on-chain.");
        }
    }

    Ok(())
}
