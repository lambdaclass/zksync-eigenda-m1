use alloy_primitives::{Bytes, FixedBytes};
use ethabi::Token;

pub fn extract_tuple(token: &Token) -> anyhow::Result<&Vec<Token>> {
    match token {
        Token::Tuple(inner) => Ok(inner),
        _ => Err(anyhow::anyhow!("Not a tuple")),
    }
}

pub fn extract_array(token: &Token) -> anyhow::Result<Vec<Token>> {
    match token {
        Token::Array(tokens) => Ok(tokens.clone()),
        _ => Err(anyhow::anyhow!("Not a uint")),
    }
}

pub fn extract_uint32(token: &Token) -> anyhow::Result<u32> {
    match token {
        Token::Uint(value) => Ok(value.as_u32()),
        _ => Err(anyhow::anyhow!("Not a uint")),
    }
}

pub fn extract_uint8(token: &Token) -> anyhow::Result<u8> {
    match token {
        Token::Uint(value) => Ok(value.as_u32() as u8),
        _ => Err(anyhow::anyhow!("Not a uint")),
    }
}

pub fn extract_fixed_bytes<const N: usize>(token: &Token) -> anyhow::Result<FixedBytes<32>> {
    match token {
        Token::FixedBytes(bytes) => Ok(FixedBytes::from_slice(bytes)),
        _ => Err(anyhow::anyhow!("Not fixed bytes")),
    }
}

pub fn extract_bytes(token: &Token) -> anyhow::Result<Bytes> {
    match token {
        Token::Bytes(bytes) => Ok(Bytes::from_iter(bytes)),
        _ => Err(anyhow::anyhow!("Not bytes")),
    }
}
