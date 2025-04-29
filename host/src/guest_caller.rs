use alloy_primitives::Address;
use alloy_sol_types::SolCall;
use anyhow::Context;
use common::polynomial_form::PolynomialForm;
use common::serializable_g1::SerializableG1;
use common::verify_blob::IVerifyBlob;
use methods::GUEST_ELF;
use risc0_steel::{ethereum::EthEvmEnv, Contract};
use risc0_zkvm::ProveInfo;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use rust_eigenda_v2_common::{EigenDACert, Payload, PayloadForm};
use rust_kzg_bn254_primitives::blob::Blob;
use rust_kzg_bn254_primitives::helpers::compute_challenge;
use rust_kzg_bn254_prover::kzg::KZG;
use rust_kzg_bn254_prover::srs::SRS;
use url::Url;

pub async fn run_guest(
    eigenda_cert: EigenDACert,
    srs: &SRS,
    data: Vec<u8>,
    rpc_url: Url,
    cert_verifier_wrapper_addr: Address,
    payload_form: PayloadForm,
) -> anyhow::Result<ProveInfo> {
    let call = IVerifyBlob::verifyDACertV2Call {
        batchHeader: eigenda_cert.batch_header.clone().into(),
        blobInclusionInfo: eigenda_cert.blob_inclusion_info.clone().into(),
        nonSignerStakesAndSignature: eigenda_cert.non_signer_stakes_and_signature.clone().into(),
        signedQuorumNumbers: eigenda_cert.signed_quorum_numbers.clone().into(),
    };

    // Create an EVM environment from an RPC endpoint defaulting to the latest block.
    let mut env = EthEvmEnv::builder().rpc(rpc_url.clone()).build().await?;

    // Preflight the call to prepare the input that is required to execute the function in
    // the guest without RPC access. It also returns the result of the call.
    // Risc0 steel creates an ethereum VM using revm, where it simulates the call to VerifyBlobV1.
    // So we need to make this preflight call to populate the VM environment with the current state of the chain
    let mut contract = Contract::preflight(cert_verifier_wrapper_addr, &mut env);
    let returns = contract.call_builder(&call).call().await?;
    println!(
        "Call {} Function on {:#} returns: {}",
        IVerifyBlob::verifyDACertV2Call::SIGNATURE,
        cert_verifier_wrapper_addr,
        returns._0
    );

    // Finally, construct the input from the environment.
    let input = env.into_input().await?;

    let payload = Payload::new(data.clone());
    let encoded_data = payload.to_blob(payload_form)?.serialize();
    let blob = Blob::new(&encoded_data); 

    let mut kzg = KZG::new();

    kzg.calculate_and_store_roots_of_unity(blob.len().try_into()?)?;

    let cert_commitment = eigenda_cert
        .blob_inclusion_info
        .blob_certificate
        .blob_header
        .commitment
        .commitment;

    // Calculate the polynomial in evaluation form
    let poly_coeff = blob.to_polynomial_coeff_form();
    let poly_eval = poly_coeff.to_eval_form()?;

    let evaluation_challenge = compute_challenge(&blob, &cert_commitment)?;

    // Compute the proof that the commitment corresponds to the given blob
    let proof = kzg.compute_proof(&poly_eval, &evaluation_challenge, &srs)?;

    let serializable_proof = SerializableG1 { g1: proof };

    let polynomial_form = match payload_form {
        PayloadForm::Coeff => PolynomialForm::Coeff,
        PayloadForm::Eval => PolynomialForm::Eval
    };
    

    println!("Running the guest with the constructed input...");
    let session_info = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let env = ExecutorEnv::builder()
            .write(&input)?
            .write(&eigenda_cert)?
            .write(&data)?
            .write(&serializable_proof)?
            .write(&cert_verifier_wrapper_addr)?
            .write(&polynomial_form)?
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
