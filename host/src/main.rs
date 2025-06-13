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

use std::{sync::Arc, time::Duration};

use alloy_primitives::Address;
use anyhow::Result;
use clap::Parser;
use common::{output::Output, polynomial_form::PolynomialForm};
use ethabi::Token;
use host::db::{
    mark_blob_proof_request_failed, proof_request_exists, retrieve_blob_id_proof,
    retrieve_next_pending_proof, store_blob_proof, store_blob_proof_request,
};
use jsonrpc_core::{ErrorCode, IoHandler, Params};
use jsonrpc_http_server::ServerBuilder;
use methods::GUEST_ELF;
use risc0_zkvm::{compute_image_id, sha::Digestible};
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
use sqlx::PgPool;
use tokio::{sync::Mutex, task::JoinHandle};
use tracing_subscriber::EnvFilter;
use tiny_http::{Header, Response, Server as MetricsServer};

use rust_kzg_bn254_prover::srs::SRS;
use url::Url;

use prometheus::{
    self, register_histogram_vec, register_int_counter, Encoder, HistogramVec, IntCounter,
};

// Prometheus metrics
lazy_static::lazy_static! {
static ref PROOF_GEN_REQ_COUNTER: IntCounter =
    register_int_counter!("proof_requests", "Number of proof generation requests received").unwrap();

    static ref PROOF_GEN_SUCCESS_COUNTER: IntCounter =
    register_int_counter!("proof_generations", "Number of successful proofs generated").unwrap();

    static ref PROOF_GEN_FAILURE_COUNTER: IntCounter =
    register_int_counter!("proof_generation_failures", "Number of failed proof generations").unwrap();

    static ref PROOF_RET_REQ_COUNTER: IntCounter =
    register_int_counter!("proof_retrievals", "Number of proof retrieval requests received").unwrap();

    static ref PROOF_GEN_TIME_HISTOGRAM: HistogramVec =
        register_histogram_vec!(
            "proof_generation_seconds",
            "Time taken to generate a proof in seconds",
            &["blob_id"]
        ).unwrap();
    }

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
    /// Private key used to get inclusion data from the disperser
    #[arg(short, long, env = "DISPERSER_PRIVATE_KEY")]
    disperser_private_key: Secret<String>,
    /// Rpc of the eigenda Disperser
    #[arg(short, long, env = "DISPERSER_RPC")]
    disperser_rpc: String,
    /// Payload Form of the dispersed blobs
    #[arg(value_enum, env = "PAYLOAD_FORM")]
    payload_form: PolynomialForm,
    /// Blob Version
    #[arg(short, long, env = "BLOB_VERSION")]
    blob_version: u16,
    /// Address of the EigenDA Cert Verifier Router
    #[arg(short, long, env = "CERT_VERIFIER_ROUTER_ADDR")]
    eigenda_cert_verifier_router_addr: Address,
    /// Address of the EigenDA Relay Registry
    #[arg(short, long, env = "EIGENDA_RELAY_REGISTRY_ADDR")]
    eigenda_relay_registry_addr: Address,
    /// Keys of the relay client
    #[arg(short, long, env = "RELAY_CLIENT_KEYS", value_delimiter = ',')]
    relay_client_keys: Vec<u32>,
    /// URL where this sidecar should run
    #[arg(short, long, env = "SIDECAR_URL")]
    sidecar_url: String,
    /// URL of the database
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,
    /// URL where the metrics server should run
    #[arg(short, long, env = "METRICS_URL")]
    metrics_url: String,
    /// Address of the Eigen Registry Coordinator
    #[arg(short, long, env = "REGISTRY_COORDINATOR_ADDR")]
    registry_coordinator_addr: String,
    /// Address of the Eigen Operator State Retriever
    #[arg(short, long, env = "OPERATOR_STATE_RETRIEVER_ADDR")]
    operator_state_retriever_addr: String,
}

const SRS_ORDER: u32 = 268435456;
const SRS_POINTS_TO_LOAD: u32 = 1024 * 1024 * 2 / 32;

// The JSON RPC Specification defines for the Server error (Reserved for implementation-defined server-errors) the range of codes -32000 to -32099
const PROOF_NOT_FOUND_ERROR: i64 = -32001;

#[derive(Deserialize)]
struct GenerateProofParams {
    blob_id: String,
}

async fn flatten(handle: JoinHandle<Result<()>>) -> Result<()> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err),
        Err(_) => Err(anyhow::anyhow!("handling failed")),
    }
}

// Receives a blob ID with the arguments needed to generate its proof.
// Returns the proof encoded as and ethabi Tuple.
async fn generate_proof(
    blob_id: String,
    payload_disperser: Arc<PayloadDisperser>,
    retriever: Arc<Mutex<RelayPayloadRetriever>>,
    srs: &SRS,
    rpc_url: Url,
    cert_verifier_router_addr: Address,
    payload_form: PayloadForm,
) -> Result<Vec<u8>> {
    let eigenda_cert: EigenDACert;
    loop {
        let blob_key = BlobKey::from_hex(&blob_id)?;
        let opt_eigenda_cert = payload_disperser.get_cert(&blob_key).await?;
        if let Some(opt_eigenda_cert) = opt_eigenda_cert {
            eigenda_cert = opt_eigenda_cert;
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    // Raw bytes dispersed by zksync sequencer to EigenDA
    let payload: Payload = retriever
        .lock()
        .await
        .get_payload(eigenda_cert.clone())
        .await?;

    let blob_data = payload.serialize();

    let result = host::guest_caller::run_guest(
        eigenda_cert.clone(),
        srs,
        blob_data,
        rpc_url,
        cert_verifier_router_addr,
        payload_form,
    )
    .await?;

    let output: Output = result.receipt.journal.decode()?;

    let image_id = compute_image_id(GUEST_ELF)?;
    let image_id: risc0_zkvm::sha::Digest = image_id;
    let image_id = image_id.as_bytes().to_vec();

    let block_proof = match result.receipt.inner.groth16() {
        Ok(inner) => {
            // The SELECTOR is used to perform an extra check inside the groth16 verifier contract.
            let mut selector = hex::encode(
                inner
                    .verifier_parameters
                    .as_bytes()
                    .get(..4)
                    .ok_or(anyhow::anyhow!("verifier parameters too short"))?,
            );
            let seal = hex::encode(inner.clone().seal);
            selector.push_str(&seal);
            hex::decode(selector)?
        }
        Err(_) => vec![0u8; 4],
    };

    let journal_digest = Digestible::digest(&result.receipt.journal)
        .as_bytes()
        .to_vec();

    let proof = ethabi::encode(&[Token::Tuple(vec![
        Token::Bytes(block_proof),
        Token::FixedBytes(image_id),
        Token::FixedBytes(journal_digest),
        Token::FixedBytes(output.hash),
    ])]);

    Ok(proof)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    tracing::info!("Starting EigenDA Sidecar");
    let args = Args::parse();
    let sidecar_url = args.sidecar_url.clone();
    let database_url = args.database_url.clone();
    let metrics_url = args.metrics_url.clone();

    let db_pool = PgPool::connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;
    let db_pool = Arc::new(Mutex::new(db_pool));

    let srs = SRS::new("resources/g1.point", SRS_ORDER, SRS_POINTS_TO_LOAD)?;

    let payload_form = match args.payload_form {
        PolynomialForm::Eval => PayloadForm::Eval,
        PolynomialForm::Coeff => PayloadForm::Coeff,
    };

    let payload_disperser_config = PayloadDisperserConfig {
        polynomial_form: payload_form,
        blob_version: args.blob_version,
        cert_verifier_router_address: args.eigenda_cert_verifier_router_addr.to_string(),
        eth_rpc_url: SecretUrl::new(args.rpc_url.clone()),
        disperser_rpc: args.disperser_rpc,
        use_secure_grpc_flag: true,
        registry_coordinator_addr: args.registry_coordinator_addr,
        operator_state_retriever_addr: args.operator_state_retriever_addr
    };
    let private_key = args
        .disperser_private_key
        .expose_secret()
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;
    let signer = Signer::new(private_key);
    let payload_disperser = Arc::new(
        PayloadDisperser::new(payload_disperser_config, signer.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Eigen client Error: {:?}", e))?,
    );

    let db_pool_clone = db_pool.clone();
    let payload_disperser_clone = payload_disperser.clone();
    let proof_gen_thread: JoinHandle<Result<()>> = tokio::spawn(async move {
        let payload_form = match args.payload_form {
            PolynomialForm::Eval => PayloadForm::Eval,
            PolynomialForm::Coeff => PayloadForm::Coeff,
        };

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

        let relay_client = RelayClient::new(relay_client_config).await?;
        let retriever = Arc::new(Mutex::new(RelayPayloadRetriever::new(
            retriever_config,
            srs_config,
            relay_client,
        )?));

        let rpc_url = args.rpc_url.clone();
        let cert_verifier_router_addr = args.eigenda_cert_verifier_router_addr;

        let db_pool = db_pool.clone();
        loop {
            let blob_id = match retrieve_next_pending_proof(db_pool.clone()).await {
                Ok(Some(blob_id)) => blob_id,
                Ok(None) => {
                    tracing::debug!("No pending proofs found");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
                Err(e) => {
                    tracing::error!("Error retrieving pending proof: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            tracing::info!("Proof generation thread: retrieved request to prove: {}", blob_id);

            let timer = PROOF_GEN_TIME_HISTOGRAM
                .with_label_values(&[&blob_id])
                .start_timer();

            match generate_proof(
                blob_id.clone(),
                payload_disperser.clone(),
                retriever.clone(),
                &srs,
                rpc_url.clone(),
                cert_verifier_router_addr,
                payload_form,
            )
            .await
            {
                Ok(proof) => {
                    tracing::info!("Proof gen thread: generated proof for Blob Id {}", blob_id);
                    // Persist proof in database
                    store_blob_proof(db_pool.clone(), blob_id, hex::encode(proof)).await?;
                    PROOF_GEN_SUCCESS_COUNTER.inc();
                }
                Err(e) => {
                    tracing::error!(
                        "Proof gen thread: error generating proof for Blob Id: {}, error: {}",
                        blob_id, e
                    );
                    // Mark the proof request as invalid in the database
                    mark_blob_proof_request_failed(db_pool.clone(), blob_id.clone()).await?;
                    PROOF_GEN_FAILURE_COUNTER.inc();
                }
            };

            timer.observe_duration();
        }
    });

    let json_rpc_server_thread: JoinHandle<Result<()>> = tokio::spawn(async move {
        let mut io = IoHandler::new();
        let db_pool = db_pool_clone.clone();
        let payload_disperser = payload_disperser_clone.clone();
        io.add_method("generate_proof", move |params: Params| {
            let db_pool = db_pool.clone();
            let payload_disperser = payload_disperser.clone();
            async move {
                PROOF_GEN_REQ_COUNTER.inc();

                let parsed: GenerateProofParams = params.parse().map_err(|_| {
                    jsonrpc_core::Error::invalid_params(
                        "Expected a single string parameter 'blob_id'",
                    )
                })?;
                let blob_id = parsed.blob_id;
                tracing::info!("Received request to generate proof for Blob Id {}", blob_id);

                let blob_key = BlobKey::from_hex(&blob_id)
                    .map_err(|_| jsonrpc_core::Error::invalid_params("Invalid blob ID"))?;
                if payload_disperser
                    .get_cert(&blob_key)
                    .await
                    .is_err()
                {
                    return Err(jsonrpc_core::Error::invalid_params(
                        "Blob ID not found in EigenDA",
                    ));
                }

                if proof_request_exists(db_pool.clone(), blob_id.clone())
                    .await
                    .map_err(|_| {
                        tracing::error!(
                            "Failed checking if Blob Id {} already has a proof request",
                            blob_id
                        );
                        jsonrpc_core::Error::internal_error()
                    })?
                {
                    return Err(jsonrpc_core::Error::invalid_params(
                        "Blob ID already submitted",
                    ));
                }

                // Persist request in database
                store_blob_proof_request(db_pool.clone(), blob_id.clone())
                    .await
                    .map_err(|_| {
                        tracing::error!("Failed sending Blob Id {} to prover thread", blob_id);
                        jsonrpc_core::Error::internal_error()
                    })?;

                Ok(jsonrpc_core::Value::String(format!(
                    "Generating Proof for {}",
                    blob_id
                )))
            }
        });

        let db_pool = db_pool_clone.clone();
        io.add_method("get_proof", move |params: Params| {
            PROOF_RET_REQ_COUNTER.inc();

            let db_pool = db_pool.clone();
            async move {
                let parsed: GenerateProofParams = params.parse().map_err(|_| {
                    jsonrpc_core::Error::invalid_params(
                        "Expected a single string parameter 'blob_id'",
                    )
                })?;

                let blob_id = parsed.blob_id;
                tracing::info!("Received request to get proof for Blob Id {}", blob_id);
                match retrieve_blob_id_proof(db_pool.clone(), blob_id.clone()).await {
                    Err(_) | Ok(None) => {
                        tracing::debug!("Proof for Blob ID {} not found", blob_id);
                        Err(jsonrpc_core::Error::internal_error())
                    }
                    Ok(Some((proof, failed))) => {
                        if failed {
                            return Err(jsonrpc_core::Error::invalid_params(
                                "Proof request for Blob ID was not valid",
                            ));
                        }

                        match proof {
                            None => {
                                tracing::debug!("Proof for Blob ID {} not found (still queued)", blob_id);
                                Err(jsonrpc_core::Error{code: ErrorCode::ServerError(PROOF_NOT_FOUND_ERROR), message:"Proof not found (still queued)".to_string(), data: None})
                            }
                            Some(proof) => Ok(jsonrpc_core::Value::String(proof)),
                        }
                    }
                }
            }
        });

        let server = ServerBuilder::new(io)
            .start_http(&sidecar_url.clone().parse()?)
            .expect("Unable to start server");
        tracing::info!("Running JSON RPC server");
        server.wait();
        Ok(())
    });

    let metrics_server_thread = tokio::spawn(async move {
        tracing::info!("Starting metrics server on port 9100");
        let server = MetricsServer::http(metrics_url).map_err(|_| anyhow::anyhow!("Failed to start metrics server"))?;
        for request in server.incoming_requests() {
            if request.url() == "/metrics" {
                let encoder = prometheus::TextEncoder::new();
                let metric_families = prometheus::gather();
                let mut buffer = Vec::new();
                encoder.encode(&metric_families, &mut buffer)?;
    
                let content_type = Header::from_bytes(
                    &b"Content-Type"[..],
                    &b"text/plain; version=0.0.4"[..],
                ).map_err(|_| anyhow::anyhow!("Failed parsing header"))?;
    
                let response = Response::from_data(buffer).with_header(content_type);
                let _ = request.respond(response);
            } else {
                let response = Response::from_string("Not Found").with_status_code(404);
                let _ = request.respond(response);
            }
        }
        Ok(())
    });

    match tokio::try_join!(flatten(proof_gen_thread), flatten(json_rpc_server_thread), flatten(metrics_server_thread)) {
        Ok(_) => {
            tracing::info!("Threads finished successfully");
        }
        Err(e) => {
            tracing::error!("Error in threads: {:?}", e);
        }
    }
    Ok(())
}
