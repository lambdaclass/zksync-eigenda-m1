use ark_bn254::{Fq, G1Affine};
use rust_kzg_bn254_primitives::blob::Blob;
use rust_kzg_bn254_prover::{kzg::KZG, srs::SRS};
use rust_kzg_bn254_verifier::verify::verify_blob_kzg_proof;
use ark_ff::PrimeField;
fn main() {
    let content = std::fs::read_to_string("sample_data.txt").unwrap(); 

    /// Blob data BEFORE padding
    let data: Vec<u8> = content
        .split(',')
        .map(|s| s.trim()) // Remove any leading/trailing spaces
        .filter(|s| !s.is_empty()) // Ignore empty strings
        .map(|s| s.parse::<u8>().expect("Invalid number")) // Parse as u8
        .collect();

    let blob = Blob::from_raw_data(&data);

    let mut kzg = KZG::new();
    kzg.calculate_and_store_roots_of_unity(blob.len().try_into().unwrap()).unwrap();
    let srs = SRS::new("resources/g1.point", 268435456, 1024 * 1024 * 2 / 32).unwrap();
    
    let x: Vec<u8> = vec![20, 153, 170, 133, 150, 17, 219, 215, 90, 29, 61, 41, 183, 105, 4, 139, 14, 161, 160, 7, 49, 89, 23, 57, 49, 52, 16, 175, 112, 57, 19, 50];
    let y: Vec<u8> =  vec![47, 50, 235, 25, 170, 240, 84, 149, 189, 33, 211, 171, 1, 250, 141, 124, 116, 49, 37, 211, 193, 146, 250, 255, 63, 16, 117, 92, 28, 237, 120, 166];
    
    let x_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&x));
    let y_fq =  Fq::from(num_bigint::BigUint::from_bytes_be(&y));
    
    let commitment = G1Affine::new(x_fq, y_fq);
    let real_commitment = kzg.commit_coeff_form(&blob.to_polynomial_coeff_form(), &srs).unwrap();
    
    assert!(commitment == real_commitment);

    let eval_commitment = kzg.commit_eval_form(&blob.to_polynomial_eval_form(), &srs).unwrap();

    let proof = kzg.compute_blob_proof(&blob, &eval_commitment, &srs).unwrap();

    let verified = verify_blob_kzg_proof(&blob, &eval_commitment, &proof).unwrap();

    assert!(verified);

    println!(":)");
}
