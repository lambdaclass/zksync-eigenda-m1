// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {Script} from "forge-std/Script.sol";
import {EigenDARegistry} from "../src/EigenDARegistry.sol";
import "forge-std/console.sol";
import {ERC1967Proxy} from "openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import {BlobVerifierWrapper} from "../src/BlobVerifierWrapper.sol";
import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";
import {ControlID, RiscZeroGroth16Verifier} from "risc0-ethereum/groth16/RiscZeroGroth16Verifier.sol";

contract ContractsDeployer is Script {
    // Environment variable name for the BlobVerifier address
    string constant BLOB_VERIFIER_ENV_VAR = "BLOB_VERIFIER_ADDRESS";

    function run() external {
        // Get deployer's private key from env
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        // Get BlobVerifier address from environment variables
        address blobVerifier = vm.envAddress(BLOB_VERIFIER_ENV_VAR);

        // Broadcast deployment transaction
        vm.startBroadcast(deployerPrivateKey);
        vm.txGasPrice( 0.000000002  gwei);

        BlobVerifierWrapper wrapper = new BlobVerifierWrapper(blobVerifier);
        console.log("BlobVerifierWrapper deployed at:", address(wrapper));

        IRiscZeroVerifier verifier = new RiscZeroGroth16Verifier(hex"8cdad9242664be3112aba377c5425a4df735eb1c6966472b561d2855932c0469", ControlID.BN254_CONTROL_ID);
        
        // Deploy EigenDARegistry
        EigenDARegistry implementation = new EigenDARegistry();

        // Encode initializer call
        bytes memory initializerData = abi.encodeWithSignature(
            "initialize(address,address)",
            address(verifier),
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
