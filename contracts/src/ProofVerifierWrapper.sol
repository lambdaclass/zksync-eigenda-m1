// ViewBypass.sol
pragma solidity ^0.8.20;

interface IRiscZeroVerifier {
    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest) external view;
}

contract ProofVerifierWrapper {
    IRiscZeroVerifier public risc0verifier;

    event ProofVerified();

    constructor(address _risc0verifier) {
        risc0verifier = IRiscZeroVerifier(_risc0verifier);
    }

    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest) public {
        risc0verifier.verify(seal, imageId, journalDigest);
        emit ProofVerified(); // This makes the function state-changing
    }
}
