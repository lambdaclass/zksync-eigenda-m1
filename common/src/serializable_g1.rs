use ark_bn254::G1Affine;
use serde::{Serialize, Serializer,Deserialize, Deserializer};
use serde::ser::SerializeTuple;
use ark_ff::Fp;

pub struct SerializableG1 {
    pub g1: G1Affine
}
use std::str::FromStr;


impl Serialize for SerializableG1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let x = format!("{:?}",self.g1.x);
        let y = format!("{:?}",self.g1.y);
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&x).unwrap();
        tup.serialize_element(&y).unwrap();
        tup.end()
    }
}

impl<'de> Deserialize<'de> for SerializableG1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y): (String, String) = Deserialize::deserialize(deserializer)?;
        let g1 = G1Affine::new_unchecked(Fp::from_str(&x).unwrap(), Fp::from_str(&y).unwrap());
        Ok(SerializableG1{g1})
    }
}
