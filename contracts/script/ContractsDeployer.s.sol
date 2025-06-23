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
        
        IRiscZeroVerifier verifier = new RiscZeroGroth16Verifier(hex"884389273e128b32475b334dec75ee619b77cb33d41c332021fe7e44c746ee60", hex"04446e66d300eb7fb45c9726bb53c793dda407a62e9601618bb43c5c14657ac0");
        
        console.log("RiscZeroVerifier deployed at:", address(verifier));
        
        vm.stopBroadcast();
    }
}
