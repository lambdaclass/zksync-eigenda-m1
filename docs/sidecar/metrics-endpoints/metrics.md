# METRICS

Instantiation:

```rust
let metrics_server_thread = tokio::spawn(async move {
    ...
}
```

The metrics endpoint exposes `prometheus` metrics, that can be used to plot with `graphana`.

The metrics availabe are:

- **proof_requests:** number of [proof generation requests](../proof-request-receiver/proof-request-receiver.md#generate_proof) received.
- **proof_generations:** number of successful proofs generated.
- **proof_generation_failures:** number of failed proof generations.
- **proof_retrievals:** number of [proof retrieval requests](../proof-request-receiver/proof-request-receiver.md#get_proof) received.
- **proof_generation_seconds:** average time taken to generate a proof in seconds.
