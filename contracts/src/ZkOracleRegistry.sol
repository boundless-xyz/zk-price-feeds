// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import {ZkOracle, IAggregatorV3Interface} from "./ZkOracle.sol";

/// @title ZkOracleRegistry
/// @notice Registry for discovering ZK Oracle price feeds.
contract ZkOracleRegistry {
    struct FeedInfo {
        address oracle;
        address pool;
        string description;
        uint8 decimals;
    }

    /// @notice Feed key => oracle address. Key is keccak256(abi.encodePacked(token0, token1, dexType)).
    mapping(bytes32 => address) public feeds;

    /// @notice All registered feed keys for enumeration.
    bytes32[] public feedKeys;

    /// @notice Oracle address => feed info.
    mapping(address => FeedInfo) public feedInfo;

    address public owner;

    event FeedRegistered(bytes32 indexed key, address indexed oracle, string description);
    event FeedRemoved(bytes32 indexed key, address indexed oracle);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    error NotOwner();
    error FeedExists(bytes32 key);
    error FeedNotFound(bytes32 key);

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    constructor() {
        owner = msg.sender;
    }

    function registerFeed(
        address token0,
        address token1,
        uint8 dexType,
        address oracle
    ) external onlyOwner {
        bytes32 key = keccak256(abi.encodePacked(token0, token1, dexType));
        if (feeds[key] != address(0)) revert FeedExists(key);

        ZkOracle oracleContract = ZkOracle(oracle);
        feeds[key] = oracle;
        feedKeys.push(key);
        feedInfo[oracle] = FeedInfo({
            oracle: oracle,
            pool: oracleContract.POOL(),
            description: oracleContract.description(),
            decimals: oracleContract.decimals()
        });

        emit FeedRegistered(key, oracle, oracleContract.description());
    }

    function removeFeed(
        address token0,
        address token1,
        uint8 dexType
    ) external onlyOwner {
        bytes32 key = keccak256(abi.encodePacked(token0, token1, dexType));
        address oracle = feeds[key];
        if (oracle == address(0)) revert FeedNotFound(key);

        delete feeds[key];
        delete feedInfo[oracle];

        // Remove from feedKeys array (swap and pop)
        for (uint256 i = 0; i < feedKeys.length; i++) {
            if (feedKeys[i] == key) {
                feedKeys[i] = feedKeys[feedKeys.length - 1];
                feedKeys.pop();
                break;
            }
        }

        emit FeedRemoved(key, oracle);
    }

    function getFeed(address token0, address token1, uint8 dexType) external view returns (address) {
        return feeds[keccak256(abi.encodePacked(token0, token1, dexType))];
    }

    function getFeedCount() external view returns (uint256) {
        return feedKeys.length;
    }

    function getLatestPrice(address token0, address token1, uint8 dexType) external view returns (int256 price, uint256 updatedAt) {
        address oracle = feeds[keccak256(abi.encodePacked(token0, token1, dexType))];
        require(oracle != address(0), "Feed not found");
        (, price, , updatedAt, ) = IAggregatorV3Interface(oracle).latestRoundData();
    }

    function transferOwnership(address newOwner) external onlyOwner {
        require(newOwner != address(0), "Zero address");
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }
}
