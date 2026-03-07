// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import {ZkOracle} from "../ZkOracle.sol";

/// @notice IRedstoneAdapter interface for price reading (push model).
interface IRedstoneAdapter {
    function getValueForDataFeed(bytes32 dataFeedId) external view returns (uint256);
    function getValuesForDataFeeds(bytes32[] memory dataFeedIds) external view returns (uint256[] memory);
    function getTimestampsFromLatestUpdate() external view returns (uint128 dataTimestamp, uint128 blockTimestamp);
    function getDataTimestampFromLatestUpdate() external view returns (uint256 lastDataTimestamp);
    function getBlockTimestampFromLatestUpdate() external view returns (uint256 blockTimestamp);
    function getDataFeedIds() external view returns (bytes32[] memory);
    function getDataFeedIndex(bytes32 dataFeedId) external view returns (uint256);
}

/// @title ZkOracleRedstoneAdapter
/// @notice Wraps a ZkOracle to expose the RedStone IRedstoneAdapter interface.
/// @dev Protocols using RedStone push-model adapters can point at this with zero code changes.
///      RedStone uses 8-decimal fixed-point by convention; this adapter scales accordingly.
contract ZkOracleRedstoneAdapter is IRedstoneAdapter {
    ZkOracle public immutable oracle;

    /// @notice The data feed ID this adapter serves (e.g. bytes32("ETH")).
    bytes32 public immutable dataFeedId;

    /// @notice RedStone convention: 8 decimal places.
    uint8 internal constant REDSTONE_DECIMALS = 8;

    error UnsupportedDataFeed(bytes32 id);
    error NoRoundData();
    error NegativePrice();

    constructor(ZkOracle _oracle, bytes32 _dataFeedId) {
        oracle = _oracle;
        dataFeedId = _dataFeedId;
    }

    function getValueForDataFeed(bytes32 _dataFeedId) external view override returns (uint256) {
        if (_dataFeedId != dataFeedId) revert UnsupportedDataFeed(_dataFeedId);
        return _getScaledPrice();
    }

    function getValuesForDataFeeds(bytes32[] memory dataFeedIds) external view override returns (uint256[] memory) {
        uint256[] memory values = new uint256[](dataFeedIds.length);
        for (uint256 i = 0; i < dataFeedIds.length; i++) {
            if (dataFeedIds[i] != dataFeedId) revert UnsupportedDataFeed(dataFeedIds[i]);
            values[i] = _getScaledPrice();
        }
        return values;
    }

    function getTimestampsFromLatestUpdate() external view override returns (uint128 dataTimestamp, uint128 blockTimestamp) {
        if (oracle.currentRoundId() == 0) revert NoRoundData();
        (,,, uint256 updatedAt,) = oracle.latestRoundData();
        return (uint128(updatedAt), uint128(updatedAt));
    }

    function getDataTimestampFromLatestUpdate() external view override returns (uint256 lastDataTimestamp) {
        if (oracle.currentRoundId() == 0) revert NoRoundData();
        (,,, uint256 updatedAt,) = oracle.latestRoundData();
        return updatedAt;
    }

    function getBlockTimestampFromLatestUpdate() external view override returns (uint256 blockTimestamp) {
        if (oracle.currentRoundId() == 0) revert NoRoundData();
        (,,, uint256 updatedAt,) = oracle.latestRoundData();
        return updatedAt;
    }

    function getDataFeedIds() external view override returns (bytes32[] memory) {
        bytes32[] memory ids = new bytes32[](1);
        ids[0] = dataFeedId;
        return ids;
    }

    function getDataFeedIndex(bytes32 _dataFeedId) external view override returns (uint256) {
        if (_dataFeedId != dataFeedId) revert UnsupportedDataFeed(_dataFeedId);
        return 0;
    }

    // ─── Internal ──────────────────────────────────────────────────────

    function _getScaledPrice() internal view returns (uint256) {
        if (oracle.currentRoundId() == 0) revert NoRoundData();
        (, int256 answer,,,) = oracle.latestRoundData();

        // Ensure price is non-negative before casting.
        if (answer < 0) revert NegativePrice();

        // Scale from oracle decimals to RedStone's 8-decimal convention.
        uint8 oracleDecimals = oracle.decimals();
        if (oracleDecimals < REDSTONE_DECIMALS) {
            return uint256(answer) * 10 ** (REDSTONE_DECIMALS - oracleDecimals);
        } else if (oracleDecimals > REDSTONE_DECIMALS) {
            return uint256(answer) / 10 ** (oracleDecimals - REDSTONE_DECIMALS);
        }
        return uint256(answer);
    }
}
