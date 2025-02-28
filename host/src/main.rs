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
use host::verify_blob::decode_blob_info;
use secrecy::Secret;
use tokio_postgres::NoTls;
use url::Url;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
    /// Private key to verify the proof
    #[arg(short, long, env = "PRIVATE_KEY")]
    private_key: Secret<String>,
    /// Chain id where the proof should be verified
    #[arg(short, long, env = "CHAIN_ID")]
    chain_id: String,
    /// Rpc were the proof should be verified
    #[arg(short, long, env = "PROOF_VERIFIER_RPC")]
    proof_verifier_rpc: Secret<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let (client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres password=notsecurepassword dbname=zksync_server_localhost_eigenda", 
        NoTls,
    ).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let mut timestamp =
        chrono::NaiveDateTime::parse_from_str("1970-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")?;

    // Parse the command line arguments.
    let args = Args::parse();

    loop {
        let rows = client
        .query("SELECT inclusion_data, sent_at FROM data_availability WHERE sent_at > $1 AND inclusion_data IS NOT NULL ORDER BY sent_at LIMIT 5", &[&timestamp])
        .await?; // Maybe this approach doesn't work, since maybe row A with has a lower timestamp than row B, but row A has inclusion data NULL so it is not included yet and will never be.
                 // Maybe just look for batch number and go one by one.

        if rows.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }

        timestamp = rows
            .last()
            .ok_or(anyhow::anyhow!("Not enough rows"))?
            .get(1);

        for row in rows {
            let inclusion_data: Vec<u8> = row.get(0);
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
                args.chain_id.clone(),
                args.proof_verifier_rpc.clone(),
            )?;
        }
    }
}
