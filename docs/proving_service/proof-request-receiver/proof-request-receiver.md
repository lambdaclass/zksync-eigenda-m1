# PROOF REQUEST RECEIVER

Instantiation:

```rust
let json_rpc_server_thread: JoinHandle<Result<()>> = tokio::spawn(async move {
    ...
}
```

**The rpc thread listens on two `json_rpc` endpoints:**

### `generate_proof`:

This endpoint is called from the `zksync-era` sequencer, it receives a `blob_id` that needs proving, it then checks:

1. The `blob_id` is a valid hex.
2. The `blob_id` belongs to `EigenDA` (it has an associated certificate).
3. The proof request hasn't already been submitted.

If the requests passes all this checks, then the proof request is stored in the postgres database as a pending proof, to be then picked up by the [Proof generator](../proof-generator/proof-generator.md).

**Sample request:**

```bash
curl -X POST "$PROVING_SERVICE_URL" -H "Content-Type: application/json" -d \
'{"jsonrpc":"2.0","method":"generate_proof","params": { "blob_id": "b2ce5a5d0e9b9c699de14aa2924336afa0645b0a5920afd9aff077d831d1299e" },"id":1}'
```

### `get_proof`:

The other endpoint that the server listens to is used to retrieve proofs once they are finished. it also receives a `blob_id`, and may return:

- **`jsonrpc_core::Error::internal_error`**: if the `blob_id` is not found on the database (it was never submitted for proving).
- **`jsonrpc_core::Error::invalid_params`**: if the proof generation for the given `blob_id` failed.
- **`jsonrpc_core::Error{code: ErrorCode::ServerError(PROOF_NOT_FOUND_ERROR), message:"Proof not found (still queued)".to_string(), data: None}`**: if the proof generation for the given `blob_id` is still running or queued.
- **`jsonrpc_core::Value::String(proof)`**: if the `blob_id` already has its proof generated and stored in the database.

**Sample request:**

```bash
curl -X POST "$PROVING_SERVICE_URL" -H "Content-Type: application/json" -d \
'{"jsonrpc":"2.0","method":"get_proof","params": { "blob_id": "b2ce5a5d0e9b9c699de14aa2924336afa0645b0a5920afd9aff077d831d1299e" },"id":1}'
```
