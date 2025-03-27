use alloy::network::EthereumWallet;
use alloy::{
    primitives::{Address, Bytes, B256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol,
};
use risc0_zkvm::ProveInfo;
use risc0_zkvm::{compute_image_id, sha::Digestible};
use secrecy::{ExposeSecret, Secret};
use url::Url;

sol!(
    #[sol(rpc)]
    interface IEigenDARegistry {
        function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest, bytes32 eigendaHash, bytes calldata inclusionData) external;
    }
);

pub async fn prove_risc0_proof(
    session_info: ProveInfo,
    guest_elf: &[u8],
    private_key: Secret<String>,
    proof_verifier_rpc: Url,
    eigenda_registry_addr: String,
    eigenda_hash: Vec<u8>,
    inclusion_data: Vec<u8>,
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

    let pk = private_key.expose_secret();
    let pk = "0x".to_owned() + pk.strip_prefix("0x").unwrap_or(pk);
    let signer: PrivateKeySigner = pk.parse()?;
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(proof_verifier_rpc);

    let eigenda_registry_addr: Address = eigenda_registry_addr
        .parse()
        .expect("Invalid contract address");

    let contract = IEigenDARegistry::new(eigenda_registry_addr, &provider);

    let pending_tx = contract
        .verify(
            Bytes::from(block_proof),
            B256::from_slice(&image_id),
            B256::from_slice(&journal_digest),
            B256::from_slice(&eigenda_hash),
            Bytes::from(inclusion_data.clone()),
        )
        .send()
        .await?;

    let receipt = pending_tx.get_receipt().await?;

    println!(
        "Proof of data inclusion for batch with inclusion data {} verified on L1. Tx hash: {}",
        hex::encode(inclusion_data),
        receipt.transaction_hash,
    );

    Ok(())
}
