#![no_main]

use alloy_primitives::{Address, Sign, U256};
use alloy_sol_types::{SolValue, sol};
use risc0_steel::{Contract, EvmBlockHeader, ethereum::EthEvmInput};
use risc0_steel::ethereum::EthChainSpec;
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

// ─── DEX interfaces ──────────────────────────────────────────────────────────

sol! {
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }

    interface ISolidlyPair {
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
        uint64 timestamp;
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
    aux_address: Option<Address>,
    aux_bytes32: Option<[u8; 32]>,
    token0_decimals: Option<u8>,
    token1_decimals: Option<u8>,
    twap_window: u32,
    chain_spec: EthChainSpec,
}

// ─── V3 price helper ─────────────────────────────────────────────────────────

/// Compute V3/Algebra price from sqrtPriceX96 without U256 overflow.
///
/// sqrtPriceX96 is stored as a Q64.96 fixed-point number.
/// price = sqrtPriceX96^2 / 2^192
///
/// Direct squaring overflows U256 for sqrtPriceX96 > 2^128.
/// We split: sqrtPriceX96 = a * 2^96 + b, then:
///   price * scale = a^2*scale + 2*a*b*scale/2^96 + b^2*scale/2^192
///
/// Max intermediate values:
///   a^2*scale: 2^128 * 2^60 = 2^188 (fits U256)
///   2*a*b*scale: 2^221 before shift (fits U256)
///   b^2*scale: 2^252 before shift (fits U256)
fn v3_price(sqrt_price: U256, scale: U256, t0_scale: U256, t1_scale: U256) -> Option<U256> {
    if sqrt_price.is_zero() {
        return None;
    }

    let q96: U256 = U256::from(1u64) << 96;
    let a: U256 = sqrt_price >> 96;
    let b: U256 = sqrt_price & (q96 - U256::from(1u64));

    let term1 = a * a * scale;
    let term2 = if a.is_zero() {
        U256::ZERO
    } else {
        (U256::from(2u64) * a * b * scale) >> 96
    };
    let term3 = if b.is_zero() {
        U256::ZERO
    } else {
        (b * b * scale) >> 192
    };

    let price_raw = term1 + term2 + term3;
    Some((price_raw * t0_scale) / t1_scale)
}

// ─── V3 TWAP helpers ────────────────────────────────────────────────────────

/// Port of Uniswap V3 TickMath.getSqrtRatioAtTick.
/// Converts a tick to sqrtPriceX96 (Q64.96 fixed-point).
fn tick_to_sqrt_price_x96(tick: i32) -> U256 {
    let abs_tick = tick.unsigned_abs();
    assert!(abs_tick <= 887272, "tick out of range");

    let mut ratio: U256 = if abs_tick & 0x1 != 0 {
        U256::from(0xfffcb933bd6fad37aa2d162d1a594001u128)
    } else {
        U256::from(1u128) << 128
    };

    if abs_tick & 0x2 != 0 { ratio = (ratio * U256::from(0xfff97272373d413259a46990580e213au128)) >> 128; }
    if abs_tick & 0x4 != 0 { ratio = (ratio * U256::from(0xfff2e50f5f656932ef12357cf3c7fdccu128)) >> 128; }
    if abs_tick & 0x8 != 0 { ratio = (ratio * U256::from(0xffe5caca7e10e4e61c3624eaa0941cd0u128)) >> 128; }
    if abs_tick & 0x10 != 0 { ratio = (ratio * U256::from(0xffcb9843d60f6159c9db58835c926644u128)) >> 128; }
    if abs_tick & 0x20 != 0 { ratio = (ratio * U256::from(0xff973b41fa98c081472e6896dfb254c0u128)) >> 128; }
    if abs_tick & 0x40 != 0 { ratio = (ratio * U256::from(0xff2ea16466c96a3843ec78b326b52861u128)) >> 128; }
    if abs_tick & 0x80 != 0 { ratio = (ratio * U256::from(0xfe5dee046a99a2a811c461f1969c3053u128)) >> 128; }
    if abs_tick & 0x100 != 0 { ratio = (ratio * U256::from(0xfcbe86c7900a88aedcffc83b479aa3a4u128)) >> 128; }
    if abs_tick & 0x200 != 0 { ratio = (ratio * U256::from(0xf987a7253ac413176f2b074cf7815e54u128)) >> 128; }
    if abs_tick & 0x400 != 0 { ratio = (ratio * U256::from(0xf3392b0822b70005940c7a398e4b70f3u128)) >> 128; }
    if abs_tick & 0x800 != 0 { ratio = (ratio * U256::from(0xe7159475a2c29b7443b29c7fa6e889d9u128)) >> 128; }
    if abs_tick & 0x1000 != 0 { ratio = (ratio * U256::from(0xd097f3bdfd2022b8845ad8f792aa5825u128)) >> 128; }
    if abs_tick & 0x2000 != 0 { ratio = (ratio * U256::from(0xa9f746462d870fdf8a65dc1f90e061e5u128)) >> 128; }
    if abs_tick & 0x4000 != 0 { ratio = (ratio * U256::from(0x70d869a156d2a1b890bb3df62baf32f7u128)) >> 128; }
    if abs_tick & 0x8000 != 0 { ratio = (ratio * U256::from(0x31be135f97d08fd981231505542fcfa6u128)) >> 128; }
    if abs_tick & 0x10000 != 0 { ratio = (ratio * U256::from(0x9aa508b5b7a84e1c677de54f3e99bc9u128)) >> 128; }
    if abs_tick & 0x20000 != 0 { ratio = (ratio * U256::from(0x5d6af8dedb81196699c329225ee604u128)) >> 128; }
    if abs_tick & 0x40000 != 0 { ratio = (ratio * U256::from(0x2216e584f5fa1ea926041bedfe98u128)) >> 128; }
    if abs_tick & 0x80000 != 0 { ratio = (ratio * U256::from(0x48a170391f7dc42444e8fa2u128)) >> 128; }

    if tick > 0 {
        ratio = U256::MAX / ratio;
    }

    // Convert Q128.128 to Q64.96 (right shift 32, round up)
    let remainder_mask: U256 = (U256::from(1u64) << 32) - U256::from(1u64);
    let remainder: U256 = ratio & remainder_mask;
    let round_up: U256 = if remainder.is_zero() { U256::ZERO } else { U256::from(1u64) };
    (ratio >> 32) + round_up
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let config: OracleConfig = env::read();
    let num_samples: u32 = env::read();

    let mut prices: Vec<U256> = Vec::with_capacity(num_samples as usize);
    let mut start_block: u64 = u64::MAX;
    let mut end_block: u64 = 0;
    let mut last_commitment: Option<Vec<u8>> = None;
    let mut last_timestamp: u64 = 0;

    let scale = U256::from(10u64).pow(U256::from(config.decimals));
    let t0dec = config.token0_decimals.unwrap_or(18);
    let t1dec = config.token1_decimals.unwrap_or(18);
    let t0_scale = U256::from(10u64).pow(U256::from(t0dec));
    let t1_scale = U256::from(10u64).pow(U256::from(t1dec));

    for _ in 0..num_samples {
        let input: EthEvmInput = env::read();
        let evm_env = input.into_env(&config.chain_spec);

        let block_number = evm_env.header().number();
        let block_timestamp = evm_env.header().timestamp();

        if block_number < start_block {
            start_block = block_number;
        }
        if block_number > end_block {
            end_block = block_number;
            last_commitment = Some(evm_env.commitment().abi_encode());
            last_timestamp = block_timestamp;
        }

        let price: Option<U256> = match config.dex_type {
            DexType::UniswapV2 => {
                let call = IUniswapV2Pair::getReservesCall {};
                let contract = Contract::new(config.pool, &evm_env);
                let returns = contract.call_builder(&call).call();
                let r0 = U256::from(returns.reserve0);
                let r1 = U256::from(returns.reserve1);
                if r0.is_zero() {
                    None
                } else {
                    // Normalize for token decimals:
                    // price = (r1/10^t1dec) / (r0/10^t0dec) * 10^decimals
                    //       = r1 * t0_scale * scale / (r0 * t1_scale)
                    Some((r1 * scale * t0_scale) / (r0 * t1_scale))
                }
            }
            DexType::UniswapV3 => {
                if config.twap_window > 0 {
                    // TWAP via observe(): flash-loan-immune, geometric-mean price
                    let call = IUniswapV3Pool::observeCall {
                        secondsAgos: vec![config.twap_window, 0],
                    };
                    let contract = Contract::new(config.pool, &evm_env);
                    let returns = contract.call_builder(&call).call();
                    // Compute tick delta from int56 accumulators
                    let tick_delta = returns.tickCumulatives[1] - returns.tickCumulatives[0];
                    let (sign, abs) = tick_delta.into_sign_and_abs();
                    let delta: i64 = {
                        let raw = abs.as_limbs()[0] as i64;
                        if sign == Sign::Negative { -raw } else { raw }
                    };
                    let tw = config.twap_window as i64;
                    // Round towards negative infinity (match Uniswap OracleLibrary)
                    let avg_tick = if delta < 0 && delta % tw != 0 {
                        delta / tw - 1
                    } else {
                        delta / tw
                    };
                    let sqrt_price = tick_to_sqrt_price_x96(avg_tick as i32);
                    v3_price(sqrt_price, scale, t0_scale, t1_scale)
                } else {
                    // Spot price fallback (twap_window = 0)
                    let call = IUniswapV3Pool::slot0Call {};
                    let contract = Contract::new(config.pool, &evm_env);
                    let returns = contract.call_builder(&call).call();
                    v3_price(U256::from(returns.sqrtPriceX96), scale, t0_scale, t1_scale)
                }
            }
            DexType::AlgebraV3 => {
                let call = IAlgebraPool::globalStateCall {};
                let contract = Contract::new(config.pool, &evm_env);
                let returns = contract.call_builder(&call).call();
                v3_price(U256::from(returns.price), scale, t0_scale, t1_scale)
            }
            DexType::CurveStableSwap => {
                // NOTE: This uses balance ratio, which is only an approximation for
                // Curve StableSwap. The exact marginal price requires computing the
                // derivative of the StableSwap invariant (A*n^n*sum + D = A*D*n^n + D^(n+1)/(n^n*prod)).
                // For pools near peg (which stable pools should be), the approximation is close.
                let call0 = ICurvePool::balancesCall { i: U256::ZERO };
                let call1 = ICurvePool::balancesCall { i: U256::from(1u64) };
                let contract = Contract::new(config.pool, &evm_env);
                let b0 = contract.call_builder(&call0).call();
                let b1 = contract.call_builder(&call1).call();
                if b0.is_zero() {
                    None
                } else {
                    Some((b1 * scale * t0_scale) / (b0 * t1_scale))
                }
            }
            DexType::BalancerV2 => {
                let vault = config.aux_address.expect("Balancer requires aux_address (Vault)");
                let pool_id = config.aux_bytes32.expect("Balancer requires aux_bytes32 (poolId)");
                let vault_call = IBalancerVault::getPoolTokensCall { poolId: pool_id.into() };
                let vault_contract = Contract::new(vault, &evm_env);
                let vault_returns = vault_contract.call_builder(&vault_call).call();
                let balances = vault_returns.balances;
                if balances.len() < 2 || balances[0].is_zero() {
                    None
                } else {
                    let weights_call = IBalancerWeightedPool::getNormalizedWeightsCall {};
                    let pool_contract = Contract::new(config.pool, &evm_env);
                    let weights = pool_contract.call_builder(&weights_call).call();
                    if weights.len() < 2 || weights[0].is_zero() || weights[1].is_zero() {
                        None
                    } else {
                        // Balancer spot: (b1 * w0) / (b0 * w1)
                        // Weights are 18-decimal normalized, balances are in token decimals.
                        // Apply decimal normalization.
                        Some((balances[1] * weights[0] * scale * t0_scale) / (balances[0] * weights[1] * t1_scale))
                    }
                }
            }
            DexType::SolidlyStable => {
                // NOTE: Solidly stable pools use x^3*y + y^3*x = k invariant.
                // The reserve ratio is only approximate. For pools near peg this is close.
                let call = ISolidlyPair::getReservesCall {};
                let contract = Contract::new(config.pool, &evm_env);
                let returns = contract.call_builder(&call).call();
                let r0 = U256::from(returns.reserve0);
                let r1 = U256::from(returns.reserve1);
                if r0.is_zero() {
                    None
                } else {
                    Some((r1 * scale * t0_scale) / (r0 * t1_scale))
                }
            }
            DexType::TraderJoeLB => {
                let id_call = ILBPair::getActiveIdCall {};
                let step_call = ILBPair::getBinStepCall {};
                let contract = Contract::new(config.pool, &evm_env);
                let active_id = contract.call_builder(&id_call).call();
                let bin_step = contract.call_builder(&step_call).call();
                let base_id: i64 = 1 << 23;
                let delta: i64 = (active_id.to::<u32>() as i64) - base_id;
                let bs = U256::from(bin_step as u64);
                let ten_k = U256::from(10_000u64);
                let abs_delta = delta.unsigned_abs();
                if abs_delta > 500 {
                    None
                } else {
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
            }
            DexType::DodoPMM => {
                // DODO getMidPrice() returns 18-decimal price. Rescale to oracle decimals.
                let call = IDODO::getMidPriceCall {};
                let contract = Contract::new(config.pool, &evm_env);
                let mid_price = contract.call_builder(&call).call();
                if mid_price.is_zero() {
                    None
                } else {
                    let dodo_scale = U256::from(10u64).pow(U256::from(18u8));
                    Some((mid_price * scale) / dodo_scale)
                }
            }
        };

        if let Some(p) = price {
            prices.push(p);
        }
    }

    // Require at least half of requested samples to be valid.
    let min_required = ((num_samples as usize) + 1) / 2;
    assert!(
        prices.len() >= min_required,
        "Insufficient valid samples: got {}, need at least {}",
        prices.len(),
        min_required
    );

    // Compute median.
    prices.sort();
    let median = if prices.len() % 2 == 0 {
        let mid = prices.len() / 2;
        (prices[mid - 1] + prices[mid]) / U256::from(2u64)
    } else {
        prices[prices.len() / 2]
    };

    let steel_commitment = last_commitment.expect("No blocks processed");

    let journal = OracleJournal {
        steelCommitment: steel_commitment.into(),
        pool: config.pool,
        medianPrice: median,
        decimals: config.decimals,
        numSamples: prices.len() as u32, // Actual valid count, not requested count
        startBlock: start_block,
        endBlock: end_block,
        timestamp: last_timestamp,
    };

    env::commit_slice(&journal.abi_encode());
}
