use rand::Rand;
use rust_kzg_bn254_primitives::helpers::to_fr_array;
use zksync_kzg::{compute_commitment, compute_proof_poly, verify_proof_poly, KzgSettings};

use boojum::pairing::{bls12_381::fr::{Fr, FrRepr}, ff::{Field, PrimeField}};

const FIELD_ELEMENTS_PER_BLOB: usize = 4096;
const SETUP_JSON: &str = "src/trusted_setup.json";

fn u8_repr_to_fr(bytes: &[u8]) -> Fr {
    assert_eq!(bytes.len(), 32);
    let mut ret = [0u64; 4];

    for (i, chunk) in bytes.chunks(8).enumerate() {
        let mut repr = [0u8; 8];
        repr.copy_from_slice(chunk);
        ret[3 - i] = u64::from_be_bytes(repr);
    }

    Fr::from_repr(FrRepr(ret)).unwrap()
}

fn main() {
    let mut rng = rand::thread_rng();

    let data: Vec<u8> = {
        vec![0]
    };

    let blob = to_fr_array(&data);

    // let mut blob = vec![];
    // for a in data.chunks(32) {
    //     let fr_r = u8_repr_to_fr(a);
    //     blob.push(fr_r);
    // }

    let settings = KzgSettings::new(SETUP_JSON);

    let commitment = compute_commitment(&settings, &blob);
    // let proof = compute_proof_poly(&settings, &blob, &commitment);
    // assert!(verify_proof_poly(&settings, &blob, &commitment, &proof));
    println!(":)");
}
