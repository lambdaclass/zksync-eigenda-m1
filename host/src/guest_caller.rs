use alloy_primitives::{address, Address};
use alloy_sol_types::SolCall;
use anyhow::Context;
use methods::GUEST_ELF;
use risc0_steel::{ethereum::EthEvmEnv, Contract};
use risc0_zkvm::ProveInfo;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use url::Url;
use crate::verify_blob::{BlobHeader, BlobVerificationProof, IVerifyBlob};

/// Address of the deployed blob verifier wrapper contract to call the function on.
const BLOB_VERIFIER_WRAPPER_CONTRACT: Address =
    address!("c551b009C1CE0b6efD691E23998AEFd4103680D3"); // If the contract address changes modify this.
/// Address of the caller.
const CALLER: Address = address!("E90E12261CCb0F3F7976Ae611A29e84a6A85f424");

use ark_bn254::{Fq, G1Affine};
use common::serializable_g1::SerializableG1;
use rust_kzg_bn254_prover::srs::SRS;
use rust_kzg_bn254_primitives::blob::Blob;
use rust_kzg_bn254_prover::kzg::KZG;

use crate::verify_blob::G1Point;

pub async fn run_guest(
    blob_header: BlobHeader,
    blob_verification_proof: BlobVerificationProof,
    srs: &SRS,
    commitment: G1Point,
    data: Vec<u8>,
    rpc_url: Url,
) -> anyhow::Result<ProveInfo> {
    let call = IVerifyBlob::verifyBlobV1Call {
        blobHeader: blob_header.clone(),
        blobVerificationProof: blob_verification_proof.clone(),
    };

    // Create an EVM environment from an RPC endpoint defaulting to the latest block.
    let mut env = EthEvmEnv::builder().rpc(rpc_url.clone()).build().await?;

    // Preflight the call to prepare the input that is required to execute the function in
    // the guest without RPC access. It also returns the result of the call.
    let mut contract = Contract::preflight(BLOB_VERIFIER_WRAPPER_CONTRACT, &mut env);
    let returns = contract.call_builder(&call).from(CALLER).call().await?;
    println!(
        "Call {} Function by {:#} on {:#} returns: {}",
        IVerifyBlob::verifyBlobV1Call::SIGNATURE,
        CALLER,
        BLOB_VERIFIER_WRAPPER_CONTRACT,
        returns._0
    );

    // Finally, construct the input from the environment.
    let input = env.into_input().await?;

    let blob_info = common::blob_info::BlobInfo {
        blob_header: blob_header.into(),
        blob_verification_proof: blob_verification_proof.clone().into(),
    };

    let blob = Blob::from_raw_data(&data);

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

    println!("Running the guest with the constructed input...");
    let session_info = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let env = ExecutorEnv::builder()
            .write(&input)?
            .write(&blob_info)?
            .write(&data)?
            .write(&serializable_eval)?
            .write(&serializable_proof)?
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
