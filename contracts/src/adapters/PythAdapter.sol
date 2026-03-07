// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import {ZkOracle} from "../ZkOracle.sol";

/// @notice Pyth PythStructs.Price return type.
struct PythPrice {
    int64 price;
    uint64 conf;
    int32 expo;
    uint publishTime;
}

/// @notice Minimal IPyth interface for price reading.
interface IPyth {
    function getPrice(bytes32 id) external view returns (PythPrice memory price);
    function getPriceUnsafe(bytes32 id) external view returns (PythPrice memory price);
    function getPriceNoOlderThan(bytes32 id, uint age) external view returns (PythPrice memory price);
    function getEmaPrice(bytes32 id) external view returns (PythPrice memory price);
    function getEmaPriceUnsafe(bytes32 id) external view returns (PythPrice memory price);
    function getEmaPriceNoOlderThan(bytes32 id, uint age) external view returns (PythPrice memory price);
    function getValidTimePeriod() external view returns (uint validTimePeriod);
}

/// @title ZkOraclePythAdapter
/// @notice Wraps a ZkOracle to expose the Pyth IPyth interface.
/// @dev Protocols using Pyth can point at this adapter with zero code changes.
///      The priceId is the keccak256 of the feed description for deterministic mapping.
///      EMA functions return the same spot price (ZK oracle computes median, not EMA).
///      Prices are scaled to Pyth's 8-decimal convention to avoid int64 overflow.
contract ZkOraclePythAdapter is IPyth {
    ZkOracle public immutable oracle;

    /// @notice The price feed ID this adapter serves.
    bytes32 public immutable priceId;

    /// @notice Maximum age (seconds) before getPrice() reverts.
    uint public immutable validTimePeriod;

    /// @notice Pyth convention: 8 decimal places.
    uint8 internal constant PYTH_DECIMALS = 8;

    error StalePrice();
    error PriceFeedNotFound();
    error NoRoundData();
    error PriceOverflow();

    constructor(ZkOracle _oracle, uint _validTimePeriod) {
        oracle = _oracle;
        validTimePeriod = _validTimePeriod;
        priceId = keccak256(abi.encodePacked(_oracle.description()));
    }

    function getValidTimePeriod() external view override returns (uint) {
        return validTimePeriod;
    }

    function getPrice(bytes32 id) external view override returns (PythPrice memory) {
        _checkId(id);
        PythPrice memory p = _latestPrice();
        if (block.timestamp - p.publishTime > validTimePeriod) revert StalePrice();
        return p;
    }

    function getPriceUnsafe(bytes32 id) external view override returns (PythPrice memory) {
        _checkId(id);
        return _latestPrice();
    }

    function getPriceNoOlderThan(bytes32 id, uint age) external view override returns (PythPrice memory) {
        _checkId(id);
        PythPrice memory p = _latestPrice();
        if (block.timestamp - p.publishTime > age) revert StalePrice();
        return p;
    }

    function getEmaPrice(bytes32 id) external view override returns (PythPrice memory) {
        _checkId(id);
        PythPrice memory p = _latestPrice();
        if (block.timestamp - p.publishTime > validTimePeriod) revert StalePrice();
        return p;
    }

    function getEmaPriceUnsafe(bytes32 id) external view override returns (PythPrice memory) {
        _checkId(id);
        return _latestPrice();
    }

    function getEmaPriceNoOlderThan(bytes32 id, uint age) external view override returns (PythPrice memory) {
        _checkId(id);
        PythPrice memory p = _latestPrice();
        if (block.timestamp - p.publishTime > age) revert StalePrice();
        return p;
    }

    // ─── Internal ──────────────────────────────────────────────────────

    function _checkId(bytes32 id) internal view {
        if (id != priceId) revert PriceFeedNotFound();
    }

    function _latestPrice() internal view returns (PythPrice memory) {
        uint80 roundId = oracle.currentRoundId();
        if (roundId == 0) revert NoRoundData();

        (, int256 answer,, uint256 updatedAt,) = oracle.latestRoundData();

        // Scale from oracle decimals to Pyth's 8-decimal convention.
        // This prevents int64 overflow which occurs at 18 decimals for prices > ~9.22.
        uint8 oracleDecimals = oracle.decimals();
        uint256 absAnswer = answer >= 0 ? uint256(answer) : uint256(-answer);

        uint256 scaled;
        if (oracleDecimals > PYTH_DECIMALS) {
            scaled = absAnswer / (10 ** (oracleDecimals - PYTH_DECIMALS));
        } else if (oracleDecimals < PYTH_DECIMALS) {
            scaled = absAnswer * (10 ** (PYTH_DECIMALS - oracleDecimals));
        } else {
            scaled = absAnswer;
        }

        // Verify the scaled value fits in int64 (max ~9.22 * 10^18).
        if (scaled > uint256(uint64(type(int64).max))) revert PriceOverflow();

        int64 scaledPrice = answer >= 0 ? int64(int256(scaled)) : -int64(int256(scaled));

        return PythPrice({
            price: scaledPrice,
            conf: 0,
            expo: -int32(uint32(PYTH_DECIMALS)),
            publishTime: updatedAt
        });
    }
}
