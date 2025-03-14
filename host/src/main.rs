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

const LAST_BATCH_FILE: &str = "last_batch.txt";

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
    /// Private key used to submit an ethereum transaction that verifys the proof
    #[arg(short, long, env = "PRIVATE_KEY")]
    private_key: Secret<String>,
    /// Rpc were the proof should be verified
    #[arg(short, long, env = "PROOF_VERIFIER_RPC")]
    proof_verifier_rpc: Secret<String>,
    /// Url of the zksync's json api
    #[arg(short, long, env = "API_URL")]
    api_url: String,
    /// If activated, it will start from the last batch verified
    #[arg(short, long, env = "RESTORE")]
    restore: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the command line arguments.
    let args = Args::parse();

    let client = Client::new();

    let mut current_batch = 1;
    if args.restore {
        let content = tokio::fs::read_to_string(LAST_BATCH_FILE).await?; // Read file as string
        current_batch = content.trim().parse()?;
    }

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
            args.proof_verifier_rpc.clone(),
        )
        .await?;
    
        current_batch += 1;
        tokio::fs::write(LAST_BATCH_FILE, current_batch.to_string()).await?;
    }
}
