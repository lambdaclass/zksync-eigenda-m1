// ViewBypass.sol
pragma solidity ^0.8.20;

interface IRiscZeroVerifier {
    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest) external view;
}

// Wraps the Risc0 groth16 verifier to make the function not view
contract EigenDARegistry {
    IRiscZeroVerifier public risc0verifier;
    mapping (uint256 => bool) public finishedBatches;
    mapping (uint256 => bool) public verifiedBatches;
    mapping (uint256 => bytes32) public hashes;

    constructor(address _risc0verifier) {
        risc0verifier = IRiscZeroVerifier(_risc0verifier);
    }

    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest, bytes32 eigendaHash, uint256 batchNumber) public {
        try risc0verifier.verify(seal, imageId, journalDigest) {
            finishedBatches[uint256(batchNumber)] = true;
            verifiedBatches[uint256(batchNumber)] = true;
            hashes[uint256(batchNumber)] = eigendaHash;
        } catch {
            finishedBatches[uint256(batchNumber)] = true;
            verifiedBatches[uint256(batchNumber)] = false;
        }
    }
}
