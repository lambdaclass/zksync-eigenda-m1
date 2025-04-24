// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
struct G1Point {
        uint256 X;
        uint256 Y;
    }

    struct G2Point {
        uint256[2] X;
        uint256[2] Y;
    }

    struct VersionedBlobParams {
        uint32 maxNumOperators;
        uint32 numChunks;
        uint8 codingRate;
    }
    
    struct SecurityThresholds {
        uint8 confirmationThreshold;
        uint8 adversaryThreshold;
    }
    
    struct RelayInfo {
        address relayAddress;
        string relayURL;
    }
    
    struct DisperserInfo {
        address disperserAddress;
    }
    
    struct BlobInclusionInfo {
        BlobCertificate blobCertificate;
        uint32 blobIndex;
        bytes inclusionProof;
    }
    
    struct BlobCertificate {
        BlobHeaderV2 blobHeader;
        bytes signature;
        uint32[] relayKeys;
    }
    
    struct BlobHeaderV2 {
        uint16 version;
        bytes quorumNumbers;
        BlobCommitment commitment;
        bytes32 paymentHeaderHash;
    }
    
    struct BlobCommitment {
        G1Point commitment;
        G2Point lengthCommitment;
        G2Point lengthProof;
        uint32 length;
    }
    
    struct BatchHeaderV2 {
        bytes32 batchRoot;
        uint32 referenceBlockNumber;
    }

    struct NonSignerStakesAndSignature {
        uint32[] nonSignerQuorumBitmapIndices;
        G1Point[] nonSignerPubkeys;
        G1Point[] quorumApks;
        G2Point apkG2;
        G1Point sigma;
        uint32[] quorumApkIndices;
        uint32[] totalStakeIndices;
        uint32[][] nonSignerStakeIndices;
    }
        

interface IBlobVerifier {
    function verifyDACertV2(
        BatchHeaderV2 calldata batchHeader,
        BlobInclusionInfo calldata blobInclusionInfo,
        NonSignerStakesAndSignature calldata nonSignerStakesAndSignature,
        bytes memory signedQuorumNumbers
    ) external view;
}

// This is necessary because risc0 steel does not support proving for functions that do not return a vaule
contract BlobVerifierWrapper {
    address public blobVerifier;

    constructor(address _blobVerifier) {
        blobVerifier = _blobVerifier;
    }

    // Wrapps the verifyBlobV1 function of the eigenDA blobVerifier contract, since we need it to return a value
    function verifyDACertV2(
        BatchHeaderV2 calldata batchHeader,
        BlobInclusionInfo calldata blobInclusionInfo,
        NonSignerStakesAndSignature calldata nonSignerStakesAndSignature,
        bytes memory signedQuorumNumbers
    ) external view returns (bool) {
        IBlobVerifier(blobVerifier).verifyDACertV2(batchHeader, blobInclusionInfo,nonSignerStakesAndSignature,signedQuorumNumbers);
        return true;
    }
}
