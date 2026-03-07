// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.26;

import "forge-std/Script.sol";
import {ZkOracle} from "../src/ZkOracle.sol";
import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";

contract DeployZkOracle is Script {
    function run() external {
        // Read from environment
        address verifier = vm.envAddress("VERIFIER_ADDRESS");
        bytes32 imageId = vm.envBytes32("IMAGE_ID");
        address pool = vm.envAddress("POOL_ADDRESS");
        uint8 decimals = uint8(vm.envUint("PRICE_DECIMALS"));
        string memory description = vm.envString("FEED_DESCRIPTION");
        bool validateCommitment = vm.envOr("VALIDATE_COMMITMENT", true);
        address boundlessMarket = vm.envOr("BOUNDLESS_MARKET_ADDRESS", address(0));

        // Compute config hash from oracle parameters (must match guest computation)
        uint8 dexType = uint8(vm.envOr("DEX_TYPE_ID", uint256(0)));
        uint8 token0Decimals = uint8(vm.envOr("TOKEN0_DECIMALS", uint256(18)));
        uint8 token1Decimals = uint8(vm.envOr("TOKEN1_DECIMALS", uint256(18)));
        uint32 twapWindow = uint32(vm.envOr("TWAP_WINDOW", uint256(0)));
        bytes32 configHash = keccak256(abi.encodePacked(pool, dexType, token0Decimals, token1Decimals, decimals, twapWindow));

        vm.startBroadcast();

        ZkOracle oracle = new ZkOracle(
            IRiscZeroVerifier(verifier),
            imageId,
            pool,
            decimals,
            description,
            validateCommitment,
            boundlessMarket,
            configHash
        );

        vm.stopBroadcast();

        console.log("ZkOracle deployed at:", address(oracle));
        console.log("  Verifier:", verifier);
        console.log("  Image ID:", vm.toString(imageId));
        console.log("  Pool:", pool);
        console.log("  Decimals:", decimals);
        console.log("  Config Hash:", vm.toString(configHash));
        console.log("  Description:", description);
        console.log("  Boundless Market:", boundlessMarket);
    }
}
