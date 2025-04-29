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

use std::{str::FromStr, time::Duration};

use alloy_primitives::Address;
use anyhow::Result;
use clap::Parser;
use common::{output::Output, polynomial_form::PolynomialForm};
use ethabi::ethereum_types::H160;
use host::blob_id::get_blob_id;
use methods::GUEST_ELF;
use rust_eigenda_v2_client::{
    core::BlobKey,
    payload_disperser::{PayloadDisperser, PayloadDisperserConfig},
    payloadretrieval::relay_payload_retriever::{
        RelayPayloadRetriever, RelayPayloadRetrieverConfig, SRSConfig,
    },
    relay_client::{RelayClient, RelayClientConfig},
    utils::{PrivateKey, SecretUrl},
};
use rust_eigenda_v2_common::{EigenDACert, Payload, PayloadForm};
use secrecy::{ExposeSecret, Secret};
use tracing_subscriber::EnvFilter;

use reqwest::Client;
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
    #[arg(short, long, env = "EIGENDA_CERT_AND_BLOB_VERIFIER_ADDR")]
    eigenda_cert_and_blob_verifier_addr: String,
    /// Rpc of the eigenda Disperser
    #[arg(short, long, env = "DISPERSER_RPC")]
    disperser_rpc: String,
    /// Blob Verifier Wrapper Contract Address
    #[arg(short, long, env = "CERT_VERIFIER_WRAPPER_ADDR")]
    cert_verifier_wrapper_addr: Address,
    /// Url of the zksync's json api
    #[arg(short, long, env = "API_URL")]
    api_url: String,
    /// Batch number where verification should start
    #[arg(short, long, env = "START_BATCH", value_parser = clap::value_parser!(u64).range(1..))]
    start_batch: u64,
    /// Payload Form of the dispersed blobs
    #[arg(value_enum, env = "PAYLOAD_FORM")]
    payload_form: PolynomialForm,
    /// Blob Version
    #[arg(short, long, env = "BLOB_VERSION")]
    blob_version: u16,
    /// Address of the EigenDA Cert Verifier
    #[arg(short, long, env = "CERT_VERIFIER_ADDR")]
    eigenda_cert_verifier_addr: H160,
    /// Address of the EigenDA Relay Registry
    #[arg(short, long, env = "EIGENDA_RELAY_REGISTRY_ADDR")]
    eigenda_relay_registry_addr: H160,
    /// Keys of the relay client
    #[arg(short, long, env = "RELAY_CLIENT_KEYS", value_delimiter = ',')]
    relay_client_keys: Vec<u32>,
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

    let disperser_pk = args.disperser_private_key.expose_secret();

    let payload_form = match args.payload_form {
        PolynomialForm::Eval => PayloadForm::Eval,
        PolynomialForm::Coeff => PayloadForm::Coeff,
    };

    let payload_disperser_config = PayloadDisperserConfig {
        polynomial_form: payload_form,
        blob_version: args.blob_version,
        cert_verifier_address: args.eigenda_cert_verifier_addr,
        eth_rpc_url: SecretUrl::new(args.rpc_url.clone()),
        disperser_rpc: args.disperser_rpc,
        use_secure_grpc_flag: true,
    };
    let private_key = PrivateKey::from_str(disperser_pk)
        .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;
    let payload_disperser = PayloadDisperser::new(payload_disperser_config, private_key.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Eigen client Error: {:?}", e))?;

    let retriever_config = RelayPayloadRetrieverConfig {
        payload_form,
        retrieval_timeout_secs: Duration::from_secs(60),
    };
    let srs_config = SRSConfig {
        source_path: "resources/g1.point".to_string(),
        order: SRS_ORDER,
        points_to_load: SRS_POINTS_TO_LOAD,
    };

    let relay_client_config = RelayClientConfig {
        max_grpc_message_size: SRS_ORDER as usize,
        relay_clients_keys: args.relay_client_keys,
        relay_registry_address: args.eigenda_relay_registry_addr,
        eth_rpc_url: SecretUrl::new(args.rpc_url.clone()),
    };

    let relay_client = RelayClient::new(relay_client_config, private_key).await?;
    let mut retriever = RelayPayloadRetriever::new(retriever_config, srs_config, relay_client)?;

    let reqwest_client = Client::new();

    let mut current_batch = args.start_batch;

    loop {
        let blob_id: String =
            get_blob_id(current_batch, args.api_url.clone(), &reqwest_client).await?;
        let eigenda_cert: EigenDACert;

        loop {
            let blob_key = BlobKey::from_hex(&blob_id)?;
            let opt_eigenda_cert = payload_disperser.get_inclusion_data(&blob_key).await?;
            if let Some(opt_eigenda_cert) = opt_eigenda_cert {
                eigenda_cert = opt_eigenda_cert;
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Raw bytes dispersed by zksync sequencer to EigenDA
        let payload: Payload = retriever
            .get_payload(eigenda_cert.clone())
            .await
            .map_err(|_| anyhow::anyhow!("Not blob data"))?;

        let blob_data = payload.serialize();

        let result = host::guest_caller::run_guest(
            eigenda_cert.clone(),
            &srs,
            blob_data,
            args.rpc_url.clone(),
            args.cert_verifier_wrapper_addr.clone(),
            payload_form,
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
            eigenda_cert
                .to_bytes()
                .map_err(|_| anyhow::anyhow!("Failed to serialize EigenDACert"))?,
        )
        .await?;

        current_batch += 1;
    }
}
