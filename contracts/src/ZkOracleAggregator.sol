// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import {IAggregatorV3Interface} from "./ZkOracle.sol";

/// @title ZkOracleAggregator
/// @notice Aggregates multiple ZkOracle sources into a single Chainlink-compatible feed.
/// @dev Reads prices from N independent ZkOracle instances and returns the median.
///      Eliminates single-pool dependency -- if one pool is manipulated or loses liquidity,
///      the median across multiple pools remains accurate.
contract ZkOracleAggregator is IAggregatorV3Interface {
    // ─── State ───────────────────────────────────────────────────────────

    /// @notice Ordered list of source oracle addresses.
    address[] public sources;

    /// @notice Price decimals (must match all source oracles).
    uint8 public immutable PRICE_DECIMALS;

    /// @notice Human-readable description.
    string internal feedDescription;

    /// @notice Owner for admin operations.
    address public owner;

    /// @notice Minimum number of sources that must have data for the aggregator to return a price.
    uint256 public minSources;

    /// @notice Maximum staleness in seconds. Sources older than this are excluded.
    /// @dev Set to 0 to disable staleness check.
    uint256 public maxStaleness;

    // ─── Events ──────────────────────────────────────────────────────────

    event SourceAdded(address indexed source);
    event SourceRemoved(address indexed source);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event MinSourcesUpdated(uint256 oldMin, uint256 newMin);
    event MaxStalenessUpdated(uint256 oldMax, uint256 newMax);

    // ─── Errors ──────────────────────────────────────────────────────────

    error NotOwner();
    error NoSources();
    error SourceAlreadyExists(address source);
    error SourceNotFound(address source);
    error InsufficientSources(uint256 available, uint256 required);
    error NoRoundData();
    error RoundNotTracked(uint80 roundId);

    // ─── Modifiers ───────────────────────────────────────────────────────

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    // ─── Constructor ─────────────────────────────────────────────────────

    /// @param _sources Initial list of ZkOracle source addresses.
    /// @param _decimals Price decimals (must match all sources).
    /// @param _description Human-readable feed description (e.g. "AERO / WETH (3-pool)").
    /// @param _minSources Minimum sources required to return a price.
    constructor(
        address[] memory _sources,
        uint8 _decimals,
        string memory _description,
        uint256 _minSources
    ) {
        if (_sources.length == 0) revert NoSources();
        sources = _sources;
        PRICE_DECIMALS = _decimals;
        feedDescription = _description;
        minSources = _minSources > 0 ? _minSources : 1;
        maxStaleness = 7200; // Default: 2 hours
        owner = msg.sender;
    }

    // ─── Admin ───────────────────────────────────────────────────────────

    function addSource(address source) external onlyOwner {
        for (uint256 i = 0; i < sources.length; i++) {
            if (sources[i] == source) revert SourceAlreadyExists(source);
        }
        sources.push(source);
        emit SourceAdded(source);
    }

    function removeSource(address source) external onlyOwner {
        bool found = false;
        for (uint256 i = 0; i < sources.length; i++) {
            if (sources[i] == source) {
                sources[i] = sources[sources.length - 1];
                sources.pop();
                found = true;
                break;
            }
        }
        if (!found) revert SourceNotFound(source);
        emit SourceRemoved(source);
    }

    function setMinSources(uint256 _minSources) external onlyOwner {
        uint256 old = minSources;
        minSources = _minSources > 0 ? _minSources : 1;
        emit MinSourcesUpdated(old, minSources);
    }

    function setMaxStaleness(uint256 _maxStaleness) external onlyOwner {
        uint256 old = maxStaleness;
        maxStaleness = _maxStaleness;
        emit MaxStalenessUpdated(old, _maxStaleness);
    }

    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "Zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }

    function sourceCount() external view returns (uint256) {
        return sources.length;
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
        return 1;
    }

    /// @inheritdoc IAggregatorV3Interface
    /// @dev The aggregator does not track historical rounds. Use individual source oracles
    ///      for historical data. Only roundId 0 (latest) is supported.
    function getRoundData(uint80 _roundId)
        external
        view
        override
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)
    {
        if (_roundId != 0) revert RoundNotTracked(_roundId);
        return this.latestRoundData();
    }

    /// @inheritdoc IAggregatorV3Interface
    /// @notice Returns the median price across all source oracles.
    function latestRoundData()
        external
        view
        override
        returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)
    {
        uint256 len = sources.length;
        int256[] memory prices = new int256[](len);
        uint256 latestUpdatedAt = 0;
        uint256 latestStartedAt = 0;
        uint80 maxRoundId = 0;
        uint256 validCount = 0;

        for (uint256 i = 0; i < len; i++) {
            try IAggregatorV3Interface(sources[i]).latestRoundData() returns (
                uint80 rid, int256 ans, uint256 sa, uint256 ua, uint80
            ) {
                if (ans > 0) {
                    // Skip stale sources
                    if (maxStaleness > 0 && block.timestamp > ua && block.timestamp - ua > maxStaleness) {
                        continue;
                    }
                    prices[validCount] = ans;
                    validCount++;
                    if (ua > latestUpdatedAt) latestUpdatedAt = ua;
                    if (sa > latestStartedAt) latestStartedAt = sa;
                    if (rid > maxRoundId) maxRoundId = rid;
                }
            } catch {
                // Source failed -- skip it.
            }
        }

        if (validCount < minSources) revert InsufficientSources(validCount, minSources);
        if (validCount == 0) revert NoRoundData();

        // Sort and take median.
        _sort(prices, validCount);
        int256 median;
        if (validCount % 2 == 1) {
            median = prices[validCount / 2];
        } else {
            median = (prices[validCount / 2 - 1] + prices[validCount / 2]) / 2;
        }

        return (maxRoundId, median, latestStartedAt, latestUpdatedAt, maxRoundId);
    }

    // ─── Internal ─────────────────────────────────────────────────────────

    /// @dev Insertion sort for small arrays (expected 2-10 sources).
    function _sort(int256[] memory arr, uint256 len) internal pure {
        for (uint256 i = 1; i < len; i++) {
            int256 key = arr[i];
            uint256 j = i;
            while (j > 0 && arr[j - 1] > key) {
                arr[j] = arr[j - 1];
                j--;
            }
            arr[j] = key;
        }
    }
}
