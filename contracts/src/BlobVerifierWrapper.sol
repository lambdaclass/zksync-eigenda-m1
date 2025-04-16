// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

struct QuorumBlobParam {
    uint8 quorumNumber;
    uint8 adversaryThresholdPercentage;
    uint8 confirmationThresholdPercentage; 
    uint32 chunkLength; 
}

struct G1Point {
    uint256 x;
    uint256 y;
}

struct BlobHeader {
    G1Point commitment; 
    uint32 dataLength; 
    QuorumBlobParam[] quorumBlobParams; 
}

struct ReducedBatchHeader {
    bytes32 blobHeadersRoot;
    uint32 referenceBlockNumber;
}

struct BatchHeader {
    bytes32 blobHeadersRoot;
    bytes quorumNumbers; 
    bytes signedStakeForQuorums; 
    uint32 referenceBlockNumber;
}
    
struct BatchMetadata {
    BatchHeader batchHeader; 
    bytes32 signatoryRecordHash; 
    uint32 confirmationBlockNumber; 
}

struct BlobVerificationProof {
    uint32 batchId;
    uint32 blobIndex;
    BatchMetadata batchMetadata;
    bytes inclusionProof;
    bytes quorumIndices;
}

interface IBlobVerifier {
    function verifyBlobV1(
        BlobHeader calldata blobHeader,
        BlobVerificationProof calldata blobVerificationProof
    ) external view;
}

// This is necessary because risc0 steel does not support proving for functions that do not return a vaule
contract BlobVerifierWrapper {
    address public blobVerifier;

    constructor(address _blobVerifier) {
        blobVerifier = _blobVerifier;
    }

    // Wrapps the verifyBlobV1 function of the eigenDA blobVerifier contract, since we need it to return a value
    function verifyBlobV1(
        BlobHeader calldata blobHeader,
        BlobVerificationProof calldata blobVerificationProof
    ) external view returns (bool) {
        IBlobVerifier(blobVerifier).verifyBlobV1(blobHeader, blobVerificationProof);
        return true;
    }
}
