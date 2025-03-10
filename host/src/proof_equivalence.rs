use ark_bn254::{Fq, G1Affine};
use common::serializable_g1::SerializableG1;
use proof_equivalence_methods::PROOF_EQUIVALENCE_GUEST_ELF;
use risc0_zkvm::{default_prover, ExecutorEnv, ProveInfo, ProverOpts, VerifierContext};
use anyhow::Context;
use rust_kzg_bn254_prover::srs::SRS;
use rust_kzg_bn254_primitives::blob::Blob;
use rust_kzg_bn254_prover::kzg::KZG;

use crate::verify_blob::G1Point;

pub async fn run_proof_equivalence(
    srs: &SRS,
    commitment: G1Point,
    data: Vec<u8>,
) -> anyhow::Result<ProveInfo> {
    let blob = Blob::from_raw_data(&data);

    let mut kzg = KZG::new();

    kzg.calculate_and_store_roots_of_unity(blob.len().try_into().unwrap()).unwrap();

    let x: [u8;32] = commitment.x.to_be_bytes();
    let y: [u8;32] = commitment.y.to_be_bytes();
    
    let x_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&x));
    let y_fq =  Fq::from(num_bigint::BigUint::from_bytes_be(&y));
    
    let commitment = G1Affine::new(x_fq, y_fq);
    let real_commitment = kzg.commit_coeff_form(&blob.to_polynomial_coeff_form(), &srs).unwrap();

    if commitment != real_commitment {
        return Err(anyhow::anyhow!("Commitments mismatched, given commitment: {:?}, real commitment: {:?}", commitment, real_commitment))
    }
    
    let eval_commitment = kzg.commit_eval_form(&blob.to_polynomial_eval_form(), &srs).unwrap();

    let proof = kzg.compute_blob_proof(&blob, &eval_commitment, &srs).unwrap();

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

    Ok(session_info)

}
