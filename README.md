# ZK Price Feeds

ZK-verified price feeds for any ERC-20 token with a DEX pool. Trustless, cost-efficient, and compatible with existing oracle standards.

## What is this?

A ZK oracle that reads Uniswap V2 pool reserves across multiple blocks inside a [RISC Zero](https://risczero.com) zkVM, computes a manipulation-resistant median price, and posts it on-chain with a proof. No committees, no multisigs — just math.

**Drop-in adapters** for Chainlink, Pyth, RedStone, and Chronicle mean your existing protocol code works with zero changes.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Guest Program (zkVM)                               │
│  • Reads getReserves() across N blocks via Steel    │
│  • Computes median price                            │
│  • Commits journal with proof                       │
└──────────────────────┬──────────────────────────────┘
                       │ ZK Proof + Journal
                       ▼
┌─────────────────────────────────────────────────────┐
│  ZkOracle.sol (on-chain)                            │
│  • Verifies ZK proof via RISC Zero verifier         │
│  • Validates Steel commitment (real chain state)    │
│  • Stores price rounds                              │
│  • Implements Chainlink AggregatorV3Interface       │
└──────────────────────┬──────────────────────────────┘
                       │
          ┌────────────┼────────────┬──────────────┐
          ▼            ▼            ▼              ▼
   PythAdapter  RedstoneAdapter  ChronicleAdapter  (direct)
   IPyth        IRedstoneAdapter IChronicle        latestRoundData()
```

## Project Structure

```
├── contracts/
│   └── src/
│       ├── ZkOracle.sol              # Core oracle + Chainlink interface
│       └── adapters/
│           ├── PythAdapter.sol       # IPyth drop-in
│           ├── RedstoneAdapter.sol   # IRedstoneAdapter drop-in
│           └── ChronicleAdapter.sol  # IChronicle + IToll drop-in
├── methods/
│   └── guest/src/main.rs            # zkVM guest program
├── host/src/bin/
│   ├── main.rs                      # CLI to run the prover
│   └── benchmark.rs                 # Benchmarking tool
├── scripts/
│   └── benchmark_all.py             # Multi-pair benchmark script
└── website/                         # Marketing site (Next.js)
```

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (1.81+)
- [RISC Zero toolchain](https://dev.risczero.com/api/zkvm/install) (`rzup install`)
- [Foundry](https://book.getfoundry.sh/getting-started/installation) (for Solidity)
- An Ethereum RPC URL (Alchemy, Infura, etc.)

### 1. Test locally (execute only, no proof)

```bash
git clone https://github.com/boundless-xyz/zk-oracle
cd zk-oracle

# Execute the guest program without generating a ZK proof.
# Fast -- good for testing the computation is correct.
cargo run --release -- \
  --rpc-url $RPC_URL \
  --pool 0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc \
  --num-blocks 10 \
  --decimals 18 \
  --execute-only
```

### 2. Generate a ZK proof

```bash
# Generate a Groth16 proof (requires RISC Zero prover).
# Takes ~15 seconds. Outputs journal + seal.
cargo run --release -- \
  --rpc-url $RPC_URL \
  --pool 0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc \
  --num-blocks 10 \
  --decimals 18
```

### 3. Deploy the oracle contract

```bash
cd contracts
forge install

forge create src/ZkOracle.sol:ZkOracle \
  --constructor-args \
    <RISC_ZERO_VERIFIER_ADDRESS> \
    <IMAGE_ID> \
    <POOL_ADDRESS> \
    18 \
    "WETH / USDC" \
  --rpc-url $RPC_URL \
  --private-key $PRIVATE_KEY
```

### 4. Prove and submit on-chain (end-to-end)

```bash
# Generate proof AND submit it to the ZkOracle contract.
# This is the full pipeline: read → prove → verify on-chain → store price.
cargo run --release -- \
  --rpc-url $RPC_URL \
  --pool 0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc \
  --num-blocks 10 \
  --decimals 18 \
  --oracle-address <DEPLOYED_ZK_ORACLE_ADDRESS> \
  --private-key $ETH_WALLET_PRIVATE_KEY
```

The host will:
1. Fetch reserves across N blocks via Steel
2. Generate a Groth16 ZK proof (~15 seconds)
3. Verify the on-chain contract's image ID matches
4. Call `updatePrice(journal, seal)` on-chain
5. Confirm the transaction and print the new price round

## Oracle Adapters

### Chainlink (built-in)

`ZkOracle` directly implements `AggregatorV3Interface`. No adapter needed.

```solidity
AggregatorV3Interface feed = AggregatorV3Interface(zkOracleAddress);
(, int256 price, , uint256 updatedAt, ) = feed.latestRoundData();
```

### Pyth

Deploy `ZkOraclePythAdapter` pointing at your `ZkOracle`:

```solidity
import {ZkOraclePythAdapter} from "./adapters/PythAdapter.sol";

// Deploy
ZkOraclePythAdapter adapter = new ZkOraclePythAdapter(oracle, 60); // 60s valid period

// Use exactly like Pyth
PythPrice memory p = IPyth(address(adapter)).getPriceUnsafe(adapter.priceId());
int64 price = p.price;
```

### RedStone

Deploy `ZkOracleRedstoneAdapter` pointing at your `ZkOracle`:

```solidity
import {ZkOracleRedstoneAdapter} from "./adapters/RedstoneAdapter.sol";

// Deploy with data feed ID (e.g. bytes32("ETH"))
ZkOracleRedstoneAdapter adapter = new ZkOracleRedstoneAdapter(oracle, bytes32("ETH"));

// Use exactly like RedStone
uint256 price = IRedstoneAdapter(address(adapter)).getValueForDataFeed(bytes32("ETH"));
```

### Chronicle

Deploy `ZkOracleChronicleAdapter` pointing at your `ZkOracle`:

```solidity
import {ZkOracleChronicleAdapter} from "./adapters/ChronicleAdapter.sol";

// Deploy with feed identifier
ZkOracleChronicleAdapter adapter = new ZkOracleChronicleAdapter(oracle, bytes32("ETH/USD"));

// Whitelist your protocol (Chronicle requires toll access)
adapter.kiss(yourProtocolAddress);

// Use exactly like Chronicle
uint price = IChronicle(address(adapter)).read(); // 18-decimal fixed-point
```

## How It Works

1. **Read**: The zkVM guest program reads `getReserves()` from a Uniswap V2 pair across N blocks using [Steel](https://github.com/boundless-xyz/steel). Every storage read is cryptographically tied to actual Ethereum state.

2. **Prove**: The guest computes the median price and the zkVM generates a proof. ~73M cycles, ~15 seconds proving time, ~$0.004 proving cost.

3. **Verify**: `ZkOracle.sol` verifies the proof on-chain (~250K gas) and stores the new price round. Your protocol calls `latestRoundData()` or whichever adapter interface you prefer.

### Flash Loan Resistance

The oracle computes the median across N blocks (default 10, configurable 5-100). A flash loan manipulates price within a single transaction. To manipulate this oracle, an attacker would need to control pool reserves across a majority of sampled blocks — costing tens of millions in capital at risk over multiple minutes.

## Cost

| Component | Cost |
|-----------|------|
| ZK proving | ~$0.004 per update |
| On-chain verification (L1) | ~250K gas |
| On-chain verification (L2) | ~$0.007 gas |
| **Total per update (L2)** | **~$0.01** |
| Vendor fees | $0 |

## Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| `--num-blocks` | 10 | Number of blocks to sample |
| `--stride` | 1 | Sample every Nth block |
| `--decimals` | 18 | Output price decimal places |
| `--pool` | — | Uniswap V2 pair address |
| `--rpc-url` | — | Ethereum RPC endpoint |

## License

Apache-2.0
