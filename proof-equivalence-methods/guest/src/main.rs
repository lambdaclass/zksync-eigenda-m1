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

use risc0_zkvm::guest::env;

use rust_kzg_bn254_primitives::blob::Blob;
use rust_kzg_bn254_verifier::verify::verify_blob_kzg_proof;
use common::serializable_g1::SerializableG1;

risc0_zkvm::guest::entry!(main);

fn main() {
    let data: Vec<u8> = env::read();

    let blob = Blob::from_raw_data(&data);

    let eval_commitment: SerializableG1 = env::read();

    let proof: SerializableG1 = env::read();

    let verified = verify_blob_kzg_proof(&blob, &eval_commitment.g1, &proof.g1).unwrap();
    
    assert!(verified);

    env::commit(&verified);
}
