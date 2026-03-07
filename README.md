# ZK Oracle

ZK-verified price feeds for any ERC-20 token with a DEX pool. No vendor approval, no subscriptions, no permission needed. ~$0.01 per update on L2.

Reads on-chain DEX reserves inside the [RISC Zero](https://risczero.com) zkVM, computes a manipulation-resistant median price, and posts it on-chain with a ZK proof. Proof generation is handled by [Boundless](https://boundless.xyz).

Drop-in compatible with Chainlink (`AggregatorV3Interface`), Pyth, RedStone, and Chronicle -- your existing protocol code works with zero changes.

## Get Started

### Prerequisites

- [Rust](https://rustup.rs/) 1.89+
- [Foundry](https://getfoundry.sh/)
- A funded wallet on Base (or your target chain)
- A [Pinata](https://app.pinata.cloud/) JWT for IPFS storage (free tier works)

### 1. Clone and build

```bash
git clone https://github.com/boundless-xyz/zk-oracle
cd zk-oracle
cargo build --release
```

> First build compiles the zkVM guest program and takes a few minutes.

### 2. Run the setup wizard

```bash
cargo run --release --bin wizard
```

The wizard walks you through everything: chain selection, pool address, token detection, `.env` generation, and contract deployment. At the end you'll have a deployed oracle and a ready-to-go config.

### 3. Start the keeper

```bash
cargo run --release --bin keeper
```

The keeper watches the pool, detects when an update is needed (heartbeat or price deviation), and submits ZK proofs automatically. It picks up config from the `.env` the wizard generated.

## How It Works

```
  DEX Pool (on-chain)          Boundless Market
        │                            │
        ▼                            ▼
  ┌───────────┐  proof request  ┌──────────┐
  │  Keeper   │────────────────>│  Provers │
  │           │<────────────────│          │
  └───────────┘  callback/proof └──────────┘
        │
        ▼
  ┌──────────────────────────────┐
  │  ZkOracle.sol                │
  │  verify proof → store price  │
  │  AggregatorV3Interface       │
  └──────────────────────────────┘
```

1. **Sample** -- The keeper reads pool reserves across N blocks via [Steel](https://github.com/risc0/risc0-ethereum/tree/main/crates/steel) storage proofs
2. **Prove** -- The guest program runs inside the zkVM, computes per-block prices, takes the median, and generates a ZK proof via Boundless (~1 min)
3. **Verify** -- The oracle contract verifies the proof on-chain, validates the Steel commitment, runs safety checks, and stores the new price

### Reading the feed

```solidity
// Works exactly like Chainlink
AggregatorV3Interface feed = AggregatorV3Interface(oracleAddress);
(, int256 price, , uint256 updatedAt, ) = feed.latestRoundData();
```

## Supported DEX Types

| DEX | Flag | Examples |
|-----|------|----------|
| Uniswap V2 | `uniswap-v2` | Aerodrome, SushiSwap, PancakeSwap |
| Uniswap V3 | `uniswap-v3` | Uniswap V3 and forks |
| Solidly Stable | `solidly-stable` | Velodrome stable pools |
| Curve | `curve-stable-swap` | Curve StableSwap |
| Balancer V2 | `balancer-v2` | Balancer weighted pools |
| Trader Joe LB | `trader-joe-lb` | Liquidity Book |
| DODO | `dodo-pmm` | DODO PMM |
| Algebra V3 | `algebra-v3` | Camelot, QuickSwap V3 |

## CLI Tools

| Binary | Purpose |
|--------|---------|
| `wizard` | Interactive setup -- configures everything and deploys |
| `keeper` | Automated service -- heartbeat + deviation triggers, persistent state, RPC fallback |
| `boundless` | One-shot -- submit a single proof and update the oracle |

### Keeper details

The keeper runs a loop checking two triggers:
- **Heartbeat**: Update if time since last update exceeds threshold (default: 1 hour)
- **Deviation**: Update if spot price moves more than threshold from on-chain (default: 1%)

It auto-detects whether the oracle supports Boundless callback delivery. If not, it falls back to manual submission (waits for proof, then calls `updatePrice()` directly).

```bash
cargo run --release --bin keeper -- \
  --heartbeat 3600 \
  --deviation-bps 100 \
  --check-interval 60
```

## Contracts

### `ZkOracle.sol` -- Core oracle

- Chainlink `AggregatorV3Interface` (drop-in replacement)
- Dual submission: direct `updatePrice()` or Boundless callback via `handleProof()`
- Upgradeable `imageId` -- update the guest program without redeploying
- Safety parameters: `maxDeviationBps`, `minSamples`, `maxBlockAge`
- Emergency pause, ownership transfer

### `ZkOracleAggregator.sol` -- Multi-pool aggregation

Reads from multiple `ZkOracle` sources and returns the median. Eliminates single-pool risk.

```solidity
address[] memory sources = new address[](3);
sources[0] = aerodromeOracle;
sources[1] = uniswapOracle;
sources[2] = sushiOracle;

ZkOracleAggregator agg = new ZkOracleAggregator(sources, 18, "AERO / WETH", 2);
(, int256 price, , , ) = agg.latestRoundData();
```

### `ZkOracleRegistry.sol` -- Feed discovery

Maps `(token0, token1, dexType)` to oracle addresses for on-chain lookup.

### Oracle adapters

Drop-in wrappers for non-Chainlink interfaces: `PythAdapter.sol`, `RedstoneAdapter.sol`, `ChronicleAdapter.sol`.

## Configuration

All CLI parameters can be set via environment variables. The wizard generates a complete `.env` file, but here's the reference:

| Variable | Description | Default |
|----------|-------------|---------|
| `POOL_ADDRESS` | DEX pool contract address | -- |
| `ORACLE_ADDRESS` | Deployed ZkOracle address | -- |
| `RPC_URL` | Oracle chain RPC | -- |
| `PRIVATE_KEY` | Wallet private key | -- |
| `DEX_TYPE` | Pool type | `uniswap-v2` |
| `NUM_BLOCKS` | Blocks to sample per proof | `5` |
| `BLOCK_STRIDE` | Sample every Nth block | `20` |
| `HEARTBEAT_SECONDS` | Max seconds between updates | `3600` |
| `DEVIATION_BPS` | Deviation trigger in basis points | `100` |
| `PINATA_JWT` | Pinata IPFS JWT | -- |

## Docker

```bash
docker build -t zk-oracle .
docker run --env-file .env zk-oracle              # keeper (default)
docker run --env-file .env zk-oracle boundless     # one-shot
docker run -it --env-file .env zk-oracle wizard    # setup wizard
```

## Limitations

### Update latency
Proof generation takes ~1 minute via Boundless. This can be made faster -- [contact the Boundless team](https://boundless.xyz) for latency-sensitive use cases. Not suitable for applications requiring sub-second prices.

### Volatile and low-liquidity assets
The multi-block median resists manipulation but can't protect against fundamental price instability. For memecoins and micro-cap tokens that move 10-50% in minutes, the oracle price will lag during rapid moves. Don't use as the sole price source for liquidation logic on assets with thin liquidity (<$100K). Consider using `ZkOracleAggregator` with multiple pools and pairing with a circuit breaker.

### Single-pool dependency
Each `ZkOracle` reads from one pool. Use `ZkOracleAggregator` to combine multiple pools and eliminate this risk.

### Gas costs
~200-400K gas per update. Fractions of a cent on L2; several dollars on Ethereum mainnet.

### Chain support
Boundless Market callback delivery is currently available on Base mainnet. On other chains, the keeper falls back to manual submission.

## Deployed Contracts (Base Mainnet)

| Contract | Address |
|----------|---------|
| ZkOracle (AERO/WETH) | `0xF556b835DCC5caEE3FEab16Ffa52746eE2edB734` |
| Boundless Market | `0xfd152dadc5183870710fe54f939eae3ab9f0fe82` |
| RISC Zero Verifier | `0xa326b2eb45a5c3c206df905a58970dca57b8719e` |

## Security

- **ZK proof** guarantees computational integrity -- the median was computed correctly
- **Steel commitment** guarantees data authenticity -- inputs came from real on-chain state
- **V3 TWAP** guarantees flash loan immunity -- tick accumulators can't be manipulated within a transaction

The smart contracts have not been audited. Do not use with significant funds without thorough review.

## License

Apache-2.0
