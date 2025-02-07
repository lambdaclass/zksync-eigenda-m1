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
        bytes memory seal = hex"16d24748f9df1257737b9c2eb91199dcd24df1bb823395bc66d81975e848979b2e6d3883b1e00e7a8c534e565e8e57656bd29ba331d35e746d1b63a12665a1ba0a72286e846c59215965fba407f0f71ca3440cb9e4efb55596760dd02b6361900176e2cb6af68f6eccce3ebbbfe5827aee12a9075e0ca0db6ab95568e767a3051e7c8530aa78645e412d7190bb27edca21abc61bb5008446f4dafbb8dcec3fec121a873f074665f4b4ca455832757c2fa0a2e97d3ad85484999243cf948da8642d1051236ce2e64a9508c2a5235ac63c17298a664ecb50051c382171a7ab93dd06c0620d8558c1c12bb17374de02a0d02638f17db5287884638580d865a17cdb";  
        bytes32 imageId = bytes32(0x2f8feaa8507c4535d9ebb83b40788b761d7a9b23f39ab77cbdf150f67c5901f7);   
        bytes32 journalDigest = bytes32(0x37af74468c1ce2ed0c17eb129175c794b2b1a1a208c749a6469d5123bc5ec3ec);  

        // Call the verify function
        contractInstance.verify(seal, imageId, journalDigest);

        vm.stopBroadcast();
    }
}
