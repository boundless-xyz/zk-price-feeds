#![no_main]

use alloy_primitives::{Address, U256};
use alloy_sol_types::{SolValue, sol};
use risc0_steel::{Contract, EvmBlockHeader, ethereum::EthEvmInput};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

// ─── DEX interfaces ──────────────────────────────────────────────────────────

sol! {
    // Uniswap V2 / SushiSwap / PancakeSwap V2 / Solidly volatile
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }

    // Solidly-style (Aerodrome V2 / Velodrome V2) -- adds stable() flag
    interface ISolidlyPair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function stable() external view returns (bool);
        function decimals() external view returns (uint8);
        function token0() external view returns (address);
        function token1() external view returns (address);
    }

    // Uniswap V3 / PancakeSwap V3 / SushiSwap V3 / Aerodrome CL / Velodrome CL
    interface IUniswapV3Pool {
        function slot0() external view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked);
        function token0() external view returns (address);
        function token1() external view returns (address);
    }

    // Algebra V3 (Camelot, QuickSwap V3) -- uses globalState instead of slot0
    interface IAlgebraPool {
        function globalState() external view returns (uint160 price, int24 tick, uint16 feeZto, uint16 feeOtz, uint16 timepointIndex, uint8 communityFeeToken0, uint8 communityFeeToken1);
        function token0() external view returns (address);
        function token1() external view returns (address);
    }

    // Curve StableSwap
    interface ICurvePool {
        function balances(uint256 i) external view returns (uint256);
        function A() external view returns (uint256);
        function coins(uint256 i) external view returns (address);
    }

    // Balancer V2 Vault
    interface IBalancerVault {
        function getPoolTokens(bytes32 poolId) external view returns (address[] tokens, uint256[] balances, uint256 lastChangeBlock);
    }

    // Balancer weighted pool
    interface IBalancerWeightedPool {
        function getNormalizedWeights() external view returns (uint256[]);
        function getPoolId() external view returns (bytes32);
    }

    // Trader Joe Liquidity Book V2.1
    interface ILBPair {
        function getActiveId() external view returns (uint24);
        function getBinStep() external view returns (uint16);
        function tokenX() external view returns (address);
        function tokenY() external view returns (address);
    }

    // DODO PMM
    interface IDODO {
        function getMidPrice() external view returns (uint256);
    }
}

// ─── Journal ──────────────────────────────────────────────────────────────────

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

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Clone, Copy, PartialEq, Eq)]
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

#[derive(serde::Deserialize)]
struct OracleConfig {
    pool: Address,
    decimals: u8,
    dex_type: DexType,
    /// Extra address needed for some DEX types (e.g. Balancer Vault).
    aux_address: Option<Address>,
    /// Extra bytes32 needed for some DEX types (e.g. Balancer pool ID).
    aux_bytes32: Option<[u8; 32]>,
    /// Token decimals for token0 (needed for V3 price math).
    token0_decimals: Option<u8>,
    /// Token decimals for token1 (needed for V3 price math).
    token1_decimals: Option<u8>,
}

// ─── Price computation per DEX type ───────────────────────────────────────────

fn price_uniswap_v2(pool: Address, scale: U256, env: &risc0_steel::ethereum::EthEvmEnv<risc0_steel::EvmInput<risc0_steel::ethereum::EthBlockHeader>>) -> Option<U256> {
    let call = IUniswapV2Pair::getReservesCall {};
    let contract = Contract::new(pool, env);
    let returns = contract.call_builder(&call).call();
    let r0 = U256::from(returns.reserve0);
    let r1 = U256::from(returns.reserve1);
    if r0.is_zero() { return None; }
    Some((r1 * scale) / r0)
}

fn price_uniswap_v3(pool: Address, scale: U256, t0dec: u8, t1dec: u8, env: &risc0_steel::ethereum::EthEvmEnv<risc0_steel::EvmInput<risc0_steel::ethereum::EthBlockHeader>>) -> Option<U256> {
    let call = IUniswapV3Pool::slot0Call {};
    let contract = Contract::new(pool, env);
    let returns = contract.call_builder(&call).call();
    let sqrt_price = U256::from(returns.sqrtPriceX96);
    if sqrt_price.is_zero() { return None; }
    // price = sqrtPriceX96^2 * 10^decimals * 10^t0dec / (2^192 * 10^t1dec)
    // Rearrange to avoid overflow: (sqrtPriceX96^2 * scale * 10^t0dec) / (2^192 * 10^t1dec)
    let sq = sqrt_price * sqrt_price;
    let q192 = U256::from(1u64) << 192;
    let t0_scale = U256::from(10u64).pow(U256::from(t0dec));
    let t1_scale = U256::from(10u64).pow(U256::from(t1dec));
    // token0 price in token1 terms: (sq * scale * t0_scale) / (q192 * t1_scale)
    // But this can overflow. Use intermediate division:
    // price = (sq / q192) * scale * t0_scale / t1_scale -- but sq/q192 loses precision
    // Better: (sq * scale) / q192 * t0_scale / t1_scale
    let numerator = sq * scale;
    let price_raw = numerator / q192;
    let price = (price_raw * t0_scale) / t1_scale;
    Some(price)
}

fn price_algebra_v3(pool: Address, scale: U256, t0dec: u8, t1dec: u8, env: &risc0_steel::ethereum::EthEvmEnv<risc0_steel::EvmInput<risc0_steel::ethereum::EthBlockHeader>>) -> Option<U256> {
    let call = IAlgebraPool::globalStateCall {};
    let contract = Contract::new(pool, env);
    let returns = contract.call_builder(&call).call();
    let sqrt_price = U256::from(returns.price);
    if sqrt_price.is_zero() { return None; }
    let sq = sqrt_price * sqrt_price;
    let q192 = U256::from(1u64) << 192;
    let t0_scale = U256::from(10u64).pow(U256::from(t0dec));
    let t1_scale = U256::from(10u64).pow(U256::from(t1dec));
    let numerator = sq * scale;
    let price_raw = numerator / q192;
    let price = (price_raw * t0_scale) / t1_scale;
    Some(price)
}

fn price_curve(pool: Address, scale: U256, env: &risc0_steel::ethereum::EthEvmEnv<risc0_steel::EvmInput<risc0_steel::ethereum::EthBlockHeader>>) -> Option<U256> {
    let call0 = ICurvePool::balancesCall { i: U256::ZERO };
    let call1 = ICurvePool::balancesCall { i: U256::from(1u64) };
    let contract = Contract::new(pool, env);
    let b0 = contract.call_builder(&call0).call()._0;
    let b1 = contract.call_builder(&call1).call()._0;
    if b0.is_zero() { return None; }
    // Simple ratio price (works well for balanced Curve pools).
    // For stableswap, b0 ~ b1 so price ~ scale. For crypto pools, this is a raw ratio.
    Some((b1 * scale) / b0)
}

fn price_balancer(pool: Address, vault: Address, pool_id: [u8; 32], scale: U256, env: &risc0_steel::ethereum::EthEvmEnv<risc0_steel::EvmInput<risc0_steel::ethereum::EthBlockHeader>>) -> Option<U256> {
    // Get balances from the Vault.
    let vault_call = IBalancerVault::getPoolTokensCall { poolId: pool_id.into() };
    let vault_contract = Contract::new(vault, env);
    let vault_returns = vault_contract.call_builder(&vault_call).call();
    let balances = vault_returns.balances;
    if balances.len() < 2 || balances[0].is_zero() { return None; }

    // Get normalized weights from the pool.
    let weights_call = IBalancerWeightedPool::getNormalizedWeightsCall {};
    let pool_contract = Contract::new(pool, env);
    let weights = pool_contract.call_builder(&weights_call).call()._0;
    if weights.len() < 2 || weights[0].is_zero() || weights[1].is_zero() { return None; }

    // Balancer price: (balance1 / weight1) / (balance0 / weight0) = (balance1 * weight0) / (balance0 * weight1)
    let price = (balances[1] * weights[0] * scale) / (balances[0] * weights[1]);
    Some(price)
}

fn price_solidly_stable(pool: Address, scale: U256, env: &risc0_steel::ethereum::EthEvmEnv<risc0_steel::EvmInput<risc0_steel::ethereum::EthBlockHeader>>) -> Option<U256> {
    // Solidly stable pools still expose getReserves, but the AMM formula is x^3*y + y^3*x = k.
    // For price discovery, the marginal price at reserves (x,y) is: dy/dx = (3x^2*y + y^3) / (x^3 + 3x*y^2)
    // For simplicity and robustness, use the ratio (same as V2) -- for stable pools this is ~1.0,
    // and the marginal price for stables is very close to the ratio.
    let call = ISolidlyPair::getReservesCall {};
    let contract = Contract::new(pool, env);
    let returns = contract.call_builder(&call).call();
    let r0 = U256::from(returns.reserve0);
    let r1 = U256::from(returns.reserve1);
    if r0.is_zero() { return None; }
    Some((r1 * scale) / r0)
}

fn price_trader_joe_lb(pool: Address, scale: U256, env: &risc0_steel::ethereum::EthEvmEnv<risc0_steel::EvmInput<risc0_steel::ethereum::EthBlockHeader>>) -> Option<U256> {
    let id_call = ILBPair::getActiveIdCall {};
    let step_call = ILBPair::getBinStepCall {};
    let contract = Contract::new(pool, env);
    let active_id = contract.call_builder(&id_call).call()._0;
    let bin_step = contract.call_builder(&step_call).call()._0;

    // LB price = (1 + binStep/10000) ^ (activeId - 2^23)
    // Use fixed-point approximation: price ~ scale * (1 + binStep/10000)^delta
    // For small delta, this is: scale * exp(delta * ln(1 + binStep/10000))
    // Approximate with: scale * (10000 + binStep)^|delta| / 10000^|delta|
    let base_id: i64 = 1 << 23; // 8388608
    let delta: i64 = (active_id as i64) - base_id;
    let bs = U256::from(bin_step as u64);
    let ten_k = U256::from(10_000u64);

    // For reasonable delta values (< 1000), compute iteratively
    let abs_delta = delta.unsigned_abs();
    if abs_delta > 500 { return None; } // safety limit

    let mut result = scale;
    for _ in 0..abs_delta {
        if delta > 0 {
            result = (result * (ten_k + bs)) / ten_k;
        } else {
            result = (result * ten_k) / (ten_k + bs);
        }
    }
    Some(result)
}

fn price_dodo(pool: Address, scale: U256, env: &risc0_steel::ethereum::EthEvmEnv<risc0_steel::EvmInput<risc0_steel::ethereum::EthBlockHeader>>) -> Option<U256> {
    let call = IDODO::getMidPriceCall {};
    let contract = Contract::new(pool, env);
    let mid_price = contract.call_builder(&call).call()._0;
    if mid_price.is_zero() { return None; }
    // DODO getMidPrice returns price with 18 decimals.
    // Scale to requested decimals: (mid_price * scale) / 10^18
    let dodo_scale = U256::from(10u64).pow(U256::from(18u8));
    Some((mid_price * scale) / dodo_scale)
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let config: OracleConfig = env::read();
    let num_samples: u32 = env::read();

    let mut prices: Vec<U256> = Vec::with_capacity(num_samples as usize);
    let mut start_block: u64 = u64::MAX;
    let mut end_block: u64 = 0;
    let mut last_commitment: Option<Vec<u8>> = None;

    let scale = U256::from(10u64).pow(U256::from(config.decimals));
    let t0dec = config.token0_decimals.unwrap_or(18);
    let t1dec = config.token1_decimals.unwrap_or(18);

    for _ in 0..num_samples {
        let input: EthEvmInput = env::read();
        let evm_env = input.into_env(&risc0_steel::ethereum::ETH_MAINNET_CHAIN_SPEC);

        let block_number = evm_env.header().number();
        if block_number < start_block {
            start_block = block_number;
        }
        if block_number > end_block {
            end_block = block_number;
            last_commitment = Some(evm_env.commitment().abi_encode());
        }

        let price = match config.dex_type {
            DexType::UniswapV2 => price_uniswap_v2(config.pool, scale, &evm_env),
            DexType::UniswapV3 => price_uniswap_v3(config.pool, scale, t0dec, t1dec, &evm_env),
            DexType::AlgebraV3 => price_algebra_v3(config.pool, scale, t0dec, t1dec, &evm_env),
            DexType::CurveStableSwap => price_curve(config.pool, scale, &evm_env),
            DexType::BalancerV2 => {
                let vault = config.aux_address.expect("Balancer requires aux_address (Vault)");
                let pool_id = config.aux_bytes32.expect("Balancer requires aux_bytes32 (poolId)");
                price_balancer(config.pool, vault, pool_id, scale, &evm_env)
            }
            DexType::SolidlyStable => price_solidly_stable(config.pool, scale, &evm_env),
            DexType::TraderJoeLB => price_trader_joe_lb(config.pool, scale, &evm_env),
            DexType::DodoPMM => price_dodo(config.pool, scale, &evm_env),
        };

        if let Some(p) = price {
            prices.push(p);
        }
    }

    assert!(!prices.is_empty(), "No valid price samples");

    // Compute median.
    prices.sort();
    let median = if prices.len() % 2 == 0 {
        let mid = prices.len() / 2;
        (prices[mid - 1] + prices[mid]) / U256::from(2)
    } else {
        prices[prices.len() / 2]
    };

    let steel_commitment = last_commitment.expect("No blocks processed");

    let journal = OracleJournal {
        steelCommitment: steel_commitment.into(),
        pool: config.pool,
        medianPrice: median,
        decimals: config.decimals,
        numSamples: num_samples,
        startBlock: start_block,
        endBlock: end_block,
    };

    env::commit_slice(&journal.abi_encode());
}
