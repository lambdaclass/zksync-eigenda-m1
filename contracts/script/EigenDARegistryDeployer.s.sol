// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {Script} from "forge-std/Script.sol";
import {EigenDARegistry} from "../src/EigenDARegistry.sol";
import "forge-std/console.sol";
import {ERC1967Proxy} from "openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";


contract EigenDARegistryDeployer is Script {
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
        
        // Deploy EigenDARegistry
        EigenDARegistry implementation = new EigenDARegistry();

        // Encode initializer call
        bytes memory initializerData = abi.encodeWithSignature(
            "initialize(address,address)",
            risc0Verifier,
            vm.addr(deployerPrivateKey) // Set the deployer as owner
        );

        // Deploy the proxy pointing to the implementation
        ERC1967Proxy proxy = new ERC1967Proxy(
            address(implementation),
            initializerData
        );
        console.log("EigenDARegistry Proxy deployed at:", address(proxy));
        
        vm.stopBroadcast();
    }
}
