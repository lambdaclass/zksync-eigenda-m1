# ZKSYNC-ERA

Current [PR](https://github.com/lambdaclass/zksync-era/pull/414)

It adds `eigenda_prover_service_rpc`, a new parameter to the config, which is the rpc used to communicate with the prover service.

```rust
pub struct EigenConfig {
    /// URL of the Disperser RPC server
    pub disperser_rpc: String,
    /// URL of the Ethereum RPC server
    #[config(secret, with = Optional(Serde![str]))]
    pub eigenda_eth_rpc: Option<SensitiveUrl>,
    /// Address of the EigenDA cert verifier router
    pub cert_verifier_router_addr: String,
    /// Blob version
    pub blob_version: u16,
    /// Address of the operator state retriever
    pub operator_state_retriever_addr: String,
    /// Address of the registry coordinator
    pub registry_coordinator_addr: String,
    /// URL of the EigenDA Prover Service RPC server
    /// This is used for EigenDA V2 Secure integration,
    /// so if its either `None` or `Some` defines whether we are using EigenDA V2 Secure or not.
    pub eigenda_prover_service_rpc: Option<String>,
}
```

The client is modified for the secure integration, it adds the `send_blob_key` and `get_proof` functions, used to communicate with the prover service:

```rust
pub struct EigenDAClient {
    client: PayloadDisperser,
    prover_service_client: Client,
    eigenda_prover_service_rpc: Option<String>,
}
```

The first one calls the `generate_proof` endpoint on the prover service, to start generating the proof once the blob is dispersed:

```rust
impl EigenDAClient {
    async fn send_blob_key(&self, blob_key: String) -> anyhow::Result<()> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": "generate_proof",
            "params": { "blob_id": blob_key },
            "id": 1
        });
        let response = self
            .prover_service_client
            .post(
                self.eigenda_prover_service_rpc
                    .clone()
                    .ok_or(anyhow::anyhow!("Failed to get proving service rpc"))?,
            )
            .json(&body)
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("Failed to send blob key"))?;

        let json_response: Value = response
            .json()
            .await
            .map_err(|_| anyhow::anyhow!("Failed to parse response"))?;

        if json_response.get("error").is_some() {
            Err(anyhow::anyhow!("Failed to send blob key"))
        } else {
            Ok(())
        }
    }
}
```

The second one calls the `get_proof` endpoint, returning `None` if it is still generating it:

```rust
async fn get_proof(&self, blob_key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "get_proof",
        "params": { "blob_id": blob_key },
        "id": 1
    });
    let response = self
        .prover_service_client
        .post(
            self.eigenda_prover_service_rpc
                .clone()
                .ok_or(anyhow::anyhow!("Failed to get proving service rpc"))?,
        )
        .json(&body)
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("Failed to get proof"))?;

    let json_response: Value = response
        .json()
        .await
        .map_err(|_| anyhow::anyhow!("Failed to parse response"))?;

    if let Some(error) = json_response.get("error") {
        if let Some(error_code) = error.get("code") {
            if error_code.as_i64() != Some(PROOF_NOT_FOUND_ERROR_CODE) {
                return Err(anyhow::anyhow!("Failed to get proof for {:?}", blob_key));
            }
        }
    }

    if let Some(result) = json_response.get("result") {
        if let Some(proof) = result.as_str() {
            let proof =
                hex::decode(proof).map_err(|_| anyhow::anyhow!("Failed to parse proof"))?;
            return Ok(Some(proof));
        }
    }

    Ok(None)
}
```

The `send_blob_key` function is called after dispersing the blob to EigenDA:

```rust
// Prover service RPC being set means we are using EigenDA V2 Secure
if self.eigenda_prover_service_rpc.is_some() {
    // In V2Secure, we need to send the blob key to the prover service for proof generation
    self.send_blob_key(blob_key.to_hex())
        .await
        .map_err(to_retriable_da_error)?;
}
```

The `get_proof` function is called on `get_inclusion_data`:

```rust
if let Some(eigenda_cert) = eigenda_cert {
    // Prover Service RPC being set means we are using EigenDA V2 Secure
    if self.eigenda_prover_service_rpc.is_some() {
        if let Some(proof) = self
            .get_proof(blob_id)
            .await
            .map_err(to_non_retriable_da_error)?
        {
            Ok(Some(InclusionData { data: proof }))
        } else {
            Ok(None)
        }
    }
}
```

This PR also adds necessary changes on the zkstack to deploy the new contracts and select this new client.

A new function `eigenda_risc_zero_verifier_addr` is added, where you should put the address of the risc zero verifier you want to use:

```rust
pub fn eigenda_risc_zero_verifier_addr(&self) -> Option<Address> {
    match self {
        L1Network::Localhost => None,
        L1Network::Sepolia | L1Network::Holesky => {
            None
            //TODO: add real address
        }
        L1Network::Mainnet => None, // TODO: add mainnet address after it is known
    }
}
```
