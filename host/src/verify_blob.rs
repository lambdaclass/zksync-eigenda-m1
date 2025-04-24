use alloy_primitives::U256;
use alloy_sol_types::sol;
use anyhow::anyhow;
use common::blob_info::G1Commitment;
use ethabi::{ParamType, Token};

use crate::utils::{
    extract_array, extract_bytes, extract_fixed_bytes, extract_tuple, extract_uint32, extract_uint8,
};

sol! {
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
    
    /// VerifyBlobV1 function signature.
    /// This must match the signature in the guest.
    interface IVerifyBlob {
        function verifyDACertV2(BatchHeaderV2 calldata batchHeader,
            BlobInclusionInfo calldata blobInclusionInfo,
            NonSignerStakesAndSignature calldata nonSignerStakesAndSignature,
            bytes memory signedQuorumNumbers)
        external view returns (bool);
    }
}
