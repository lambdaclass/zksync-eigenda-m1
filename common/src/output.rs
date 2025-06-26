use ethabi::Token;

pub struct Output {
    pub hash: Vec<u8>,
    pub env_commitment: Vec<u8>,
    pub inclusion_data: Vec<u8>,
    pub proof: Vec<u8>,
}


impl Output {
    pub fn abi_encode(self) -> Vec<u8> {
        ethabi::encode(&[
            Token::FixedBytes(self.hash),
            Token::Bytes(self.env_commitment),
            Token::Bytes(self.inclusion_data),
            Token::Bytes(self.proof)
        ])
    }
}
