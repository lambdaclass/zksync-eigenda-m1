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

use std::process::Stdio;

use alloy_primitives::{address, Address};
use alloy_sol_types::{SolCall, SolType};
use anyhow::{Context, Result};
use clap::Parser;
use erc20_methods::ERC20_GUEST_ELF;
use host::verify_blob::{decode_blob_info, IVerifyBlob};
use risc0_steel::{ethereum::EthEvmEnv, Commitment, Contract};
use risc0_zkvm::{compute_image_id, default_executor, default_prover, sha::Digestible, ExecutorEnv, ProverOpts, VerifierContext};
use tokio_postgres::{row, NoTls};
use tracing_subscriber::EnvFilter;
use url::Url;

/// Address of the deployed contract to call the function on.
const CONTRACT: Address = address!("c551b009C1CE0b6efD691E23998AEFd4103680D3"); // If the contract address changes modify this.
/// Address of the caller.
const CALLER: Address = address!("E90E12261CCb0F3F7976Ae611A29e84a6A85f424");

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
    /// Private key to verify the proof
    #[arg(short, long, env = "PRIVATE_KEY")]
    private_key: String // TODO: maybe make this a secret
}

#[tokio::main]
async fn main() -> Result<()> {
    let image_id = compute_image_id(ERC20_GUEST_ELF)?;
    let image_id: risc0_zkvm::sha::Digest = image_id.into();
    let image_id = image_id.as_bytes().to_vec();
    println!("Image id {:?}", image_id);
    let (client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres password=notsecurepassword dbname=zksync_server_localhost_eigenda", 
        NoTls,
    ).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let mut timestamp = chrono::NaiveDateTime::parse_from_str("1970-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")?;

    loop {
        
        let rows = client
        .query("SELECT inclusion_data, sent_at FROM data_availability WHERE sent_at > $1 AND inclusion_data IS NOT NULL ORDER BY sent_at", &[&timestamp])
        .await?; // Maybe this approach doesn't work, since maybe row A with has a lower timestamp than row B, but row A has inclusion data NULL so it is not included yet and will never be.

        println!("Rows len {}", rows.len());

        if rows.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }

        timestamp = rows.last().ok_or(anyhow::anyhow!("Not enough rows"))?.get(1);

        for row in rows {
            let inclusion_data: Vec<u8> = row.get(0);
            let (blob_header, blob_verification_proof) = decode_blob_info(inclusion_data)?;

            let call = IVerifyBlob::verifyBlobV1Call {
                blobHeader: blob_header.clone(),
                blobVerificationProof: blob_verification_proof.clone(),
            };

            // Parse the command line arguments.
            let args = Args::parse();

            // Create an EVM environment from an RPC endpoint defaulting to the latest block.
            let mut env = EthEvmEnv::builder().rpc(args.rpc_url.clone()).build().await?;

            // Preflight the call to prepare the input that is required to execute the function in
            // the guest without RPC access. It also returns the result of the call.
            let mut contract = Contract::preflight(CONTRACT, &mut env);
            let returns = contract.call_builder(&call).from(CALLER).call().await?;
            println!(
                "Call {} Function by {:#} on {:#} returns: {}",
                IVerifyBlob::verifyBlobV1Call::SIGNATURE,
                CALLER,
                CONTRACT,
                returns._0
            ); 

            // Finally, construct the input from the environment.
            let input = env.into_input().await?;
            
            let blob_info = host::blob_info::BlobInfo {
                blob_header: blob_header.into(),
                blob_verification_proof: blob_verification_proof.clone().into(),
            };

            println!("Running the guest with the constructed input...");
            let session_info = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
                let env = ExecutorEnv::builder()
                    .write(&input)?
                    .write(&blob_info)?
                    .build()?;
                let exec = default_prover();
                exec.prove_with_ctx(env,&VerifierContext::default(), ERC20_GUEST_ELF,&ProverOpts::groth16())
                    .context("failed to run executor")
            }).await??;

            let block_proof = match session_info.receipt.inner.groth16() {
                Ok(inner) => {
                    // The SELECTOR is used to perform an extra check inside the groth16 verifier contract.
                    let mut selector = hex::encode(
                        inner
                            .verifier_parameters
                            .as_bytes()
                            .get(..4).ok_or(anyhow::anyhow!("verifier parameters too short"))?,
                    );
                    let seal = hex::encode(inner.clone().seal);
                    selector.push_str(&seal);
                    hex::decode(selector)?
                }
                Err(_) => vec![0u8; 4],
            };

            let journal_digest = Digestible::digest(&session_info.receipt.journal)
                .as_bytes()
                .to_vec();

            println!("journal digest {:?}", hex::encode(journal_digest.clone()));
            println!("image id digest {:?}", hex::encode(image_id.clone()));
            println!("block proof {:?}", hex::encode(block_proof.clone()));

            let output = std::process::Command::new("forge")
                .arg("script")
                .arg("contracts/script/ProofVerifier.s.sol:ProofVerifier")
                .arg("--rpc-url")
                .arg("https://ethereum-holesky-rpc.publicnode.com")
                .arg("--broadcast")
                .arg("-vvvv")
                .env("PRIVATE_KEY", args.private_key) // Set environment variable
                .env("SEAL", format!("0x{}", hex::encode(&block_proof))) // Convert seal to hex string
                .env("IMAGE_ID", format!("0x{}", hex::encode(&image_id))) // Convert image ID to hex string
                .env("JOURNAL_DIGEST", format!("0x{}", hex::encode(&journal_digest))) // Convert journal digest to hex string
                .output()?;

            let stdout = std::str::from_utf8(&output.stdout).unwrap_or("");
            let stderr = std::str::from_utf8(&output.stderr).unwrap_or("");
        
            // Combine stdout and stderr for parsing
            let combined_output = format!("{}\n{}", stdout, stderr);
        
            if output.status.success() {
                // Extract the transaction hash (regex looks for 0x followed by 64 hex chars)
                let tx_hash = combined_output
                .lines()
                .find(|line| line.contains("[Success] Hash: 0x"))
                .and_then(|line| line.split_whitespace().find(|s| s.starts_with("0x")))
                .unwrap_or("Transaction hash not found");
                println!("Proof of data inclusion for blob {} verified on L1. Tx hash: {tx_hash}",blob_verification_proof.blobIndex);
            } else {
                println!("Proof verification failed");
            }
        }
    }
}
