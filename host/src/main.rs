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

use std::{collections::HashMap, sync::Arc, time::Duration};

use alloy_primitives::Address;
use anyhow::Result;
use clap::Parser;
use common::{output::Output, polynomial_form::PolynomialForm};
use ethabi::ethereum_types::H160;
use jsonrpc_core::{IoHandler, Params};
use jsonrpc_http_server::ServerBuilder;
use methods::GUEST_ELF;
use rust_eigenda_v2_client::{
    core::BlobKey,
    payload_disperser::{PayloadDisperser, PayloadDisperserConfig},
    relay_client::{RelayClient, RelayClientConfig},
    relay_payload_retriever::{RelayPayloadRetriever, RelayPayloadRetrieverConfig, SRSConfig},
    rust_eigenda_signers::signers::private_key::Signer,
    utils::SecretUrl,
};
use rust_eigenda_v2_common::{EigenDACert, Payload, PayloadForm};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};
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
    #[arg(short, long, env = "EIGENDA_CERT_AND_BLOB_VERIFIER_ADDR")]
    eigenda_cert_and_blob_verifier_addr: String,
    /// Rpc of the eigenda Disperser
    #[arg(short, long, env = "DISPERSER_RPC")]
    disperser_rpc: String,
    /// Blob Verifier Wrapper Contract Address
    #[arg(short, long, env = "CERT_VERIFIER_WRAPPER_ADDR")]
    cert_verifier_wrapper_addr: Address,
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

#[derive(Deserialize)]
struct GenerateProofParams {
    blob_id: String,
}

enum BlobIdProofStatus {
    Queued,
    Finished(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let srs = SRS::new("resources/g1.point", SRS_ORDER, SRS_POINTS_TO_LOAD)?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let requests = Arc::new(Mutex::new(HashMap::new()));
    let (tx, mut rx) = mpsc::channel(100);

    let requests_clone = requests.clone();
    let proof_gen_thread: JoinHandle<Result<()>> = tokio::spawn(async move {
        let requests = requests_clone;

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
        let private_key = disperser_pk
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;
        let signer = Signer::new(private_key);
        let payload_disperser = PayloadDisperser::new(payload_disperser_config, signer.clone())
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

        let relay_client = RelayClient::new(relay_client_config, signer).await?;
        let mut retriever = RelayPayloadRetriever::new(retriever_config, srs_config, relay_client)?;

        println!("Running proof gen thread");
        loop {
            let blob_id: String = match rx.recv().await {
                Some(blob_id) => blob_id,
                None => continue,
            };

            println!("Proof gen thread: received request to prove: {}", blob_id);

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

            println!(
                "Proof gen thread: finished generating proof for Blob Id {}",
                blob_id
            );
            requests
                .lock()
                .await
                .insert(blob_id, BlobIdProofStatus::Finished("PROOF".to_string()));
        }
    });

    let json_rpc_server_thread = tokio::spawn(async {
        let mut io = IoHandler::new();
        io.add_method("generate_proof", move |params: Params| {
            let tx = tx.clone();
            let requests = requests.clone();
            async move {
                let parsed: GenerateProofParams = params.parse().map_err(|_| {
                    jsonrpc_core::Error::invalid_params(
                        "Expected a single string parameter 'blob_id'",
                    )
                })?;
                let blob_id = parsed.blob_id;

                let mut requests_lock = requests.lock().await;
                match requests_lock.get(&blob_id) {
                    Some(req) => match req {
                        BlobIdProofStatus::Finished(proof) => {
                            return Ok(jsonrpc_core::Value::String(format!(
                                "Blob Id {} proved: {}",
                                blob_id, proof
                            )))
                        }
                        BlobIdProofStatus::Queued => {
                            return Ok(jsonrpc_core::Value::String(format!(
                                "Blob Id {} already submitted",
                                blob_id
                            )))
                        }
                    },
                    None => {
                        requests_lock.insert(blob_id.clone(), BlobIdProofStatus::Queued);
                    }
                }

                tx.send(blob_id.clone()).await.map_err(|_| {
                    println!("Failed sending Blob Id {} to prover thread", blob_id);
                    jsonrpc_core::Error::internal_error()
                })?;

                Ok(jsonrpc_core::Value::String(format!(
                    "Generating Proof for {}",
                    blob_id
                )))
            }
        });

        let server = ServerBuilder::new(io)
            .start_http(&"127.0.0.1:3030".parse().unwrap()) // TODO: make custom?
            .expect("Unable to start server");
        println!("Running JSON RPC server");
        server.wait();
    });

    let res = tokio::try_join!(proof_gen_thread, json_rpc_server_thread)?;
    res.0?; // TODO: this looks clumsy
    Ok(())
}
