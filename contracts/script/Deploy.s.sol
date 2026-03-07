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

        vm.startBroadcast();

        ZkOracle oracle = new ZkOracle(
            IRiscZeroVerifier(verifier),
            imageId,
            pool,
            decimals,
            description,
            validateCommitment,
            boundlessMarket
        );

        vm.stopBroadcast();

        console.log("ZkOracle deployed at:", address(oracle));
        console.log("  Verifier:", verifier);
        console.log("  Image ID:", vm.toString(imageId));
        console.log("  Pool:", pool);
        console.log("  Decimals:", decimals);
        console.log("  Description:", description);
        console.log("  Boundless Market:", boundlessMarket);
    }
}
