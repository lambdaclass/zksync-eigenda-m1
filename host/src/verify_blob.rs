use alloy_primitives::U256;
use alloy_sol_types::sol;
use anyhow::anyhow;
use ethabi::{ParamType, Token};
use common::blob_info::G1Commitment;

use crate::utils::{
    extract_array, extract_bytes, extract_fixed_bytes, extract_tuple, extract_uint32, extract_uint8,
};

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

impl From<G1Point> for G1Commitment {
    fn from(point: G1Point) -> Self {
        G1Commitment {
            x: point.x.to_be_bytes_vec(),
            y: point.y.to_be_bytes_vec(),
        }
    }
}

impl From<QuorumBlobParam> for common::blob_info::BlobQuorumParam {
    fn from(param: QuorumBlobParam) -> Self {
        common::blob_info::BlobQuorumParam {
            quorum_number: param.quorumNumber as u32,
            adversary_threshold_percentage: param.adversaryThresholdPercentage as u32,
            confirmation_threshold_percentage: param.confirmationThresholdPercentage as u32,
            chunk_length: param.chunkLength,
        }
    }
}

impl From<BlobHeader> for common::blob_info::BlobHeader {
    fn from(header: BlobHeader) -> Self {
        common::blob_info::BlobHeader {
            commitment: header.commitment.into(),
            data_length: header.dataLength,
            blob_quorum_params: header
                .quorumBlobParams
                .iter()
                .map(|param| common::blob_info::BlobQuorumParam::from(param.clone()))
                .collect(),
        }
    }
}

impl From<BatchHeader> for common::blob_info::BatchHeader {
    fn from(header: BatchHeader) -> Self {
        common::blob_info::BatchHeader {
            batch_root: header.blobHeadersRoot.to_vec(),
            quorum_numbers: header.quorumNumbers.to_vec(),
            quorum_signed_percentages: header.signedStakeForQuorums.to_vec(),
            reference_block_number: header.referenceBlockNumber,
        }
    }
}

impl From<BatchMetadata> for common::blob_info::BatchMetadata {
    fn from(metadata: BatchMetadata) -> Self {
        common::blob_info::BatchMetadata {
            batch_header: common::blob_info::BatchHeader::from(metadata.batchHeader),
            signatory_record_hash: metadata.signatoryRecordHash.to_vec(),
            confirmation_block_number: metadata.confirmationBlockNumber,
            fee: vec![],
            batch_header_hash: vec![],
        }
    }
}

impl From<BlobVerificationProof> for common::blob_info::BlobVerificationProof {
    fn from(proof: BlobVerificationProof) -> Self {
        common::blob_info::BlobVerificationProof {
            batch_id: proof.batchId,
            blob_index: proof.blobIndex,
            batch_medatada: common::blob_info::BatchMetadata::from(proof.batchMetadata),
            inclusion_proof: proof.inclusionProof.to_vec(),
            quorum_indexes: proof.quorumIndices.to_vec(),
        }
    }
}

pub fn decode_blob_info(
    inclusion_data: Vec<u8>,
) -> Result<(BlobHeader, BlobVerificationProof, Vec<u8>), anyhow::Error> {
    let param_types = vec![ParamType::Tuple(vec![
        // BlobHeader
        ParamType::Tuple(vec![
            ParamType::Tuple(vec![ParamType::Uint(256), ParamType::Uint(256)]), // G1Commitment
            ParamType::Uint(32),                                                // data_length
            ParamType::Array(Box::new(ParamType::Tuple(vec![
                ParamType::Uint(32),
                ParamType::Uint(32),
                ParamType::Uint(32),
                ParamType::Uint(32),
            ]))), // BlobQuorumParam
        ]),
        // BlobVerificationProof
        ParamType::Tuple(vec![
            ParamType::Uint(32), // batch_id
            ParamType::Uint(32), // blob_index
            ParamType::Tuple(vec![
                ParamType::Tuple(vec![
                    ParamType::FixedBytes(32),
                    ParamType::Bytes,
                    ParamType::Bytes,
                    ParamType::Uint(32),
                ]), // BatchHeader
                ParamType::FixedBytes(32), // signatory_record_hash
                ParamType::Uint(32),       // confirmation_block_number
                ParamType::Bytes,          // batch_header_hash
                ParamType::Bytes,          // fee
            ]), // BatchMetadata
            ParamType::Bytes,    // inclusion_proof
            ParamType::Bytes,    // quorum_indexes
        ]),
    ])];

    let decoded = ethabi::decode(&param_types, &inclusion_data)?;
    let blob_info = extract_tuple(&decoded[0])?;

    // Extract BlobHeader
    let blob_header_tokens = extract_tuple(&blob_info[0])?;
    let commitment_tokens = extract_tuple(&blob_header_tokens[0])?;

    let x = commitment_tokens[0].clone().into_uint().ok_or(anyhow!("Incorrect commitment"))?;
    let y = commitment_tokens[1].clone().into_uint().ok_or(anyhow!("Incorrect commitment"))?;

    let mut x_bytes = [0u8; 32];
    let mut y_bytes = [0u8; 32];
    x.to_big_endian(&mut x_bytes);
    y.to_big_endian(&mut y_bytes);

    let data_length = extract_uint32(&blob_header_tokens[1])?;
    let blob_quorum_params_tokens = extract_array(&blob_header_tokens[2])?;

    let blob_quorum_params: Vec<QuorumBlobParam> = blob_quorum_params_tokens
        .iter()
        .map(|param| {
            let tuple = extract_tuple(param)?;
            Ok(QuorumBlobParam {
                quorumNumber: extract_uint8(&tuple[0])?,
                adversaryThresholdPercentage: extract_uint8(&tuple[1])?,
                confirmationThresholdPercentage: extract_uint8(&tuple[2])?,
                chunkLength: extract_uint32(&tuple[3])?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let blob_header = BlobHeader {
        commitment: G1Point {
            x: U256::from_be_bytes(x_bytes),
            y: U256::from_be_bytes(y_bytes),
        },
        dataLength: data_length,
        quorumBlobParams: blob_quorum_params,
    };

    // Extract BlobVerificationProof
    let blob_verification_tokens = extract_tuple(&blob_info[1])?;

    let batch_id = extract_uint32(&blob_verification_tokens[0])?;
    let blob_index = extract_uint32(&blob_verification_tokens[1])?;

    let batch_metadata_tokens = extract_tuple(&blob_verification_tokens[2])?;
    let batch_header_tokens = extract_tuple(&batch_metadata_tokens[0])?;

    let batch_header = BatchHeader {
        blobHeadersRoot: extract_fixed_bytes::<32>(&batch_header_tokens[0])?,
        quorumNumbers: extract_bytes(&batch_header_tokens[1])?,
        signedStakeForQuorums: extract_bytes(&batch_header_tokens[2])?,
        referenceBlockNumber: extract_uint32(&batch_header_tokens[3])?,
    };

    let batch_metadata = BatchMetadata {
        batchHeader: batch_header,
        signatoryRecordHash: extract_fixed_bytes::<32>(&batch_metadata_tokens[1])?,
        confirmationBlockNumber: extract_uint32(&batch_metadata_tokens[2])?,
    };

    let batch_header_hash: Vec<u8> = match batch_metadata_tokens[3].clone() {
        Token::Bytes(bytes) => Ok(bytes),
        _ => Err(anyhow::anyhow!("Fixed bytes not found")),
    }?;

    let blob_verification_proof = BlobVerificationProof {
        batchId: batch_id,
        blobIndex: blob_index,
        batchMetadata: batch_metadata,
        inclusionProof: extract_bytes(&blob_verification_tokens[3])?,
        quorumIndices: extract_bytes(&blob_verification_tokens[4])?,
    };

    Ok((blob_header, blob_verification_proof, batch_header_hash))
}
