use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub hash: Vec<u8>,
    pub env: Vec<u8>
}
