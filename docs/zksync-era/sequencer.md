# ZKSYNC-ERA

Current [PR](https://github.com/lambdaclass/zksync-era/pull/414)

This PR adds a new `PubdataType` (`EigenDAV2Secure`) due to the need of selecting the EigenDA L1 and L2 Validator contracts:

```rust
pub enum PubdataType {
    Celestia,
    Eigen,
    ObjectStore,
    EigenDAV2Secure,
}

impl FromStr for PubdataType {
    "Avail" => Ok(Self::Avail),
    "Celestia" => Ok(Self::Celestia),
    "Eigen" => Ok(Self::Eigen),
    "EigenDAV2Secure" => Ok(Self::EigenDAV2Secure),
    "ObjectStore" => Ok(Self::ObjectStore),
    _ => Err("Incorrect DA client type; expected one of `Rollup`, `NoDA`, `Avail`, `Celestia`, `Eigen`, `EigenDAV2Secure`, `ObjectStore`"),
}
```

It adds `eigenda_sidecar_rpc`, a new parameter to the config, which is the rpc used to communicate with the sidecar and `version` which specifies whether it uses Insecure or Secure V2:

```rust
pub struct EigenDAConfig {
    /// Custom quorum numbers
    #[config(default, with = Delimited(","))]
    pub custom_quorum_numbers: Vec<u8>,
    // V2 and V2Secure specific fields
    //
    /// Address of the EigenDA cert verifier
    pub cert_verifier_addr: Address,
    /// Polynomial form to disperse the blobs
    #[serde(default)]
    pub polynomial_form: PolynomialForm,
    /// Version of the EigenDA client
    pub version: Version,
    // V2Secure specific fields
    //
    /// URL of the EigenDA Sidecar RPC server
    pub eigenda_sidecar_rpc: String,
}
```

The client is modified for the secure integration, it adds the `send_blob_key` and `get_proof` functions, used to communicate with the sidecar:

```rust
pub struct EigenDAClient {
    client: PayloadDisperser,
    sidecar_client: Client,
    sidecar_rpc: String,
    secure: bool,
}
```

The first one calls the `generate_proof` endpoint on the sidecar, to start generating the proof once the blob is dispersed:

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
            .sidecar_client
            .post(&self.sidecar_rpc)
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
        .sidecar_client
        .post(&self.sidecar_rpc)
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
if self.secure {
    // In V2Secure, we need to send the blob key to the sidecar for proof generation
    self.send_blob_key(blob_key.to_hex())
        .await
        .map_err(to_retriable_da_error)?;
}
```

The `get_proof` function is called on `get_inclusion_data`:

```rust
if self.secure {
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
