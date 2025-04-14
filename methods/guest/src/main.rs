// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(unused_doc_comments)]
#![no_main]

use alloy_primitives::Address;
use common::blob_info::BlobInfo;
use common::output::Output;
use common::serializable_g1::SerializableG1;
use guest::verify_blob::IVerifyBlob;
use risc0_steel::{ethereum::EthEvmInput, Contract};
use risc0_zkvm::guest::env;
use rust_kzg_bn254_primitives::blob::Blob;
use rust_kzg_bn254_verifier::verify::verify_blob_kzg_proof;
use tiny_keccak::{Hasher, Keccak};

risc0_zkvm::guest::entry!(main);

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(data);
    hasher.finalize(&mut output);
    output
}

/// This guest proves that an EigenDA Cert (BlobInfo) is valid, as well as that
/// the cert's commitment corresponds to a given blob.
/// It uses the risc0 Steel library to prove the cert validity via an eth_call on the BlobVerifierWrapper.
/// It receives serialized blob_info from the host, which it uses as arguments to the eth_call.
/// Then it verifies that the received blob (data) commits to the same commitment as found in the blob_info
/// It also computes the keccak256 hash of the blob data, which is used as a public output.
/// This is done to later compare on EigenDAL1DAValidator against the calculated hashes
fn main() {
    // Read the input from the guest environment.
    let input: EthEvmInput = env::read();
    // aka EigenDACert
    let blob_info: BlobInfo = env::read();
    // Raw bytes dispersed by zksync's sequencer to EigenDA
    let data: Vec<u8> = env::read();
    // Commitment to the blob
    let eval_commitment: SerializableG1 = env::read();
    // Proof that the given commitment commits to the blob
    let proof: SerializableG1 = env::read();
    let blob_verifier_wrapper_addr: Address = env::read();
    // Address that is used to call the VerifyBlobV1 function
    let caller_addr: Address = env::read();
    let blob = Blob::from_raw_data(&data);

    // Converts the input into a `EvmEnv` for execution.
    let env = input.into_env();

    // Execute the view call; it returns the result in the type generated by the `sol!` macro.
    let contract = Contract::new(blob_verifier_wrapper_addr, &env);
    let call = IVerifyBlob::verifyBlobV1Call {
        blobHeader: blob_info.blob_header.into(),
        blobVerificationProof: blob_info.blob_verification_proof.into(),
    };
    let returns = contract.call_builder(&call).from(caller_addr).call();
    println!("View call result: {}", returns._0);
    // Here we assert that the result of the verifyBlobV1 call is true, meaning it executed correctly
    assert!(returns._0);

    // Verification of the kzg proof for the given commitment and blob
    let verified = verify_blob_kzg_proof(&blob, &eval_commitment.g1, &proof.g1).unwrap();
    assert!(verified);

    // Here we calculate the keccak hash of the data, which we will use on zksync's EigenDAL1Validator to compare it to the hashes there
    let hash = keccak256(&data);

    // Public outputs of the guest, eigenDAHash and commitment to the risc0 steel environment, they are embedded on the proof
    let output = Output {
        hash: hash.to_vec(),
        env_commitment: env.commitment().abi_encode(),
    };

    env::commit(&output);
}
