use serde::{Deserialize, Serialize};

/// Internal of BlobInfo
/// Contains the KZG Commitment
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct G1Commitment {
    pub x: Vec<u8>,
    pub y: Vec<u8>,
}

/// Internal of BlobInfo
/// Contains data related to the blob quorums  
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct BlobQuorumParam {
    /// The ID of the quorum.
    pub quorum_number: u32,
    /// The max percentage of stake within the quorum that can be held by or delegated to adversarial operators.
    pub adversary_threshold_percentage: u32,
    /// The min percentage of stake that must attest in order to consider the dispersal successful.    
    pub confirmation_threshold_percentage: u32,
    /// The length of each chunk in bn254 field elements (32 bytes each).
    pub chunk_length: u32,
}

/// Internal of BlobInfo
/// Contains the blob header data
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct BlobHeader {
    pub commitment: G1Commitment,
    pub data_length: u32,
    pub blob_quorum_params: Vec<BlobQuorumParam>,
}

/// Internal of BlobInfo
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct BatchHeader {
    pub batch_root: Vec<u8>,
    pub quorum_numbers: Vec<u8>,
    pub quorum_signed_percentages: Vec<u8>,
    pub reference_block_number: u32,
}

/// Internal of BlobInfo
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct BatchMetadata {
    pub batch_header: BatchHeader,
    pub signatory_record_hash: Vec<u8>,
    pub fee: Vec<u8>,
    pub confirmation_block_number: u32,
    pub batch_header_hash: Vec<u8>,
}

/// Internal of BlobInfo
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct BlobVerificationProof {
    pub batch_id: u32,
    pub blob_index: u32,
    pub batch_medatada: BatchMetadata,
    pub inclusion_proof: Vec<u8>,
    pub quorum_indexes: Vec<u8>,
}

/// Data returned by the disperser when a blob is dispersed
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct BlobInfo {
    pub blob_header: BlobHeader,
    pub blob_verification_proof: BlobVerificationProof,
}
