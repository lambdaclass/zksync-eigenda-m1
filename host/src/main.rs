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

use std::{str::FromStr, sync::Arc};

use alloy_primitives::Address;
use anyhow::Result;
use clap::Parser;
use common::output::Output;
use host::eigen_client::EigenClientRetriever;
use host::verify_blob::decode_blob_info;
use methods::GUEST_ELF;
use rust_eigenda_client::{
    client::BlobProvider,
    config::{EigenConfig, EigenSecrets, PrivateKey, SecretUrl, SrsPointsSource},
    EigenClient,
};
use secrecy::{ExposeSecret, Secret};
use std::error::Error;
use tokio_postgres::NoTls;
use tracing_subscriber::EnvFilter;

use rust_kzg_bn254_prover::srs::SRS;
use url::Url;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
    /// Private key used to submit an ethereum transaction that verifys the proof
    #[arg(short, long, env = "VERIFICATION_PRIVATE_KEY")]
    verification_private_key: Secret<String>,
    /// Private key used to get inclusion data from the disperser
    #[arg(short, long, env = "DISPERSER_PRIVATE_KEY")]
    disperser_private_key: Secret<String>,
    /// Address of the EigenDA Registry
    #[arg(short, long, env = "EIGENDA_REGISTRY_ADDR")]
    eigenda_registry_addr: String,
    /// Rpc of the eigenda Disperser
    #[arg(short, long, env = "DISPERSER_RPC")]
    disperser_rpc: String,
    /// Service Manager Address
    #[arg(short, long, env = "SVC_MANAGER_ADDR")]
    svc_manager_addr: String,
    /// Blob Verifier Wrapper Contract Address
    #[arg(short, long, env = "BLOB_VERIFIER_WRAPPER_ADDR")]
    blob_verifier_wrapper_addr: Address,
    /// Caller Address
    #[arg(short, long, env = "CALLER_ADDR")]
    caller_addr: Address,
}

#[derive(Debug)]
struct FakeBlobProvider;

#[async_trait::async_trait]
impl BlobProvider for FakeBlobProvider {
    async fn get_blob(
        &self,
        _input: &str,
    ) -> Result<Option<Vec<u8>>, Box<dyn Error + Send + Sync>> {
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let srs = SRS::new("resources/g1.point", 268435456, 1024 * 1024 * 2 / 32)?;

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

    let eigen_retriever = EigenClientRetriever::new(&args.disperser_rpc).await?;

    let disperser_pk = args.disperser_private_key.expose_secret();

    let eigen_client = EigenClient::new(
        EigenConfig::new(
            args.disperser_rpc,
            SecretUrl::new(args.rpc_url.clone()),
            0,
            ethabi::ethereum_types::H160::from_str(&args.svc_manager_addr)?,
            false,
            false,
            SrsPointsSource::Path("./resources".to_string()),
            vec![],
        )?,
        EigenSecrets {
            private_key: PrivateKey::from_str(
                disperser_pk.strip_prefix("0x").unwrap_or(disperser_pk),
            )?,
        },
        Arc::new(FakeBlobProvider {}),
    )
    .await?;

    loop {
        let rows = client
        .query("SELECT blob_id, sent_at FROM data_availability WHERE sent_at > $1 ORDER BY sent_at LIMIT 5", &[&timestamp])
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
            let blob_id: String = row.get(0);
            let inclusion_data: Vec<u8>;
            loop {
                let opt_inclusion_data = eigen_client.get_inclusion_data(&blob_id).await?;
                if let Some(opt_inclusion_data) = opt_inclusion_data {
                    inclusion_data = opt_inclusion_data;
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
            let (blob_header, blob_verification_proof, batch_header_hash) =
                decode_blob_info(inclusion_data.clone())?;
            let blob_data = eigen_retriever
                .get_blob_data(blob_verification_proof.blobIndex, batch_header_hash)
                .await?
                .ok_or(anyhow::anyhow!("Not blob data"))?;

            let result = host::guest_caller::run_guest(
                blob_header.clone(),
                blob_verification_proof.clone(),
                &srs,
                blob_data,
                args.rpc_url.clone(),
                args.blob_verifier_wrapper_addr.clone(),
                args.caller_addr.clone(),
            )
            .await?;

            let output: Output = result.receipt.journal.decode()?;

            host::prove_risc0::prove_risc0_proof(
                result,
                GUEST_ELF,
                args.verification_private_key.clone(),
                args.rpc_url.clone(),
                args.eigenda_registry_addr.clone(),
                output.hash,
                inclusion_data,
            )
            .await?;
        }
    }
}
