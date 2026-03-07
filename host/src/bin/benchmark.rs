use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use anyhow::{Context, Result};
use risc0_steel::{
    Contract, EvmBlockHeader,
    ethereum::{ETH_MAINNET_CHAIN_SPEC, EthChainSpec, EthEvmEnv},
};
use risc0_zkvm::{ExecutorEnv, default_executor};
use std::time::Instant;
use url::Url;
use zk_oracle_methods::ZK_ORACLE_GUEST_ELF;

sol! {
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }
}

// Journal structure matching the guest.
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

struct BenchmarkResult {
    pair_name: String,
    pool: Address,
    price: U256,
    decimals: u8,
    num_blocks: u32,
    segments: usize,
    total_cycles: u64,
    user_cycles: u64,
    exec_time_ms: u128,
    block_number: u64,
}

async fn benchmark_pair(
    rpc_url: &Url,
    pair_name: &str,
    pool: Address,
    num_blocks: u32,
    decimals: u8,
) -> Result<BenchmarkResult> {
    println!("  Preflighting {pair_name} ({pool:#})...");

    // Build a single-block environment and preflight.
    let mut env = EthEvmEnv::builder()
        .rpc(rpc_url.clone())
        .chain_spec(&ETH_MAINNET_CHAIN_SPEC)
        .build()
        .await?;

    let block_number = env.header().number();

    let call = IUniswapV2Pair::getReservesCall {};
    let mut contract = Contract::preflight(pool, &mut env);
    let returns = contract.call_builder(&call).call().await?;
    let input = env.into_input().await?;

    let reserve0 = U256::from(returns.reserve0);
    let reserve1 = U256::from(returns.reserve1);
    let scale = U256::from(10u64).pow(U256::from(decimals));
    let spot_price = if !reserve0.is_zero() {
        (reserve1 * scale) / reserve0
    } else {
        U256::ZERO
    };

    println!("    reserve0={reserve0}, reserve1={reserve1}");
    println!("    spot price: {spot_price}");

    // Build executor env with num_blocks copies of the same input.
    // This simulates the cycle cost of a 20-block median computation.
    let config = OracleConfig {
        pool,
        decimals,
        dex_type: DexType::UniswapV2,
        aux_address: None,
        aux_bytes32: None,
        token0_decimals: None,
        token1_decimals: None,
        twap_window: 0,
        chain_spec: ETH_MAINNET_CHAIN_SPEC.clone(),
    };
    let mut builder = ExecutorEnv::builder();
    builder.write(&config)?;
    builder.write(&num_blocks)?;
    for _ in 0..num_blocks {
        builder.write(&input)?;
    }
    let executor_env = builder.build().context("failed to build executor env")?;

    println!("    Executing guest ({num_blocks} blocks)...");
    let start = Instant::now();
    let exec = default_executor();
    let session = exec
        .execute(executor_env, ZK_ORACLE_GUEST_ELF)
        .context("failed to run executor")?;
    let exec_time_ms = start.elapsed().as_millis();

    let journal =
        OracleJournal::abi_decode(session.journal.as_ref()).context("failed to decode journal")?;

    let total_cycles: u64 = session.segments.iter().map(|s| 1u64 << s.po2).sum();
    let user_cycles: u64 = session.segments.iter().map(|s| s.cycles as u64).sum();

    Ok(BenchmarkResult {
        pair_name: pair_name.to_string(),
        pool,
        price: journal.medianPrice,
        decimals,
        num_blocks,
        segments: session.segments.len(),
        total_cycles,
        user_cycles,
        exec_time_ms,
        block_number,
    })
}

fn format_price(price: &U256, decimals: u8, token0_decimals: u8, token1_decimals: u8) -> String {
    // Price is (reserve1 * 10^decimals) / reserve0
    // To get human-readable, we need to account for token decimal differences.
    // For USDC (6 decimals) as token0 and WETH (18 decimals) as token1:
    //   raw_price = reserve1_18dp * 10^18 / reserve0_6dp
    //   human_price = raw_price / 10^(18 + 18 - 6) = raw_price / 10^30
    // For USDC as token0 and WBTC (8 decimals) as token1:
    //   human_price = raw_price / 10^(18 + 8 - 6) = raw_price / 10^20
    let adjustment = decimals as i32 + token1_decimals as i32 - token0_decimals as i32;
    let divisor = U256::from(10u64).pow(U256::from(adjustment as u32));
    let whole = price / divisor;
    let frac = (price % divisor) / U256::from(10u64).pow(U256::from((adjustment - 2).max(0) as u32));
    format!("{whole}.{frac:02}")
}

// Estimated proving cost in USD (rough: $0.001 per 1M cycles on Boundless).
fn estimate_cost_usd(total_cycles: u64) -> f64 {
    total_cycles as f64 / 1_000_000.0 * 0.001
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url: Url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://ethereum-rpc.publicnode.com".to_string())
        .parse()?;

    let num_blocks: u32 = std::env::var("NUM_BLOCKS")
        .unwrap_or_else(|_| "20".to_string())
        .parse()?;

    println!("=== ZK Price Oracle Benchmarks ===");
    println!("RPC:     {rpc_url}");
    println!("Blocks:  {num_blocks}");
    println!();

    // Pairs to benchmark.
    // Uniswap V2 token ordering: token0 < token1 (by address).
    // ETH/USDC pool: token0=USDC(6dp), token1=WETH(18dp)
    // WBTC/USDC pool: token0=WBTC(8dp), token1=USDC(6dp)
    let pairs: Vec<(&str, Address, u8, u8, u8)> = vec![
        (
            "ETH/USDC",
            "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".parse()?,
            18,  // price output decimals
            6,   // token0 decimals (USDC)
            18,  // token1 decimals (WETH)
        ),
        (
            "BTC/USDC",
            "0x004375Dff511095CC5A197A54140a24eFEF3A416".parse()?,
            18,
            8,   // token0 decimals (WBTC)
            6,   // token1 decimals (USDC)
        ),
    ];

    let mut results: Vec<BenchmarkResult> = Vec::new();

    for (name, pool, decimals, _t0dec, _t1dec) in &pairs {
        match benchmark_pair(&rpc_url, name, *pool, num_blocks, *decimals).await {
            Ok(r) => results.push(r),
            Err(e) => println!("  ERROR benchmarking {name}: {e:#}"),
        }
        println!();
    }

    // Print summary table.
    println!("=== BENCHMARK RESULTS ({num_blocks}-block median) ===");
    println!();
    println!(
        "{:<12} {:>12} {:>8} {:>14} {:>14} {:>10} {:>10}",
        "Pair", "Block", "Segs", "Total Cycles", "User Cycles", "Exec (ms)", "Est. Cost"
    );
    println!("{}", "-".repeat(82));

    for r in &results {
        println!(
            "{:<12} {:>12} {:>8} {:>14} {:>14} {:>10} {:>10}",
            r.pair_name,
            r.block_number,
            r.segments,
            format_cycles(r.total_cycles),
            format_cycles(r.user_cycles),
            r.exec_time_ms,
            format!("${:.4}", estimate_cost_usd(r.total_cycles)),
        );
    }

    println!();
    println!("Notes:");
    println!("  - All {num_blocks} samples use the latest block (public RPC lacks historical eth_getProof)");
    println!("  - Cycle counts are representative since per-block work is identical");
    println!("  - ZKC/USDC: no Uniswap V2 pair exists (ZKC trades on V4 only)");
    println!("  - Est. Cost based on ~$0.001 / 1M cycles (Boundless marketplace estimate)");
    println!("  - Price is raw uint256 with 18 decimals");

    for (i, r) in results.iter().enumerate() {
        let (_, _, _, t0dec, t1dec) = pairs[i];
        let human = format_price(&r.price, r.decimals, t0dec, t1dec);
        println!("  - {}: median price = {} (human ≈ ${human})", r.pair_name, r.price);
    }

    Ok(())
}

fn format_cycles(cycles: u64) -> String {
    if cycles >= 1_000_000 {
        format!("{:.1}M", cycles as f64 / 1_000_000.0)
    } else if cycles >= 1_000 {
        format!("{:.1}K", cycles as f64 / 1_000.0)
    } else {
        format!("{cycles}")
    }
}
