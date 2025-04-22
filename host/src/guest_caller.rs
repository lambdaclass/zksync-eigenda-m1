use crate::verify_blob::{BlobHeader, BlobVerificationProof, IVerifyBlob};
use alloy_primitives::Address;
use alloy_sol_types::SolCall;
use anyhow::Context;
use ark_bn254::{Fq, G1Affine};
use common::serializable_g1::SerializableG1;
use methods::GUEST_ELF;
use risc0_steel::{ethereum::EthEvmEnv, Contract};
use risc0_zkvm::ProveInfo;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use rust_kzg_bn254_primitives::blob::Blob;
use rust_kzg_bn254_primitives::helpers::compute_challenge;
use rust_kzg_bn254_prover::kzg::KZG;
use rust_kzg_bn254_prover::srs::SRS;
use url::Url;

pub async fn run_guest(
    blob_header: BlobHeader,
    blob_verification_proof: BlobVerificationProof,
    srs: &SRS,
    data: Vec<u8>,
    rpc_url: Url,
    blob_verifier_wrapper_addr: Address,
) -> anyhow::Result<ProveInfo> {
    let call = IVerifyBlob::verifyBlobV1Call {
        blobHeader: blob_header.clone(),
        blobVerificationProof: blob_verification_proof.clone(),
    };

    // Create an EVM environment from an RPC endpoint defaulting to the latest block.
    let mut env = EthEvmEnv::builder().rpc(rpc_url.clone()).build().await?;

    // Preflight the call to prepare the input that is required to execute the function in
    // the guest without RPC access. It also returns the result of the call.
    // Risc0 steel creates an ethereum VM using revm, where it simulates the call to VerifyBlobV1.
    // So we need to make this preflight call to populate the VM environment with the current state of the chain
    let mut contract = Contract::preflight(blob_verifier_wrapper_addr, &mut env);
    let returns = contract
        .call_builder(&call)
        .call()
        .await?;
    println!(
        "Call {} Function on {:#} returns: {}",
        IVerifyBlob::verifyBlobV1Call::SIGNATURE,
        blob_verifier_wrapper_addr,
        returns._0
    );

    // Finally, construct the input from the environment.
    let input = env.into_input().await?;

    // aka EigenDACert
    let blob_info = common::blob_info::BlobInfo {
        blob_header: blob_header.clone().into(),
        blob_verification_proof: blob_verification_proof.clone().into(),
    };

    let blob = Blob::from_raw_data(&data);

    let mut kzg = KZG::new();

    kzg.calculate_and_store_roots_of_unity(blob.len().try_into()?)?;

    let x: [u8; 32] = blob_header.commitment.x.to_be_bytes();
    let y: [u8; 32] = blob_header.commitment.y.to_be_bytes();

    let x_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&x));
    let y_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&y));

    let cert_commitment = G1Affine::new(x_fq, y_fq);

    // Calculate the polynomial in evaluation form
    let poly_coeff = blob.to_polynomial_coeff_form();
    let poly_eval = poly_coeff.to_eval_form()?;

    let evaluation_challenge = compute_challenge(&blob, &cert_commitment)?;

    // Compute the proof that the commitment corresponds to the given blob
    let proof = kzg.compute_proof(&poly_eval,&evaluation_challenge,&srs)?;

    let serializable_proof = SerializableG1 { g1: proof };

    println!("Running the guest with the constructed input...");
    let session_info = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let env = ExecutorEnv::builder()
            .write(&input)?
            .write(&blob_info)?
            .write(&data)?
            .write(&serializable_proof)?
            .write(&blob_verifier_wrapper_addr)?
            .build()?;
        let exec = default_prover();
        exec.prove_with_ctx(
            env,
            &VerifierContext::default(),
            GUEST_ELF,
            &ProverOpts::groth16(),
        )
        .context("failed to run executor")
    })
    .await??;

    Ok(session_info)
}
