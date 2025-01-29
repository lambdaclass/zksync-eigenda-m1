// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {Script} from "forge-std/Script.sol";
import {VerifierWrapper} from "./VerifierWrapper.sol";

contract Deployer is Script {
    // Environment variable name for the BlobVerifier address
    string constant BLOB_VERIFIER_ENV_VAR = "BLOB_VERIFIER_ADDRESS";

    function run() external {
        // Get deployer's private key from env
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        // Get BlobVerifier address from environment variables
        address blobVerifier = vm.envAddress(BLOB_VERIFIER_ENV_VAR);

        // Broadcast deployment transaction
        vm.startBroadcast(deployerPrivateKey);
        
        // Deploy VerifierWrapper with the specified BlobVerifier address
        VerifierWrapper wrapper = new VerifierWrapper(blobVerifier);
        
        vm.stopBroadcast();

        // Log the deployed address
        console.log("VerifierWrapper deployed at:", address(wrapper));
    }
}
