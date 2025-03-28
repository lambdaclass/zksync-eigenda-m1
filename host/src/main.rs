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

use anyhow::Result;
use clap::Parser;
use host::{inclusion_data::get_inclusion_data, verify_blob::decode_blob_info};
use reqwest::Client;
use secrecy::Secret;
use url::Url;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
    /// Private key used to submit an ethereum transaction that verifys the proof
    #[arg(short, long, env = "PRIVATE_KEY")]
    private_key: Secret<String>,
    /// Url of the zksync's json api
    #[arg(short, long, env = "API_URL")]
    api_url: String,
    /// Batch number where verification should start
    #[arg(short, long, env = "START_BATCH", value_parser = clap::value_parser!(u64).range(1..))]
    start_batch: u64,
    /// Address of the Risc0 Verifier Wrapper
    #[arg(short, long, env = "RISC0_VERIFIER_WRAPPER")]
    risc0_verifier_address: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the command line arguments.
    let args = Args::parse();

    let client = Client::new();

    let mut current_batch = args.start_batch;

    loop {
        let inclusion_data = get_inclusion_data(current_batch, args.api_url.clone(), &client).await?;
        
        let (blob_header, blob_verification_proof) = decode_blob_info(inclusion_data)?;
        let session_info = host::verify_blob::run_blob_verification_guest(
            blob_header,
            blob_verification_proof.clone(),
            args.rpc_url.clone(),
        )
        .await?;

        host::prove_risc0::prove_risc0_proof(
            session_info,
            args.private_key.clone(),
            blob_verification_proof.blobIndex,
            args.rpc_url.clone(),
            args.risc0_verifier_address.clone()
        )
        .await?;
    
        current_batch += 1;
    }
}
