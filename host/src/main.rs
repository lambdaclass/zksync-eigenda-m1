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

use alloy_primitives::{address, Address};
use alloy_sol_types::{SolCall, SolType};
use anyhow::{Context, Result};
use clap::Parser;
use erc20_methods::ERC20_GUEST_ELF;
use host::verify_blob::{decode_blob_info, IVerifyBlob};
use risc0_steel::{ethereum::EthEvmEnv, Commitment, Contract};
use risc0_zkvm::{compute_image_id, default_executor, default_prover, ExecutorEnv, ProverOpts, VerifierContext};
use tokio_postgres::NoTls;
use tracing_subscriber::EnvFilter;
use url::Url;

/// Address of the deployed contract to call the function on (USDT contract on Sepolia).
const CONTRACT: Address = address!("c551b009C1CE0b6efD691E23998AEFd4103680D3"); //TODO: Add the address of the deployed contract.
/// Address of the caller.
const CALLER: Address = address!("e706e60ab5Dc512C36A4646D719b889F398cbBcB");

/// Simple program to show the use of Ethereum contract data inside the guest.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
}

#[tokio::main]
async fn main() -> Result<()> {

    let imageid = compute_image_id(ERC20_GUEST_ELF)?;
    println!("Image ID: {:?}", imageid);
    let (client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres password=notsecurepassword dbname=zksync_server_localhost_eigenda", 
        NoTls,
    ).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let rows = client
        .query("SELECT inclusion_data FROM data_availability", &[])
        .await?;

    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

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
        let mut env = EthEvmEnv::builder().rpc(args.rpc_url).build().await?;
        //  The `with_chain_spec` method is used to specify the chain configuration.

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
            blob_verification_proof: blob_verification_proof.into(),
        };

        println!("Running the guest with the constructed input...");
        let session_info = tokio::task::spawn_blocking(move || {
            let env = ExecutorEnv::builder()
                .write(&input)
                .unwrap()
                .write(&blob_info)
                .unwrap()
                .build()
                .unwrap();
            let exec = default_prover();
            exec.prove_with_ctx(env,&VerifierContext::default(), ERC20_GUEST_ELF,&ProverOpts::succinct())
                .context("failed to run executor")
        }).await??;

        // The journal should be the ABI encoded commitment.
        let commitment = Commitment::abi_decode(session_info.receipt.journal.as_ref(), true)
            .context("failed to decode journal")?;
        println!("{:?}", commitment);
    }
    Ok(())
}
