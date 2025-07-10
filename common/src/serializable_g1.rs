use ark_bn254::G1Affine;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub struct SerializableG1 {
    pub g1: G1Affine,
}

impl Serialize for SerializableG1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut compressed_bytes = Vec::new();
        self.g1
            .serialize_compressed(&mut compressed_bytes)
            .map_err(|e| {
                serde::ser::Error::custom(format!("Failed to serialize G1Affine: {:?}", e))
            })?;
        let mut tup = serializer.serialize_tuple(1)?;
        tup.serialize_element(&compressed_bytes)?;
        tup.end()
    }
}

impl<'de> Deserialize<'de> for SerializableG1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        let g1 = G1Affine::deserialize_compressed(&bytes[..]).map_err(|e| {
            serde::de::Error::custom(format!("Failed to deserialize G1Affine: {:?}", e))
        })?;
        Ok(SerializableG1 { g1 })
    }
}
