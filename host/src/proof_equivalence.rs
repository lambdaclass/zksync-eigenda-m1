use ark_bn254::{Fq, G1Affine};
use proof_equivalence_methods::PROOF_EQUIVALENCE_GUEST_ELF;
use risc0_zkvm::{default_prover, ExecutorEnv, ProveInfo, ProverOpts, VerifierContext};
use anyhow::Context;
use rust_kzg_bn254_prover::srs::SRS;
use serde::{Serialize, Serializer};
use serde::ser::SerializeTuple;
use rust_kzg_bn254_primitives::blob::Blob;
use rust_kzg_bn254_prover::kzg::KZG;

use crate::verify_blob::G1Point;

pub struct SerializableG1 { //TODO: Move to common
    pub g1: G1Affine
}

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

pub async fn run_proof_equivalence(
    srs: &SRS,
    commitment: G1Point,
    data: Vec<u8>,
) -> anyhow::Result<ProveInfo> {
    //let content = std::fs::read_to_string("sample_data.txt").unwrap(); // todo add real blob

    // Blob data BEFORE padding
    /*let data: Vec<u8> = content
        .split(',')
        .map(|s| s.trim()) // Remove any leading/trailing spaces
        .filter(|s| !s.is_empty()) // Ignore empty strings
        .map(|s| s.parse::<u8>().expect("Invalid number")) // Parse as u8
        .collect();*/

    let blob = Blob::from_raw_data(&data);

    let mut kzg = KZG::new();

    kzg.calculate_and_store_roots_of_unity(blob.len().try_into().unwrap()).unwrap();

    let x: [u8;32] = commitment.x.to_be_bytes();
    let y: [u8;32] = commitment.y.to_be_bytes();

    //let x: Vec<u8> = vec![20, 153, 170, 133, 150, 17, 219, 215, 90, 29, 61, 41, 183, 105, 4, 139, 14, 161, 160, 7, 49, 89, 23, 57, 49, 52, 16, 175, 112, 57, 19, 50];
    //let y: Vec<u8> =  vec![47, 50, 235, 25, 170, 240, 84, 149, 189, 33, 211, 171, 1, 250, 141, 124, 116, 49, 37, 211, 193, 146, 250, 255, 63, 16, 117, 92, 28, 237, 120, 166];
    
    let x_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&x));
    println!("x done");
    let y_fq =  Fq::from(num_bigint::BigUint::from_bytes_be(&y));
    println!("y done");
    
    let commitment = G1Affine::new(x_fq, y_fq);
    println!("com done");
    let real_commitment = kzg.commit_coeff_form(&blob.to_polynomial_coeff_form(), &srs).unwrap();
    println!("real done");
    
    assert!(commitment == real_commitment);

    let eval_commitment = kzg.commit_eval_form(&blob.to_polynomial_eval_form(), &srs).unwrap();

    println!("eval done");

    let proof = kzg.compute_blob_proof(&blob, &eval_commitment, &srs).unwrap();
    println!("proof done");

    let serializable_eval = SerializableG1{g1: eval_commitment};
    let serializable_proof = SerializableG1{g1: proof};
    let session_info = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let env = ExecutorEnv::builder()
            .write(&data).unwrap()
            .write(&serializable_eval).unwrap()
            .write(&serializable_proof).unwrap()
            .build()?;
        let exec = default_prover();
        exec.prove_with_ctx(
            env,
            &VerifierContext::default(),
            PROOF_EQUIVALENCE_GUEST_ELF,
            &ProverOpts::groth16(),
        )
        .context("failed to run executor")
    }).await??;
    println!("Finished run");

    Ok(session_info)

}
