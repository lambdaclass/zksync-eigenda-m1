// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {Script} from "forge-std/Script.sol";
import {Risc0ProofVerifierWrapper} from "../src/Risc0ProofVerifierWrapper.sol";
import "forge-std/console.sol";


contract Risc0ProofVerifierWrapperDeployer is Script {
    // Environment variable name for the BlobVerifier address
    string constant RISC0_VERIFIER_ADDRESS = "RISC0_VERIFIER_ADDRESS";

    function run() external {
        // Get deployer's private key from env
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        // Get BlobVerifier address from environment variables
        address risc0Verifier = vm.envAddress(RISC0_VERIFIER_ADDRESS);

        // Broadcast deployment transaction
        vm.startBroadcast(deployerPrivateKey);
        vm.txGasPrice( 0.000000002  gwei);
        
        // Deploy VerifierWrapper with the specified BlobVerifier address
        Risc0ProofVerifierWrapper wrapper = new Risc0ProofVerifierWrapper(risc0Verifier);
        
        vm.stopBroadcast();

        // Log the deployed address
        console.log("Risc0ProofVerifierWrapper deployed at:", address(wrapper));
    }
}
