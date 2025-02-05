use alloy_primitives::{Bytes, FixedBytes, U256};
use alloy_sol_types::sol;

use crate::blob_info::BlobQuorumParam;

sol! {
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

    /// VerifyBlobV1 function signature.
    /// This must match the signature in the guest.
    interface IVerifyBlob {
        function verifyBlobV1(BlobHeader calldata blobHeader, BlobVerificationProof calldata blobVerificationProof) external view returns (bool);
    }
}

impl From<BlobQuorumParam> for QuorumBlobParam {
    fn from(param: BlobQuorumParam) -> Self {
        QuorumBlobParam {
            quorumNumber: param.quorum_number as u8,
            adversaryThresholdPercentage: param.adversary_threshold_percentage as u8,
            confirmationThresholdPercentage: param.confirmation_threshold_percentage as u8,
            chunkLength: param.chunk_length,
        }
    }
}

impl From<crate::blob_info::BlobHeader> for BlobHeader {
    fn from(blob_header: crate::blob_info::BlobHeader) -> Self {
        let x: [u8;32] = blob_header.commitment.x.try_into().expect("slice with incorrect length");
        let y: [u8;32]  = blob_header.commitment.y.try_into().expect("slice with incorrect length");
        BlobHeader {
            commitment: G1Point {
                x: U256::from_be_bytes(x),
                y: U256::from_be_bytes(y),
            },
            dataLength: blob_header.data_length,
            quorumBlobParams: blob_header.blob_quorum_params.iter().map(|param| QuorumBlobParam::from(param.clone())).collect(),
        }
    }
}

impl From<crate::blob_info::G1Commitment > for G1Point {
    fn from(commitment: crate::blob_info::G1Commitment) -> Self {
        let x: [u8;32] = commitment.x.try_into().expect("slice with incorrect length");
        let y: [u8;32]  = commitment.y.try_into().expect("slice with incorrect length");
        G1Point {
            x: U256::from_be_bytes(x),
            y: U256::from_be_bytes(y),
        }
    }
}

impl From<crate::blob_info::BatchHeader> for BatchHeader {
    fn from(batch_header: crate::blob_info::BatchHeader) -> Self {
        let root: [u8;32] = batch_header.batch_root.try_into().expect("slice with incorrect length");
        BatchHeader {
            blobHeadersRoot: FixedBytes::from(root),
            quorumNumbers: Bytes::from(batch_header.quorum_numbers),
            signedStakeForQuorums: Bytes::from(batch_header.quorum_signed_percentages),
            referenceBlockNumber: batch_header.reference_block_number,
        }
    }
}

impl From<crate::blob_info::BatchMetadata> for BatchMetadata {
    fn from(batch_metadata: crate::blob_info::BatchMetadata) -> Self {
        let header: BatchHeader = BatchHeader::from(batch_metadata.batch_header);
        let signatory_record_hash: [u8;32] = batch_metadata.signatory_record_hash.try_into().expect("slice with incorrect length");
        BatchMetadata {
            batchHeader: header,
            signatoryRecordHash: FixedBytes::from(signatory_record_hash),
            confirmationBlockNumber: batch_metadata.confirmation_block_number,
        }
    }
}

impl From<crate::blob_info::BlobVerificationProof> for BlobVerificationProof {
    fn from(blob_verification_proof: crate::blob_info::BlobVerificationProof) -> Self {
        let metadata: BatchMetadata = BatchMetadata::from(blob_verification_proof.batch_medatada);
        BlobVerificationProof {
            batchId: blob_verification_proof.batch_id,
            blobIndex: blob_verification_proof.blob_index,
            batchMetadata: metadata,
            inclusionProof: Bytes::from(blob_verification_proof.inclusion_proof),
            quorumIndices: Bytes::from(blob_verification_proof.quorum_indexes),
        }
    }
}
