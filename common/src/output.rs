use serde::{Deserialize, Serialize};

use crate::blob_info::BlobInfo;

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub hash: Vec<u8>,
    pub env_commitment: Vec<u8>,
    pub blob_info: BlobInfo,
    pub proof: Vec<u8>,
}
