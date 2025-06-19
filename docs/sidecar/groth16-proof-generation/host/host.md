# HOST

In order to generate the `RiscZero Groth16 Proof`, we first make the preflight call to `checkDACert` on the host.

```rust
let call = IVerifyBlob::checkDACertCall {
    eigendacert: eigenda_cert.to_abi_encoded()?.into(),
};

// Create an EVM environment from an RPC endpoint defaulting to the latest block.
let mut env = EthEvmEnv::builder()
    .rpc(rpc_url.clone())
    .chain_spec(&ETH_HOLESKY_CHAIN_SPEC)
    .build()
    .await?;

// Preflight the call to prepare the input that is required to execute the function in
// the guest without RPC access. It also returns the result of the call.
// Risc0 steel creates an ethereum VM using revm, where it simulates the call to checkDACert.
// So we need to make this preflight call to populate the VM environment with the current state of the chain
let mut contract = Contract::preflight(cert_verifier_router_addr, &mut env);
let returns = contract.call_builder(&call).call().await?;
tracing::info!(
    "Call {} Function on {:#} returns: {}",
    IVerifyBlob::checkDACertCall::SIGNATURE,
    cert_verifier_router_addr,
    returns
);

// Finally, construct the input from the environment.
let input = env.into_input().await?;
```

The input is then passed to the guest in order for it to have the state of the chain.

In the host we also compute the `KZG Proof` for the Proof of Equivalence:

```rust
let mut kzg = KZG::new();

kzg.calculate_and_store_roots_of_unity(blob.len().try_into()?)?;

let cert_commitment = eigenda_cert
    .blob_inclusion_info
    .blob_certificate
    .blob_header
    .commitment
    .commitment;

// Calculate the polynomial in evaluation form
let poly_coeff = blob.to_polynomial_coeff_form();
let poly_eval = poly_coeff.to_eval_form()?;

let evaluation_challenge = compute_challenge(&blob, &cert_commitment)?;

// Compute the proof that the commitment corresponds to the given blob
let proof = kzg.compute_proof(&poly_eval, &evaluation_challenge, srs)?;
```

We then call the guest which returns the necessary data to return the Proof:

```rust
let session_info = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
    let env = ExecutorEnv::builder()
        .write(&input)?
        .write(&eigenda_cert)?
        .write(&data)?
        .write(&serializable_proof)?
        .write(&cert_verifier_router_addr)?
        .write(&polynomial_form)?
        .build()?;
    let exec = default_prover();
    exec.prove_with_ctx(
        env,
        &VerifierContext::default(),
        GUEST_ELF,
        &ProverOpts::groth16(),
    )
    .context("failed to run executor")
})
.await??;
```
