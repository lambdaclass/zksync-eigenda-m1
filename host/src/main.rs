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
use tokio::sync::Mutex;
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let srs = SRS::new("resources/g1.point", SRS_ORDER, SRS_POINTS_TO_LOAD)?;

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
    let payload_disperser = Arc::new(Mutex::new(payload_disperser));

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
    let retriever = RelayPayloadRetriever::new(retriever_config, srs_config, relay_client)?;
    let retriever = Arc::new(Mutex::new(retriever));

    let payload_disperser_clone = Arc::clone(&payload_disperser);
    let retriever_clone = Arc::clone(&retriever);
    let srs_clone = srs.clone();
    let rpc_url_clone = args.rpc_url.clone();
    let cert_verifier_wrapper_addr_clone = args.cert_verifier_wrapper_addr.clone();
    let verification_private_key_clone = args.verification_private_key.clone();
    let eigenda_cert_and_blob_verifier_addr_clone =
        args.eigenda_cert_and_blob_verifier_addr.clone();
    let mut io = IoHandler::new();
    io.add_method("generate_proof", move |params: Params| {
        let payload_disperser_clone = Arc::clone(&payload_disperser_clone);
        let retriever_clone = Arc::clone(&retriever_clone);
        let srs_clone = srs_clone.clone();
        let rpc_url_clone = rpc_url_clone.clone();
        let verification_private_key_clone = verification_private_key_clone.clone();
        let eigenda_cert_and_blob_verifier_addr_clone =
            eigenda_cert_and_blob_verifier_addr_clone.clone();
        async move {
            let parsed: GenerateProofParams = params.parse().map_err(|_| {
                jsonrpc_core::Error::invalid_params("Expected a single string parameter 'blob_id'")
            })?;
            let blob_id = parsed.blob_id;

            let eigenda_cert: EigenDACert;
            loop {
                let blob_key = BlobKey::from_hex(&blob_id).map_err(|_| {
                    jsonrpc_core::Error::invalid_params("Provided 'blob_id' is not valid")
                })?;
                let opt_eigenda_cert = payload_disperser_clone
                    .lock()
                    .await
                    .get_inclusion_data(&blob_key)
                    .await
                    .map_err(|e| {
                        jsonrpc_core::Error::invalid_params_with_details(
                            "Provided 'blob_id' is not valid",
                            e.to_string(),
                        )
                    })?;
                if let Some(opt_eigenda_cert) = opt_eigenda_cert {
                    eigenda_cert = opt_eigenda_cert;
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }

            // Raw bytes dispersed by zksync sequencer to EigenDA
            let payload: Payload = retriever_clone
                .lock()
                .await
                .get_payload(eigenda_cert.clone())
                .await
                .map_err(|e| {
                    jsonrpc_core::Error::invalid_params_with_details(
                        "Provided 'blob_id' is not valid",
                        e.to_string(),
                    )
                })?;

            let blob_data = payload.serialize();

            let result = host::guest_caller::run_guest(
                eigenda_cert.clone(),
                &srs_clone,
                blob_data,
                rpc_url_clone.clone(),
                cert_verifier_wrapper_addr_clone,
                payload_form,
            )
            .await
            .map_err(|_| jsonrpc_core::Error::internal_error())?;

            let output: Output = result
                .receipt
                .journal
                .decode()
                .map_err(|_| jsonrpc_core::Error::internal_error())?;

            host::prove_risc0::prove_risc0_proof(
                result,
                GUEST_ELF,
                verification_private_key_clone,
                rpc_url_clone,
                eigenda_cert_and_blob_verifier_addr_clone,
                output.hash,
                eigenda_cert
                    .to_bytes()
                    .map_err(|_| jsonrpc_core::Error::internal_error())?,
            )
            .await
            .map_err(|_| jsonrpc_core::Error::internal_error())?;

            Ok(jsonrpc_core::Value::String("Done".to_string()))
        }
    });

    let server = ServerBuilder::new(io)
        .start_http(&"127.0.0.1:3030".parse().unwrap()) // TODO: make custom?
        .expect("Unable to start server");
    server.wait();
    Ok(())
}
