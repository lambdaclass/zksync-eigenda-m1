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
        bytes memory seal = vm.envBytes("SEAL");
        bytes32 imageId = vm.envBytes32("IMAGE_ID");
        bytes32 journalDigest = vm.envBytes32("JOURNAL_DIGEST"); 

        // Call the verify function
        contractInstance.verify(seal, imageId, journalDigest);

        vm.stopBroadcast();
    }
}
