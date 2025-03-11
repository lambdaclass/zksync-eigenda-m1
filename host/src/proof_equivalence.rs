use ark_bn254::{Fq, G1Affine};
use common::serializable_g1::SerializableG1;
use proof_equivalence_methods::PROOF_EQUIVALENCE_GUEST_ELF;
use risc0_zkvm::{default_prover, ExecutorEnv, ProveInfo, ProverOpts, VerifierContext};
use anyhow::Context;
use rust_kzg_bn254::{blob::Blob, kzg::Kzg};
//use rust_kzg_bn254_prover::srs::SRS;
//use rust_kzg_bn254_primitives::blob::Blob;
//use rust_kzg_bn254_prover::kzg::KZG;

use crate::verify_blob::G1Point;

pub async fn run_proof_equivalence(
    commitment: G1Point,
    data: Vec<u8>,
) -> anyhow::Result<ProveInfo> {
    let kzg_handle = tokio::task::spawn_blocking(move || {
        Kzg::setup("resources/g1.point", "", "resources/g2.point.powerOf2", 268435456, 2097152 / 32, "".to_string())
    });

    let mut kzg = kzg_handle
            .await
            .map_err(|e| anyhow::anyhow!("kzg error"))??;

    let blob = Blob::from_bytes_and_pad(&data);

    kzg.calculate_roots_of_unity(blob.len() as u64);

    let real_commitment = kzg.blob_to_kzg_commitment(&blob, rust_kzg_bn254::polynomial::PolynomialFormat::InCoefficientForm)?;

    let x: [u8;32] = commitment.x.to_be_bytes();
    let y: [u8;32] = commitment.y.to_be_bytes();
    
    let x_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&x));
    let y_fq =  Fq::from(num_bigint::BigUint::from_bytes_be(&y));
    
    let commitment = G1Affine::new(x_fq, y_fq);

    if commitment != real_commitment {
        return Err(anyhow::anyhow!("Commitments mismatched, given commitment: {:?}, real commitment: {:?}", commitment, real_commitment))
    }

    let proof = kzg.compute_kzg_proof_with_roots_of_unity(&blob.to_polynomial(rust_kzg_bn254::polynomial::PolynomialFormat::InCoefficientForm)?, 0)?;

    kzg.verify_kzg_proof(commitment, proof, blob.to_polynomial(rust_kzg_bn254::polynomial::PolynomialFormat::InCoefficientForm)?.to_vec()[0], *kzg.get_nth_root_of_unity(0).unwrap());
    
    let session_info = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let env = ExecutorEnv::builder()
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
    /*let blob = Blob::from_raw_data(&data);

    let mut kzg = KZG::new();

    kzg.calculate_and_store_roots_of_unity(blob.len().try_into()?)?;

    let x: [u8;32] = commitment.x.to_be_bytes();
    let y: [u8;32] = commitment.y.to_be_bytes();
    
    let x_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&x));
    let y_fq =  Fq::from(num_bigint::BigUint::from_bytes_be(&y));
    
    let commitment = G1Affine::new(x_fq, y_fq);
    let real_commitment = kzg.commit_coeff_form(&blob.to_polynomial_coeff_form(), &srs)?;

    if commitment != real_commitment {
        return Err(anyhow::anyhow!("Commitments mismatched, given commitment: {:?}, real commitment: {:?}", commitment, real_commitment))
    }
    
    let eval_commitment = kzg.commit_eval_form(&blob.to_polynomial_eval_form(), &srs)?;

    let proof = kzg.compute_blob_proof(&blob, &eval_commitment, &srs)?;

    let serializable_eval = SerializableG1{g1: eval_commitment};
    let serializable_proof = SerializableG1{g1: proof};
    let session_info = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let env = ExecutorEnv::builder()
            .write(&data)?
            .write(&serializable_eval)?
            .write(&serializable_proof)?
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

    Ok(session_info)*/



}
