use alloy_primitives::{Bytes, FixedBytes, Uint, U256};
use alloy_sol_types::sol;
use ark_bn254::{G1Affine, G2Affine};
use ark_ff::PrimeField;
use ark_ff::BigInteger;
use rust_eigenda_v2_cert::BlobCommitments as BlobCommitmentsClient;
use rust_eigenda_v2_cert::BlobCertificate as BlobCertificateClient;
use rust_eigenda_v2_cert::BlobHeader as BlobHeaderClient;
use rust_eigenda_v2_cert::BatchHeaderV2 as BatchHeaderV2Client;
use rust_eigenda_v2_cert::BlobInclusionInfo as BlobInclusionInfoClient;
use rust_eigenda_v2_cert::NonSignerStakesAndSignature as NonSignerStakesAndSignatureClient;

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

fn g2_contract_point_from_g2_affine(g2_affine: &G2Affine) -> G2Point {
    let x = g2_affine.x;
    let y = g2_affine.y;
    G2Point {
        X: [
            Uint::from_be_bytes::<32>(x.c1.into_bigint().to_bytes_be().try_into().unwrap()),
            Uint::from_be_bytes::<32>(x.c0.into_bigint().to_bytes_be().try_into().unwrap()),
        ],
        Y: [
            Uint::from_be_bytes::<32>(y.c1.into_bigint().to_bytes_be().try_into().unwrap()),
            Uint::from_be_bytes::<32>(y.c0.into_bigint().to_bytes_be().try_into().unwrap()),
        ],
    }
}

fn g1_contract_point_from_g1_affine(g1_affine: &G1Affine) -> G1Point {
    let x = g1_affine.x;
    let y = g1_affine.y;
    G1Point {
        X: Uint::from_be_bytes::<32>(x.into_bigint().to_bytes_be().try_into().unwrap()),
        Y: Uint::from_be_bytes::<32>(y.into_bigint().to_bytes_be().try_into().unwrap()),
    }
}

impl From<BlobCommitmentsClient> for BlobCommitment {
    fn from(value: BlobCommitmentsClient) -> Self {
        Self {
            lengthCommitment: g2_contract_point_from_g2_affine(&value.length_commitment),
            lengthProof: g2_contract_point_from_g2_affine(&value.length_proof),
            length: value.length,
            commitment: g1_contract_point_from_g1_affine(&value.commitment),
        }
    }
}

impl From<BlobHeaderClient> for BlobHeaderV2 {
    fn from(value: BlobHeaderClient) -> Self {
        Self {
            version: value.version,
            quorumNumbers: value.quorum_numbers.clone().into(),
            commitment: value.commitment.clone().into(),
            paymentHeaderHash: alloy_primitives::FixedBytes(value.payment_header_hash),
        }
    }
}

impl From<BlobCertificateClient> for BlobCertificate {
    fn from(value: BlobCertificateClient) -> Self {
        Self {
            blobHeader: value.blob_header.into(),
            signature: value.signature.into(),
            relayKeys: value.relay_keys,
        }
    }
}

impl From<BlobInclusionInfoClient> for BlobInclusionInfo {
    fn from(value: BlobInclusionInfoClient) -> Self {
        BlobInclusionInfo {
            blobCertificate: value.blob_certificate.into(),
            blobIndex: value.blob_index,
            inclusionProof: value.inclusion_proof.clone().into(),
        }
    }
}

impl From<BatchHeaderV2Client> for BatchHeaderV2 {
    fn from(value: BatchHeaderV2Client) -> Self {
        Self {
            batchRoot: alloy_primitives::FixedBytes(value.batch_root),
            referenceBlockNumber: value.reference_block_number,
        }
    }
}

impl From<NonSignerStakesAndSignatureClient> for NonSignerStakesAndSignature {
    fn from(value: NonSignerStakesAndSignatureClient) -> Self {
        Self {
            nonSignerQuorumBitmapIndices: value.non_signer_quorum_bitmap_indices.clone(),
            nonSignerPubkeys: value
                .non_signer_pubkeys
                .iter()
                .map(g1_contract_point_from_g1_affine)
                .collect(),
                quorumApks: value
                .quorum_apks
                .iter()
                .map(g1_contract_point_from_g1_affine)
                .collect(),
                apkG2: g2_contract_point_from_g2_affine(&value.apk_g2),
            sigma: g1_contract_point_from_g1_affine(&value.sigma),
            quorumApkIndices: value.quorum_apk_indices.clone(),
            totalStakeIndices: value.total_stake_indices.clone(),
            nonSignerStakeIndices: value.non_signer_stake_indices.clone(),
        }
    }
}
