// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";

interface IRiscZeroVerifier {
    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest) external view;
}

contract ProofVerifier is Script {
    function run() external {
        // Load the deployer's private key from env (make sure it's funded)
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");

        // Define RPC URL and contract address
        address contractAddress = address(0xAC292cF957Dd5BA174cdA13b05C16aFC71700327);

        // Start broadcasting (not needed for view functions but useful for txs)
        vm.startBroadcast(deployerPrivateKey);

        // Contract instance
        IRiscZeroVerifier contractInstance = IRiscZeroVerifier(contractAddress);

        // Define parameters
        bytes memory seal = hex"c101b42b2461eb5184fb58ce7171bda5f50fd4b558b5d44b01a8a7c8a85d6692cc059d9f08514b82dbc0280911e2f8984861a90080e511cf729bfa63b57bf3a7a9538e611fa73bb004f4b91d2db1a1214d0f6b630f6d81d22e9adebda33539fd78e731c91e81f20e9fb39d55b24ef157012222891eb8ffd22c63c85706f2699215e8eb5308d732d490705a72e9e2d9f5f8b5b7c0d3fc54dfa2e12271acce0dbc4ffbc9ce22d606d9b61d943c8483f6412688c1e06bdcd48deb7499fb9c88842ab5f0a35d2b1841baf8c292b4fa0b5cf2bbcc80ea1eab0f4182bf6c320f7d2a910d469b902324a4914372aebe381c8450ca40f0035f2eaeff606310bd02bac25c3a69a50a";  
        bytes32 imageId = bytes32(0x9d5bf6aca18a7b346d4c0083b81619f8854541d3fc20e52e46ebe7f373b7f05d);   
        bytes32 journalDigest = bytes32(0xe6d781479571100d2efb5df6c7f658fb47fd46e5e5ad95129fb8202bbbfa3355);  

        // Call the verify function
        contractInstance.verify(seal, imageId, journalDigest);

        vm.stopBroadcast();
    }
}
