//! ZK Oracle Setup Wizard
//!
//! Interactive CLI that walks you through deploying a new ZK Oracle feed:
//! 1. Configure chain, pool, DEX type
//! 2. Auto-detect token decimals from the pool contract
//! 3. Generate a .env configuration file
//! 4. Optionally deploy the oracle contract via forge
//! 5. Print instructions for running the keeper

use alloy_primitives::Address;
use alloy_sol_types::sol;
use anyhow::{Context, Result};
use risc0_steel::alloy::providers::ProviderBuilder;
use std::io::{self, Write};
use std::path::PathBuf;
use url::Url;

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function decimals() external view returns (uint8);
        function symbol() external view returns (string);
    }

    #[sol(rpc)]
    interface IUniswapV2Pair {
        function token0() external view returns (address);
        function token1() external view returns (address);
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }

    #[sol(rpc)]
    interface IUniswapV3Pool {
        function token0() external view returns (address);
        function token1() external view returns (address);
        function fee() external view returns (uint24);
    }
}

// Known Boundless deployments.
struct BoundlessDeployment {
    name: &'static str,
    chain_id: u64,
    market_address: &'static str,
    verifier_address: &'static str,
    set_verifier_address: &'static str,
    order_stream_url: &'static str,
    default_rpc: &'static str,
}

const DEPLOYMENTS: &[BoundlessDeployment] = &[
    BoundlessDeployment {
        name: "Base Mainnet",
        chain_id: 8453,
        market_address: "0xfd152dadc5183870710fe54f939eae3ab9f0fe82",
        verifier_address: "0xa326b2eb45a5c3c206df905a58970dca57b8719e",
        set_verifier_address: "0x1Ab08498CfF17b9723ED67143A050c8E8c2e3104",
        order_stream_url: "https://base-mainnet.boundless.network",
        default_rpc: "https://mainnet.base.org",
    },
    BoundlessDeployment {
        name: "Ethereum Mainnet",
        chain_id: 1,
        market_address: "0xfd152dadc5183870710fe54f939eae3ab9f0fe82",
        verifier_address: "0xa326b2eb45a5c3c206df905a58970dca57b8719e",
        set_verifier_address: "0x1Ab08498CfF17b9723ED67143A050c8E8c2e3104",
        order_stream_url: "https://eth-mainnet.boundless.network",
        default_rpc: "https://eth.llamarpc.com",
    },
];

fn prompt(msg: &str) -> String {
    print!("{msg}");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn prompt_default(msg: &str, default: &str) -> String {
    print!("{msg} [{default}]: ");
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();
    if trimmed.is_empty() { default.to_string() } else { trimmed.to_string() }
}

fn prompt_select(msg: &str, options: &[&str]) -> usize {
    println!("{msg}");
    for (i, opt) in options.iter().enumerate() {
        println!("  {}: {}", i + 1, opt);
    }
    loop {
        let choice = prompt("> ");
        if let Ok(n) = choice.parse::<usize>() {
            if n >= 1 && n <= options.len() {
                return n - 1;
            }
        }
        println!("Invalid choice. Enter a number 1-{}.", options.len());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!();
    println!("  ╔═══════════════════════════════════════╗");
    println!("  ║       ZK Oracle Setup Wizard          ║");
    println!("  ║    Powered by Boundless + Steel       ║");
    println!("  ╚═══════════════════════════════════════╝");
    println!();

    // ─── Step 1: Chain selection ──────────────────────────────────────

    println!("Step 1: Select chain");
    println!();
    let chain_names: Vec<&str> = DEPLOYMENTS.iter().map(|d| d.name).collect();
    let chain_idx = prompt_select("Which chain?", &chain_names);
    let deployment = &DEPLOYMENTS[chain_idx];
    println!();

    // ─── Step 2: RPC URL ─────────────────────────────────────────────

    println!("Step 2: RPC Configuration");
    println!();
    let rpc_url = prompt_default(
        "RPC URL (paid RPC recommended for reliability)",
        deployment.default_rpc,
    );
    let rpc = Url::parse(&rpc_url).context("invalid RPC URL")?;
    println!();

    // ─── Step 3: Pool configuration ──────────────────────────────────

    println!("Step 3: Pool Configuration");
    println!();

    let dex_types = &[
        "Uniswap V2 (includes Aerodrome, SushiSwap, PancakeSwap forks)",
        "Uniswap V3 (includes forks)",
        "Curve StableSwap",
        "Balancer V2",
        "Solidly Stable",
        "Trader Joe LB",
        "DODO PMM",
        "Algebra V3 (Camelot, QuickSwap V3)",
    ];
    let dex_idx = prompt_select("DEX type:", dex_types);
    let dex_type = match dex_idx {
        0 => "uniswap-v2",
        1 => "uniswap-v3",
        2 => "curve-stable-swap",
        3 => "balancer-v2",
        4 => "solidly-stable",
        5 => "trader-joe-lb",
        6 => "dodo-pmm",
        7 => "algebra-v3",
        _ => unreachable!(),
    };
    println!();

    let pool_str = prompt("Pool/pair contract address: ");
    let pool: Address = pool_str.parse().context("invalid pool address")?;
    println!();

    // ─── Step 4: Auto-detect tokens ──────────────────────────────────

    println!("Step 4: Detecting token info from pool...");
    let provider = ProviderBuilder::new().connect_http(rpc.clone());

    let (token0_addr, token1_addr, token0_symbol, token1_symbol, token0_decimals, token1_decimals) =
        match dex_idx {
            0 | 4 => {
                // V2-style pool
                let pair = IUniswapV2Pair::new(pool, &provider);
                let t0 = pair.token0().call().await.context("failed to read token0")?;
                let t1 = pair.token1().call().await.context("failed to read token1")?;

                let tok0 = IERC20::new(t0, &provider);
                let tok1 = IERC20::new(t1, &provider);

                let d0 = tok0.decimals().call().await.unwrap_or(18);
                let d1 = tok1.decimals().call().await.unwrap_or(18);
                let s0 = tok0.symbol().call().await.unwrap_or_else(|_| "???".to_string());
                let s1 = tok1.symbol().call().await.unwrap_or_else(|_| "???".to_string());

                // Verify pool is live
                let res = pair.getReserves().call().await.context("failed to call getReserves - is this a valid V2 pool?")?;
                println!("  Pool reserves: {} / {}", res.reserve0, res.reserve1);

                (t0, t1, s0, s1, d0, d1)
            }
            1 | 7 => {
                // V3-style pool
                let pool_contract = IUniswapV3Pool::new(pool, &provider);
                let t0 = pool_contract.token0().call().await.context("failed to read token0")?;
                let t1 = pool_contract.token1().call().await.context("failed to read token1")?;

                let tok0 = IERC20::new(t0, &provider);
                let tok1 = IERC20::new(t1, &provider);

                let d0 = tok0.decimals().call().await.unwrap_or(18);
                let d1 = tok1.decimals().call().await.unwrap_or(18);
                let s0 = tok0.symbol().call().await.unwrap_or_else(|_| "???".to_string());
                let s1 = tok1.symbol().call().await.unwrap_or_else(|_| "???".to_string());

                (t0, t1, s0, s1, d0, d1)
            }
            _ => {
                // Other DEX types - ask the user
                println!("  Auto-detection not supported for this DEX type.");
                let t0_str = prompt("  Token0 address: ");
                let t1_str = prompt("  Token1 address: ");
                let t0: Address = t0_str.parse().context("invalid token0 address")?;
                let t1: Address = t1_str.parse().context("invalid token1 address")?;

                let tok0 = IERC20::new(t0, &provider);
                let tok1 = IERC20::new(t1, &provider);

                let d0 = tok0.decimals().call().await.unwrap_or(18);
                let d1 = tok1.decimals().call().await.unwrap_or(18);
                let s0 = tok0.symbol().call().await.unwrap_or_else(|_| "???".to_string());
                let s1 = tok1.symbol().call().await.unwrap_or_else(|_| "???".to_string());

                (t0, t1, s0, s1, d0, d1)
            }
        };

    println!("  Token0: {} ({}) - {} decimals", token0_symbol, token0_addr, token0_decimals);
    println!("  Token1: {} ({}) - {} decimals", token1_symbol, token1_addr, token1_decimals);
    let description = format!("{} / {}", token0_symbol, token1_symbol);
    println!("  Feed:   {description}");
    println!();

    // ─── Step 5: Price decimals ──────────────────────────────────────

    println!("Step 5: Price Configuration");
    let price_decimals = prompt_default("Price decimals", "18");
    let twap_window = if dex_idx == 1 {
        prompt_default("TWAP window (seconds, 0 for spot)", "1800")
    } else {
        "0".to_string()
    };
    println!();

    // ─── Step 6: Sampling config ─────────────────────────────────────

    println!("Step 6: Sampling Configuration");
    let num_blocks = prompt_default("Number of blocks to sample", "5");
    let block_stride = prompt_default("Block stride (sample every Nth block)", "20");
    println!();

    // ─── Step 7: Keeper triggers ─────────────────────────────────────

    println!("Step 7: Keeper Triggers");
    let heartbeat = prompt_default("Heartbeat interval (seconds)", "3600");
    let deviation_bps = prompt_default("Deviation threshold (basis points, 100 = 1%)", "100");
    let check_interval = prompt_default("Check interval (seconds)", "60");
    println!();

    // ─── Step 8: Private key ─────────────────────────────────────────

    println!("Step 8: Wallet");
    println!("  Your private key is needed for deploying and funding the keeper.");
    println!("  It will only be stored in the local .env file.");
    let private_key = prompt("Private key (0x...): ");
    println!();

    // ─── Step 9: Storage (Pinata) ────────────────────────────────────

    println!("Step 9: Storage (for uploading proofs to IPFS)");
    println!("  Get a Pinata JWT at https://app.pinata.cloud/developers/api-keys");
    let pinata_jwt = prompt("Pinata JWT (or press Enter to skip): ");
    println!();

    // ─── Step 10: Aux config for special DEX types ───────────────────

    let mut aux_address = String::new();
    let mut aux_bytes32 = String::new();

    if dex_idx == 3 {
        // Balancer V2
        println!("Step 10: Balancer V2 Configuration");
        aux_address = prompt("Balancer Vault address: ");
        aux_bytes32 = prompt("Balancer Pool ID (bytes32 hex): ");
        println!();
    }

    // ─── Generate IMAGE_ID ───────────────────────────────────────────

    // Use the pre-built image ID from the methods crate.
    let image_id_words = zk_oracle_methods::ZK_ORACLE_GUEST_ID;
    let image_id_bytes: Vec<u8> = image_id_words
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();
    let image_id_hex = format!("0x{}", hex::encode(&image_id_bytes));

    // ─── Generate .env file ──────────────────────────────────────────

    println!("Generating configuration...");
    println!();

    let env_path = prompt_default("Output .env path", ".env.generated");
    let env_path = PathBuf::from(&env_path);

    let mut env_content = format!(
        r#"# ZK Oracle - {description} on {chain_name}
# Generated by zk-oracle wizard

PRIVATE_KEY={private_key}

# Chain
RPC_URL={rpc_url}
BASE_MAINNET_RPC={rpc_url}
CHAIN_ID={chain_id}
SOURCE_CHAIN_ID={chain_id}

# Boundless Market ({chain_name})
BOUNDLESS_MARKET_ADDRESS={market}
SET_VERIFIER_ADDRESS={set_verifier}
ORDER_STREAM_URL={order_stream}

# Verifier
VERIFIER_ADDRESS={verifier}
VERIFIER_ROUTER_ADDRESS={verifier}

# Pool
POOL_ADDRESS={pool:#}
DEX_TYPE={dex_type}
PRICE_DECIMALS={price_decimals}
FEED_DESCRIPTION={description}
TOKEN0_DECIMALS={token0_decimals}
TOKEN1_DECIMALS={token1_decimals}
TWAP_WINDOW={twap_window}

# Sampling
NUM_BLOCKS={num_blocks}
BLOCK_STRIDE={block_stride}

# Keeper
HEARTBEAT_SECONDS={heartbeat}
DEVIATION_BPS={deviation_bps}
CHECK_INTERVAL={check_interval}

# Guest program
IMAGE_ID={image_id_hex}
"#,
        description = description,
        chain_name = deployment.name,
        private_key = private_key,
        rpc_url = rpc_url,
        chain_id = deployment.chain_id,
        market = deployment.market_address,
        set_verifier = deployment.set_verifier_address,
        order_stream = deployment.order_stream_url,
        verifier = deployment.verifier_address,
        pool = pool,
        dex_type = dex_type,
        price_decimals = price_decimals,
        token0_decimals = token0_decimals,
        token1_decimals = token1_decimals,
        twap_window = twap_window,
        num_blocks = num_blocks,
        block_stride = block_stride,
        heartbeat = heartbeat,
        deviation_bps = deviation_bps,
        check_interval = check_interval,
        image_id_hex = image_id_hex,
    );

    if !pinata_jwt.is_empty() {
        env_content.push_str(&format!("\n# Storage (Pinata IPFS)\nPINATA_JWT={pinata_jwt}\n"));
    }

    if !aux_address.is_empty() {
        env_content.push_str(&format!("\nAUX_ADDRESS={aux_address}\n"));
    }
    if !aux_bytes32.is_empty() {
        env_content.push_str(&format!("AUX_BYTES32={aux_bytes32}\n"));
    }

    env_content.push_str("\n# Set after deploy\nORACLE_ADDRESS=\n");

    std::fs::write(&env_path, &env_content).context("failed to write .env file")?;
    println!("  Wrote {}", env_path.display());
    println!();

    // ─── Deploy? ─────────────────────────────────────────────────────

    let deploy = prompt_default("Deploy the oracle contract now? (y/n)", "y");

    if deploy.to_lowercase() == "y" {
        println!();
        println!("Deploying ZkOracle via forge...");

        let contracts_dir = find_contracts_dir()?;

        let status = std::process::Command::new("forge")
            .args(["script", "script/Deploy.s.sol:DeployZkOracle"])
            .args(["--rpc-url", &rpc_url])
            .args(["--private-key", &private_key])
            .args(["--broadcast", "--verify"])
            .env("VERIFIER_ADDRESS", deployment.verifier_address)
            .env("IMAGE_ID", &image_id_hex)
            .env("POOL_ADDRESS", &format!("{pool:#}"))
            .env("PRICE_DECIMALS", &price_decimals)
            .env("FEED_DESCRIPTION", &description)
            .env("VALIDATE_COMMITMENT", "true")
            .env("BOUNDLESS_MARKET_ADDRESS", deployment.market_address)
            .current_dir(&contracts_dir)
            .status()
            .context("failed to run forge. Is foundry installed?")?;

        if status.success() {
            println!();
            println!("  Contract deployed successfully!");
            println!();
            println!("  Copy the deployed address from the forge output above");
            println!("  and set ORACLE_ADDRESS in your .env file:");
            println!();
            println!("    ORACLE_ADDRESS=0x...");
            println!();
        } else {
            eprintln!();
            eprintln!("  Forge deployment failed. You can retry manually:");
            eprintln!();
            eprintln!("    cd contracts");
            eprintln!("    source ../{}", env_path.display());
            eprintln!("    forge script script/Deploy.s.sol:DeployZkOracle \\");
            eprintln!("      --rpc-url $RPC_URL --private-key $PRIVATE_KEY --broadcast");
            eprintln!();
        }
    }

    // ─── Print next steps ────────────────────────────────────────────

    println!("┌─────────────────────────────────────────────────────────┐");
    println!("│  Setup Complete!                                        │");
    println!("└─────────────────────────────────────────────────────────┘");
    println!();
    println!("  Feed: {description}");
    println!("  Chain: {} ({})", deployment.name, deployment.chain_id);
    println!("  Pool: {pool:#}");
    println!("  Image ID: {image_id_hex}");
    println!();
    println!("  Next steps:");
    println!();
    println!("  1. Set ORACLE_ADDRESS in {} (after deploy)", env_path.display());
    println!();
    println!("  2. Run a one-shot proof to verify everything works:");
    println!("     cargo run --release --bin boundless");
    println!();
    println!("  3. Start the keeper for automated updates:");
    println!("     cargo run --release --bin keeper");
    println!();
    println!("  4. (Optional) Run with Docker:");
    println!("     docker build -t zk-oracle .");
    println!("     docker run --env-file {} zk-oracle keeper", env_path.display());
    println!();

    Ok(())
}

/// Find the contracts directory relative to the workspace.
fn find_contracts_dir() -> Result<PathBuf> {
    // Try common locations.
    let candidates = [
        PathBuf::from("contracts"),
        PathBuf::from("../contracts"),
        PathBuf::from("../../contracts"),
    ];

    for candidate in &candidates {
        if candidate.join("foundry.toml").exists() {
            return Ok(candidate.clone());
        }
    }

    // Try to find workspace root via Cargo.
    if let Ok(output) = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version=1"])
        .output()
    {
        if let Ok(meta) = String::from_utf8(output.stdout) {
            if let Some(root) = meta.split("\"workspace_root\":\"").nth(1) {
                if let Some(end) = root.find('"') {
                    let root_path = PathBuf::from(&root[..end]);
                    let contracts = root_path.join("contracts");
                    if contracts.join("foundry.toml").exists() {
                        return Ok(contracts);
                    }
                }
            }
        }
    }

    anyhow::bail!("Could not find contracts/ directory. Run from the zk-oracle workspace root.")
}
