use ark_bn254::G1Affine;
use ark_ff::Fp;
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub struct SerializableG1 {
    pub g1: G1Affine,
}
use std::str::FromStr;

impl Serialize for SerializableG1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let x = format!("{:?}", self.g1.x);
        let y = format!("{:?}", self.g1.y);
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&x)?;
        tup.serialize_element(&y)?;
        tup.end()
    }
}

impl<'de> Deserialize<'de> for SerializableG1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y): (String, String) = Deserialize::deserialize(deserializer)?;
        let g1 = G1Affine::new_unchecked(
            Fp::from_str(&x).map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?,
            Fp::from_str(&y).map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?,
        );
        Ok(SerializableG1 { g1 })
    }
}
