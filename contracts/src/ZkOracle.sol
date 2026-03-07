// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";
import {Steel} from "risc0-steel/Steel.sol";

/// @title IAggregatorV3Interface
/// @notice Chainlink-compatible price feed interface for drop-in replacement.
interface IAggregatorV3Interface {
    function decimals() external view returns (uint8);
    function description() external view returns (string memory);
    function version() external pure returns (uint256);

    function getRoundData(uint80 _roundId)
        external
        view
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);

    function latestRoundData()
        external
        view
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound);
}

/// @title IBoundlessMarketCallback
/// @notice Interface for handling proof callbacks from BoundlessMarket.
interface IBoundlessMarketCallback {
    function handleProof(bytes32 _imageId, bytes calldata journal, bytes calldata seal) external;
}

/// @title ZkOracle
/// @notice A ZK-verified price oracle implementing the Chainlink AggregatorV3Interface.
/// @dev Prices are proven from on-chain DEX reserves using Steel (RISC Zero zkVM).
///      Supports both direct proof submission and Boundless Market callback delivery.
contract ZkOracle is IAggregatorV3Interface, IBoundlessMarketCallback {
    // ─── Immutables ──────────────────────────────────────────────────────

    /// @notice RISC Zero verifier contract.
    IRiscZeroVerifier public immutable VERIFIER;

    /// @notice Image ID of the ZK oracle guest program.
    bytes32 public imageId;

    /// @notice Address of the DEX pool being tracked.
    address public immutable POOL;

    /// @notice Price decimals.
    uint8 internal immutable PRICE_DECIMALS;

    /// @notice Whether to validate Steel commitments on-chain.
    /// @dev Must be true for same-chain deployments. Set false for cross-chain testnet only.
    bool public immutable VALIDATE_COMMITMENT;

    /// @notice Boundless Market contract address (authorized callback sender).
    /// @dev Set to address(0) to disable callback path.
    address public immutable BOUNDLESS_MARKET;

    /// @notice Hash of the oracle config parameters (dex type, decimals, twap window, etc.).
    /// @dev Binds the ZK proof to the exact config so a prover can't substitute parameters.
    bytes32 public immutable CONFIG_HASH;

    // ─── Safety parameters ───────────────────────────────────────────────

    /// @notice Maximum deviation from last price (in basis points). 0 = disabled.
    uint256 public maxDeviationBps;

    /// @notice Minimum valid samples required in a proof.
    uint32 public minSamples;

    /// @notice Maximum staleness: reject proofs if block.number - endBlock exceeds this.
    /// @dev Set to 0 to disable (useful when oracle chain differs from source chain).
    uint64 public maxBlockAge;

    // ─── State ───────────────────────────────────────────────────────────

    /// @notice Human-readable description (e.g. "ETH / USDC").
    string internal feedDescription;

    /// @notice Stored round data.
    struct Round {
        int256 answer;
        uint64 startBlock;
        uint64 endBlock;
        uint32 numSamples;
        uint64 updatedAt;
        uint64 timestamp;
    }

    /// @notice Current round ID (incremented on each update).
    uint80 public currentRoundId;

    /// @notice Mapping of round ID to round data.
    mapping(uint80 => Round) internal rounds;

    /// @notice Owner for admin operations.
    address public owner;

    /// @notice Emergency pause flag.
    bool public paused;

    // ─── Events ──────────────────────────────────────────────────────────

    event PriceUpdated(uint80 indexed roundId, int256 answer, uint64 startBlock, uint64 endBlock, uint32 numSamples);
    event Paused(address account);
    event Unpaused(address account);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event SafetyParamsUpdated(uint256 maxDeviationBps, uint32 minSamples, uint64 maxBlockAge);
    event ImageIdUpdated(bytes32 indexed oldImageId, bytes32 indexed newImageId);

    // ─── Errors ──────────────────────────────────────────────────────────

    /// @notice Journal structure matching the guest program output.
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

    error InvalidPool(address expected, address received);
    error InvalidDecimals(uint8 expected, uint8 received);
    error RoundNotFound(uint80 roundId);
    error NoRoundData();
    error OraclePaused();
    error NotOwner();
    error InsufficientSamples(uint32 got, uint32 required);
    error PriceOverflow();
    error StaleBlockRange(uint64 endBlock, uint64 lastEndBlock);
    error ProofTooOld(uint64 endBlock, uint64 currentBlock, uint64 maxAge);
    error DeviationTooLarge(uint256 newPrice, uint256 lastPrice, uint256 maxBps);
    error InvalidCallbackSender();
    error InvalidImageId();
    error InvalidConfigHash(bytes32 expected, bytes32 received);

    // ─── Modifiers ───────────────────────────────────────────────────────

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    modifier whenNotPaused() {
        if (paused) revert OraclePaused();
        _;
    }

    // ─── Constructor ─────────────────────────────────────────────────────

    constructor(
        IRiscZeroVerifier _verifier,
        bytes32 _imageId,
        address _pool,
        uint8 _decimals,
        string memory _description,
        bool _validateCommitment,
        address _boundlessMarket,
        bytes32 _configHash
    ) {
        VERIFIER = _verifier;
        imageId = _imageId;
        POOL = _pool;
        PRICE_DECIMALS = _decimals;
        VALIDATE_COMMITMENT = _validateCommitment;
        BOUNDLESS_MARKET = _boundlessMarket;
        CONFIG_HASH = _configHash;
        feedDescription = _description;
        owner = msg.sender;

        // Defaults: 50% max deviation, 3 min samples, 256-block max age
        maxDeviationBps = 5000;
        minSamples = 3;
        maxBlockAge = 256;
    }

    // ─── Admin ───────────────────────────────────────────────────────────

    function pause() external onlyOwner {
        paused = true;
        emit Paused(msg.sender);
    }

    function unpause() external onlyOwner {
        paused = false;
        emit Unpaused(msg.sender);
    }

    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "Zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }

    function setSafetyParams(uint256 _maxDeviationBps, uint32 _minSamples, uint64 _maxBlockAge) external onlyOwner {
        maxDeviationBps = _maxDeviationBps;
        minSamples = _minSamples;
        maxBlockAge = _maxBlockAge;
        emit SafetyParamsUpdated(_maxDeviationBps, _minSamples, _maxBlockAge);
    }

    function setImageId(bytes32 _imageId) external onlyOwner {
        bytes32 oldImageId = imageId;
        imageId = _imageId;
        emit ImageIdUpdated(oldImageId, _imageId);
    }

    // ─── Core: Direct submission ──────────────────────────────────────────

    /// @notice Submit a ZK proof to update the price feed (direct path).
    /// @param journalData ABI-encoded OracleJournal from the guest program.
    /// @param seal The RISC Zero proof seal.
    function updatePrice(bytes calldata journalData, bytes calldata seal) external whenNotPaused {
        bytes32 journalHash = sha256(journalData);
        VERIFIER.verify(seal, imageId, journalHash);
        _applyPriceUpdate(journalData);
    }

    // ─── Core: Boundless callback ─────────────────────────────────────────

    /// @notice Handle proof delivered by the Boundless Market callback mechanism.
    /// @dev Called by BoundlessMarket after proof fulfillment. The market already verified
    ///      the proof, but we re-verify for defense-in-depth.
    function handleProof(bytes32 _imageId, bytes calldata journal, bytes calldata seal) external whenNotPaused {
        if (msg.sender != BOUNDLESS_MARKET) revert InvalidCallbackSender();
        if (_imageId != imageId) revert InvalidImageId();
        bytes32 journalHash = sha256(journal);
        VERIFIER.verify(seal, imageId, journalHash);
        _applyPriceUpdate(journal);
    }

    // ─── Internal ─────────────────────────────────────────────────────────

    /// @notice Decode journal, validate, and store the new price round.
    function _applyPriceUpdate(bytes calldata journalData) internal {
        OracleJournal memory journal = abi.decode(journalData, (OracleJournal));

        if (journal.pool != POOL) revert InvalidPool(POOL, journal.pool);
        if (journal.journalDecimals != PRICE_DECIMALS) revert InvalidDecimals(PRICE_DECIMALS, journal.journalDecimals);
        if (CONFIG_HASH != bytes32(0) && journal.configHash != CONFIG_HASH) {
            revert InvalidConfigHash(CONFIG_HASH, journal.configHash);
        }

        if (VALIDATE_COMMITMENT) {
            // Validate ALL block commitments to ensure every sampled block is from the real chain.
            for (uint256 i = 0; i < journal.steelCommitments.length; i++) {
                Steel.Commitment memory commitment = abi.decode(journal.steelCommitments[i], (Steel.Commitment));
                require(Steel.validateCommitment(commitment), "Invalid Steel commitment");
            }
        }

        if (journal.numSamples < minSamples) {
            revert InsufficientSamples(journal.numSamples, minSamples);
        }

        if (journal.medianPrice > uint256(type(int256).max)) revert PriceOverflow();

        if (currentRoundId > 0) {
            if (journal.endBlock <= rounds[currentRoundId].endBlock) {
                revert StaleBlockRange(journal.endBlock, rounds[currentRoundId].endBlock);
            }
        }

        if (maxBlockAge > 0 && block.number > journal.endBlock) {
            if (block.number - journal.endBlock > maxBlockAge) {
                revert ProofTooOld(journal.endBlock, uint64(block.number), maxBlockAge);
            }
        }

        if (currentRoundId > 0 && maxDeviationBps > 0) {
            int256 lastPrice = rounds[currentRoundId].answer;
            if (lastPrice > 0) {
                uint256 absLast = uint256(lastPrice);
                uint256 absNew = journal.medianPrice;
                uint256 absDiff = absNew > absLast ? absNew - absLast : absLast - absNew;
                if (absDiff * 10000 > absLast * maxDeviationBps) {
                    revert DeviationTooLarge(absNew, absLast, maxDeviationBps);
                }
            }
        }

        currentRoundId++;
        rounds[currentRoundId] = Round({
            answer: int256(journal.medianPrice),
            startBlock: journal.startBlock,
            endBlock: journal.endBlock,
            numSamples: journal.numSamples,
            updatedAt: uint64(block.timestamp),
            timestamp: journal.timestamp
        });

        emit PriceUpdated(currentRoundId, int256(journal.medianPrice), journal.startBlock, journal.endBlock, journal.numSamples);
    }

    // ─── AggregatorV3Interface ───────────────────────────────────────────

    /// @inheritdoc IAggregatorV3Interface
    function decimals() external view override returns (uint8) {
        return PRICE_DECIMALS;
    }

    /// @inheritdoc IAggregatorV3Interface
    function description() external view override returns (string memory) {
        return feedDescription;
    }

    /// @inheritdoc IAggregatorV3Interface
    function version() external pure override returns (uint256) {
        return 2;
    }

    /// @inheritdoc IAggregatorV3Interface
    function getRoundData(uint80 _roundId)
        external
        view
        override
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)
    {
        Round storage round = rounds[_roundId];
        if (round.updatedAt == 0) revert RoundNotFound(_roundId);

        return (
            _roundId,
            round.answer,
            uint256(round.timestamp),
            uint256(round.updatedAt),
            _roundId
        );
    }

    /// @inheritdoc IAggregatorV3Interface
    function latestRoundData()
        external
        view
        override
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)
    {
        if (currentRoundId == 0) revert NoRoundData();
        Round storage round = rounds[currentRoundId];

        return (
            currentRoundId,
            round.answer,
            uint256(round.timestamp),
            uint256(round.updatedAt),
            currentRoundId
        );
    }

    // ─── View helpers ────────────────────────────────────────────────────

    /// @notice Returns the full round metadata including block range and sample count.
    function getRoundMetadata(uint80 _roundId)
        external
        view
        returns (uint64 startBlock, uint64 endBlock, uint32 numSamples, uint64 updatedAt, uint64 timestamp)
    {
        Round storage round = rounds[_roundId];
        if (round.updatedAt == 0) revert RoundNotFound(_roundId);
        return (round.startBlock, round.endBlock, round.numSamples, round.updatedAt, round.timestamp);
    }
}
