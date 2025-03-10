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
use host::eigen_client::EigenClientRetriever;
use host::proof_equivalence;
use host::verify_blob::decode_blob_info;
use tokio_postgres::NoTls;
use tracing_subscriber::EnvFilter;

use ark_bn254::G1Affine;
use url::Url;
use blob_verification_methods::BLOB_VERIFICATION_GUEST_ELF;
use proof_equivalence_methods::PROOF_EQUIVALENCE_GUEST_ELF;
use serde::{Serialize, Serializer};
use serde::ser::SerializeTuple;
use rust_kzg_bn254_prover::srs::SRS;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
    /// Private key to verify the proof
    #[arg(short, long, env = "PRIVATE_KEY")]
    private_key: String, // TODO: maybe make this a secret
    /// Chain id were the proof should be verified
    #[arg(short, long, env = "CHAIN_ID")]
    chain_id: String,
    /// Rpc were the proof should be verified
    #[arg(short, long, env = "PROOF_VERIFIER_RPC")]
    proof_verifier_rpc: String,
    /// Rpc of the eigenda Disperser
    #[arg(short, long, env = "DISPERSER_RPC")]
    disperser_rpc: String,
}

pub struct SerializableG1 {
    pub g1: G1Affine
}

impl Serialize for SerializableG1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let x = format!("{:?}",self.g1.x);
        let y = format!("{:?}",self.g1.y);
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&x).unwrap();
        tup.serialize_element(&y).unwrap();
        tup.end()
    }
}

#[tokio::main]
async fn main() -> Result<()> {

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let srs = SRS::new("resources/g1.point", 268435456, 1024 * 1024 * 2 / 32).unwrap();

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

    let eigen_client = EigenClientRetriever::new(&args.disperser_rpc).await?;

    loop {
        let rows = client
        .query("SELECT inclusion_data, sent_at FROM data_availability WHERE sent_at > $1 AND inclusion_data IS NOT NULL ORDER BY sent_at", &[&timestamp])
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
            let (blob_header, blob_verification_proof, batch_header_hash) = decode_blob_info(inclusion_data)?;
            let blob_data = eigen_client.get_blob_data(blob_verification_proof.blobIndex, batch_header_hash).await?.ok_or(anyhow::anyhow!("Not blob data"))?;

            println!("Executing Proof Equivalence guest");
            let proof_equivalence_result = proof_equivalence::run_proof_equivalence(&srs, blob_header.clone().commitment,blob_data).await?;
            
            println!("Verifying Proof Equivalence guest");
            host::prove_risc0::prove_risc0_proof(
                proof_equivalence_result,
                PROOF_EQUIVALENCE_GUEST_ELF,
                args.private_key.clone(),
                blob_verification_proof.blobIndex,
                args.chain_id.clone(),
                args.proof_verifier_rpc.clone(),
            )?;

            println!("Executing Blob Verification guest");
            let blob_verification_result = host::verify_blob::run_blob_verification_guest(
                blob_header.clone(),
                blob_verification_proof.clone(),
                args.rpc_url.clone(),
            )
            .await?;

            println!("Verifying Blob Verification guest");
            host::prove_risc0::prove_risc0_proof(
                blob_verification_result,
                BLOB_VERIFICATION_GUEST_ELF,
                args.private_key.clone(),
                blob_verification_proof.blobIndex,
                args.chain_id.clone(),
                args.proof_verifier_rpc.clone(),
            )?;
        }
    }
}
