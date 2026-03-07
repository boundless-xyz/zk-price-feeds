// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import "forge-std/Script.sol";
import {ZkOracle} from "../src/ZkOracle.sol";
import {IRiscZeroVerifier, Receipt} from "risc0-ethereum/IRiscZeroVerifier.sol";

/// @notice Mock verifier that accepts all proofs (for testnet only).
contract MockVerifier is IRiscZeroVerifier {
    function verify(bytes calldata, bytes32, bytes32) external pure {}
    function verifyIntegrity(Receipt calldata) external pure {}
}

contract DeployMockZkOracle is Script {
    function run() external {
        bytes32 imageId = vm.envBytes32("IMAGE_ID");
        address pool = vm.envAddress("POOL_ADDRESS");
        uint8 decimals = uint8(vm.envUint("PRICE_DECIMALS"));
        string memory description = vm.envString("FEED_DESCRIPTION");

        vm.startBroadcast();

        // Deploy mock verifier
        MockVerifier mockVerifier = new MockVerifier();

        // Deploy oracle pointing at mock verifier (commitment validation disabled for testnet)
        ZkOracle oracle = new ZkOracle(
            IRiscZeroVerifier(address(mockVerifier)),
            imageId,
            pool,
            decimals,
            description,
            false, // validateCommitment = false for cross-chain testnet
            address(0), // no Boundless Market callback
            bytes32(0) // no config hash enforcement in testnet
        );

        vm.stopBroadcast();

        console.log("MockVerifier deployed at:", address(mockVerifier));
        console.log("ZkOracle deployed at:", address(oracle));
        console.log("  Image ID:", vm.toString(imageId));
        console.log("  Pool:", pool);
        console.log("  Decimals:", decimals);
        console.log("  Description:", description);
        console.log("  Commitment validation: DISABLED (testnet)");
    }
}
