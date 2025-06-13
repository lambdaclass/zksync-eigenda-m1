// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import {Script} from "forge-std/Script.sol";
import "forge-std/console.sol";
import {IRiscZeroVerifier} from "risc0-ethereum/IRiscZeroVerifier.sol";
import {ControlID, RiscZeroGroth16Verifier} from "risc0-ethereum/groth16/RiscZeroGroth16Verifier.sol";

contract ContractsDeployer is Script {
    function run() external {
        // Get deployer's private key from env
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        // Broadcast deployment transaction
        vm.startBroadcast(deployerPrivateKey);
        vm.txGasPrice( 0.000000002  gwei);

        IRiscZeroVerifier verifier = new RiscZeroGroth16Verifier(hex"8cdad9242664be3112aba377c5425a4df735eb1c6966472b561d2855932c0469", ControlID.BN254_CONTROL_ID);
        
        console.log("RiscZeroVerifier deployed at:", address(verifier));
        
        vm.stopBroadcast();
    }
}
