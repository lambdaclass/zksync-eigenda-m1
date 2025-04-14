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
use host::blob_id::get_blob_id;
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
use tracing_subscriber::EnvFilter;

use rust_kzg_bn254_prover::srs::SRS;
use reqwest::Client;
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
    #[arg(short, long, env = "EIGENDA_CERT_AND_BLOB_VERIFIER_ADDR")]
    eigenda_cert_and_blob_verifier_addr: String,
    /// Rpc of the eigenda Disperser
    #[arg(short, long, env = "DISPERSER_RPC")]
    disperser_rpc: String,
    /// Service Manager Address
    #[arg(short, long, env = "SVC_MANAGER_ADDR")]
    svc_manager_addr: String,
    /// Blob Verifier Wrapper Contract Address
    #[arg(short, long, env = "BLOB_VERIFIER_WRAPPER_ADDR")]
    blob_verifier_wrapper_addr: Address,
    /// Url of the zksync's json api
    #[arg(short, long, env = "API_URL")]
    api_url: String,
    /// Batch number where verification should start
    #[arg(short, long, env = "START_BATCH", value_parser = clap::value_parser!(u64).range(1..))]
    start_batch: u64,
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

const SRS_ORDER: u32 = 268435456;
const SRS_POINTS_TO_LOAD: u32 = 1024 * 1024 * 2 / 32;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let srs = SRS::new("resources/g1.point", SRS_ORDER, SRS_POINTS_TO_LOAD)?;
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
    let reqwest_client = Client::new();

    let mut current_batch = args.start_batch;

    loop {
        let blob_id: String = get_blob_id(current_batch, args.api_url.clone(), &reqwest_client).await?;
        // Abi encoded BlobInfo (EigenDACert)
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
        
        // Raw bytes dispersed by zksync sequencer to EigenDA
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
        )
        .await?;

        let output: Output = result.receipt.journal.decode()?;

        host::prove_risc0::prove_risc0_proof(
            result,
            GUEST_ELF,
            args.verification_private_key.clone(),
            args.rpc_url.clone(),
            args.eigenda_cert_and_blob_verifier_addr.clone(),
            output.hash,
            inclusion_data,
        )
        .await?;

        current_batch += 1;
    }
}
