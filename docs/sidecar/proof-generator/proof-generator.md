Instantiation:

```rust
let proof_gen_thread: JoinHandle<Result<()>> = tokio::spawn(async move {
    ...
}
```

This thread is the one responsible for picking up proof requests and executing them with risc0. It constantly runs this loop:

1. Query the database for the next `blob_id` to be proven.
2. Request the `certificate` associated with the `blob_id` to a `payload_disperser`, in case it's not ready, it will loop until it is.
3. Generate the `groth16` proof.
4. Store it in the database.

**This is the format of the generated proof:**

```rust
let proof = ethabi::encode(&[Token::Tuple(vec![
    Token::Bytes(block_proof),
    Token::FixedBytes(image_id),
    Token::FixedBytes(journal_digest),
    Token::FixedBytes(output.hash),
])]);
```
