// ViewBypass.sol
pragma solidity ^0.8.20;

interface IRiscZeroVerifier {
    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest) external view;
}

// Wraps the Risc0 groth16 verifier to make the function not view
contract EigenDARegistry {
    IRiscZeroVerifier public risc0verifier;
    mapping (bytes => bool) public finishedBatches;
    mapping (bytes => bool) public verifiedBatches;
    mapping (bytes => bytes32) public hashes;

    constructor(address _risc0verifier) {
        risc0verifier = IRiscZeroVerifier(_risc0verifier);
    }

    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest, bytes32 eigendaHash, bytes calldata inclusion_data) public {
        try risc0verifier.verify(seal, imageId, journalDigest) {
            finishedBatches[inclusion_data] = true;
            verifiedBatches[inclusion_data] = true;
            hashes[inclusion_data] = eigendaHash;
        } catch {
            finishedBatches[inclusion_data] = true;
            verifiedBatches[inclusion_data] = false;
        }
    }
}
