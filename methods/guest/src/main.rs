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

use alloy_primitives::{address, Address};
use common::blob_info::BlobInfo;
use blob_verification_guest::verify_blob::IVerifyBlob;
use risc0_steel::{ethereum::EthEvmInput, Contract};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

/// Address of the deployed blob verifier wrapper contract to call the function.
const BLOB_VERIFIER_WRAPPER_CONTRACT: Address = address!("c551b009C1CE0b6efD691E23998AEFd4103680D3"); // If the contract address changes modify this.
/// Address of the caller. If not provided, the caller will be the [CONTRACT].
const CALLER: Address = address!("E90E12261CCb0F3F7976Ae611A29e84a6A85f424");

/// This guest uses the risc0 Steel library to prove an eth_call on the BlobVerifierWrapper. 
/// It receives serialized blob_info from the host, which it uses as arguments to the eth_call.
fn main() {
    // Read the input from the guest environment.
    let input: EthEvmInput = env::read();
    let blob_info: BlobInfo = env::read();

    // Converts the input into a `EvmEnv` for execution.
    let env = input.into_env();
    
    // Execute the view call; it returns the result in the type generated by the `sol!` macro.
    let contract = Contract::new(BLOB_VERIFIER_WRAPPER_CONTRACT, &env);
    let call = IVerifyBlob::verifyBlobV1Call {
        blobHeader: blob_info.blob_header.into(),
        blobVerificationProof: blob_info.blob_verification_proof.into(),
    };
    let returns = contract.call_builder(&call).from(CALLER).call();
    println!("View call result: {}", returns._0);
    // Commit the block hash and number used when deriving `EvmEnv` to the journal.
    env::commit_slice(&env.commitment().abi_encode());
}
