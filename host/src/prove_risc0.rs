use alloy::network::EthereumWallet;
use alloy::{
    primitives::{Address, Bytes, B256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol,
};
use blob_verification_methods::BLOB_VERIFICATION_GUEST_ELF;
use risc0_zkvm::ProveInfo;
use risc0_zkvm::{compute_image_id, sha::Digestible};
use secrecy::{ExposeSecret, Secret};

sol!(
    #[sol(rpc)]
    interface IRiscZeroVerifier {
        function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest) external;
    }
);

pub async fn prove_risc0_proof(
    session_info: ProveInfo,
    guest_elf: &[u8],
    private_key: Secret<String>,
    blob_index: u32,
    proof_verifier_rpc: Secret<String>,
) -> anyhow::Result<()> {
    let image_id = compute_image_id(guest_elf)?;
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
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(proof_verifier_rpc.expose_secret().parse()?);

    let contract_address: Address = "0x25b0F3F5434924821Ad73Eed8C7D81Db87DB0a15"
        .parse()
        .expect("Invalid contract address");

    let contract = IRiscZeroVerifier::new(contract_address, &provider);

    let pending_tx = contract
        .verify(
            Bytes::from(block_proof),
            B256::from_slice(&image_id),
            B256::from_slice(&journal_digest),
        )
        .send()
        .await?;

    let receipt = pending_tx.get_receipt().await?;

    println!(
        "Proof of data inclusion for blob {} verified on L1. Tx hash: {}",
        blob_index, receipt.transaction_hash,
    );

    Ok(())
}
