use alloy::network::EthereumWallet;
use alloy::providers::fillers::{JoinFill, RecommendedFillers};
use alloy::providers::Identity;
use alloy::transports::http::Client;
use blob_verification_methods::BLOB_VERIFICATION_GUEST_ELF;
use risc0_zkvm::ProveInfo;
use risc0_zkvm::{compute_image_id, sha::Digestible};
use secrecy::{ExposeSecret, Secret};
use alloy::{
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    network::Ethereum,
    rpc::types::eth::TransactionRequest,
    primitives::{Address, Bytes, B256},
    transports::http::{Http, reqwest::ClientBuilder},
    contract::SolCallBuilder,
    sol
};

sol!(
    #[sol(rpc)]
    interface IRiscZeroVerifier {
        function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest) external;
    }
);

pub async fn prove_risc0_proof(
    session_info: ProveInfo,
    private_key: Secret<String>,
    blob_index: u32,
    chain_id: String,
    proof_verifier_rpc: Secret<String>,
) -> anyhow::Result<()> {
    let image_id = compute_image_id(BLOB_VERIFICATION_GUEST_ELF)?;
    let image_id: risc0_zkvm::sha::Digest = image_id.into();
    let image_id = image_id.as_bytes().to_vec();

    let block_proof = match session_info.receipt.inner.groth16() {
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

    let journal_digest = Digestible::digest(&session_info.receipt.journal)
        .as_bytes()
        .to_vec();

    let signer: PrivateKeySigner = private_key.expose_secret().parse()?;
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new().wallet(wallet).on_http(proof_verifier_rpc.expose_secret().parse()?);


    let contract_address: Address = "0x25b0F3F5434924821Ad73Eed8C7D81Db87DB0a15"
        .parse()
        .expect("Invalid contract address");

    let contract = IRiscZeroVerifier::new(contract_address, &provider);

    let pending_tx = contract.verify(Bytes::from(block_proof), B256::from_slice(&image_id), B256::from_slice(&journal_digest))
        .send()
        .await?;

    println!("Transaction submitted: {}", pending_tx.tx_hash());

    let receipt = pending_tx.get_receipt().await?;

    println!(
        "Transaction confirmed in block: {:?}",
        receipt.block_number
    );

    Ok(())
    /*let output = std::process::Command::new("forge")
        .arg("script")
        .arg("contracts/script/Risc0ProofVerifier.s.sol:Risc0ProofVerifier")
        .arg("--rpc-url")
        .arg(proof_verifier_rpc.expose_secret())
        .arg("--broadcast")
        .arg("-vvvv")
        .env("PRIVATE_KEY", private_key.expose_secret()) // Set environment variable
        .env("SEAL", format!("0x{}", hex::encode(&block_proof))) // Convert seal to hex string
        .env("IMAGE_ID", format!("0x{}", hex::encode(&image_id))) // Convert image ID to hex string
        .env(
            "JOURNAL_DIGEST",
            format!("0x{}", hex::encode(&journal_digest)),
        ) // Convert journal digest to hex string
        .output()?;

    if output.status.success() {
        // Extract the transaction hash
        let path = format!(
            "./broadcast/Risc0ProofVerifier.s.sol/{}/run-latest.json",
            chain_id
        );
        let path = std::path::Path::new(&path);

        // Read the JSON file
        let data = std::fs::read_to_string(path)?;

        // Parse the JSON content
        let json: serde_json::Value = serde_json::from_str(&data)?;

        // Extract the transaction hash from "transactions" array
        let transactions =
            json.get("transactions")
                .and_then(|t| t.as_array())
                .ok_or(anyhow::anyhow!(
                    "Invalid JSON structure: 'transactions' not found or not an array"
                ))?;
        let first_transaction = transactions.first().ok_or(anyhow::anyhow!(
            "Invalid JSON structure: 'transactions' array is empty"
        ))?;
        let tx_hash = first_transaction
            .get("hash")
            .and_then(|h| h.as_str())
            .ok_or(anyhow::anyhow!(
                "Invalid JSON structure: 'hash' not found or not a string"
            ))?;
        println!(
            "Proof of data inclusion for blob {} verified on L1. Tx hash: {tx_hash}",
            blob_index
        );
        return Ok(());
    } else {
        println!(
            "Proof verification failed: {:?}",
            std::str::from_utf8(&output.stderr).unwrap()
        );
        return Err(anyhow::anyhow!("Proof Verification failed"));
    }*/
}
