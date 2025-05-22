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

use std::{net::SocketAddr, sync::Arc, time::Duration};

use alloy_primitives::Address;
use anyhow::Result;
use clap::Parser;
use common::{output::Output, polynomial_form::PolynomialForm};
use ethabi::{ethereum_types::H160, Token};
use host::db::{
    proof_request_exists, retrieve_blob_id_proof, retrieve_next_pending_proof, store_blob_proof,
    store_blob_proof_request,
};
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

use rust_kzg_bn254_prover::srs::SRS;
use url::Url;
use tokio::sync::watch;
use jsonrpsee::{server::{RpcModule, Server as RPCServer}, types::ErrorObject};


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
    /// URL where this sidecar should run
    #[arg(short, long, env = "SIDECAR_URL")]
    sidecar_url: String,
    /// URL of the database
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,
}

const SRS_ORDER: u32 = 268435456;
const SRS_POINTS_TO_LOAD: u32 = 1024 * 1024 * 2 / 32;

#[derive(Deserialize)]
struct GenerateProofParams {
    blob_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let args = Args::parse();
    let sidecar_url = args.sidecar_url.clone();
    let database_url = args.database_url.clone();

    let db_pool = PgPool::connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;
    let db_pool = Arc::new(Mutex::new(db_pool));

    let srs = SRS::new("resources/g1.point", SRS_ORDER, SRS_POINTS_TO_LOAD)?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let db_pool_clone = db_pool.clone();
    let mut proof_gen_thread: JoinHandle<Result<()>> = tokio::spawn(async move {
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
        let db_pool = db_pool.clone();
        loop {
            let blob_id = match retrieve_next_pending_proof(db_pool.clone()).await {
                Ok(Some(blob_id)) => blob_id,
                Ok(None) => {
                    println!("No pending proofs found");
                    continue;
                }
                Err(e) => {
                    println!("Error retrieving pending proof: {}", e);
                    continue;
                }
            };

            println!("Proof gen thread: retrieved request to prove: {}", blob_id);

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

            let image_id = compute_image_id(GUEST_ELF)?;
            let image_id: risc0_zkvm::sha::Digest = image_id.into();
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

            println!(
                "Proof gen thread: finished generating proof for Blob Id {}",
                blob_id
            );

            // Persist proof in database
            store_blob_proof(db_pool.clone(), blob_id, hex::encode(proof)).await?;
        }
    });

    let db_pool_clone = db_pool_clone.clone();
    let mut json_rpc_server_thread: JoinHandle<Result<()>> = tokio::spawn(async move {
        let server = RPCServer::builder().build(sidecar_url.parse::<SocketAddr>()?).await?;
        let db_pool = db_pool_clone.clone();
        let mut module = RpcModule::new(());
        module.register_async_method("generate_proof", move |params, _ctx, _| {
            let db_pool = db_pool.clone();
            async move {
                let parsed: GenerateProofParams = params.parse().map_err(|_| {
                    ErrorObject::owned(-32602, "Invalid params", Some("Expected 'blob_id'"))
                })?;
                let blob_id = parsed.blob_id;

                if proof_request_exists(db_pool.clone(), blob_id.clone())
                    .await
                    .map_err(|_| {
                        ErrorObject::owned(-32000, "Internal error", Some("Failed checking blob"))
                    })?
                {
                    return Err(
                        ErrorObject::owned(
                            -32000,
                            "Conflict",
                            Some("Blob ID already submitted")
                    ));
                }

                // Persist request in database
                store_blob_proof_request(db_pool.clone(), blob_id.clone())
                    .await
                    .map_err(|_| {
                        ErrorObject::owned(
                            -32000,
                            "Internal error",
                            Some("Failed to store blob proof request"),
                        )
                    })?;

                Ok(format!(
                    "Generating Proof for {}",
                    blob_id
                ))
            }
        })?;
        let db_pool = db_pool_clone.clone();
        module.register_async_method("get_proof", move |params, _ctx, _| {
            let db_pool = db_pool.clone();
            async move {
                let parsed: GenerateProofParams = params.parse().map_err(|_| {
                    ErrorObject::owned(-32602, "Invalid params", Some("Expected 'blob_id'"))
                })?;

                let blob_id = parsed.blob_id;
                match retrieve_blob_id_proof(db_pool.clone(), blob_id.clone()).await {
                    Some(proof) => return Ok(proof),
                    None => {
                        println!("Proof for Blob ID {} not found", blob_id);
                        Err(ErrorObject::owned(
                            -32000,
                            "Proof not found",
                            Some(format!("Proof for Blob ID {} not found", blob_id))))
                    }
                }
            }
        })?;
        let handle = server.start(module);
        let handle_for_shutdown = handle.clone();
        tokio::select! {
            _ = handle.stopped() => {
                println!("Server has stopped.");
            }
            _ = shutdown_rx.changed() => {
                println!("Shutting down JSON-RPC server...");
                handle_for_shutdown.stop()?;
            }
        }
        Ok(())
    });

    tokio::select! {
        res = &mut proof_gen_thread => {
            match res {
                Ok(Ok(_)) => println!("Proof generation finished."),
                Ok(Err(e)) => {
                    println!("Error in proof generation: {:?}", e);
                    shutdown_tx.send(true)?;
                }
                Err(e) => {
                    println!("Error in proof generation: {:?}", e);
                    shutdown_tx.send(true)?;
                }
            }
        }
        res = &mut json_rpc_server_thread => {
            match res {
                Ok(Ok(_)) => println!("JSON-RPC server finished."),
                Ok(Err(e)) => {
                    println!("Error in JSON-RPC server: {:?}", e);
                    proof_gen_thread.abort();
                }
                Err(e) => {
                    println!("Error in JSON-RPC server: {:?}", e);
                    proof_gen_thread.abort();
                }
            }
        }
    }
    Ok(())
}
