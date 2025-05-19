# ZKSYNC-EIGENDA M1

**Warning: This sidecar only works on a x86 machine with cuda support**

**The EigenDA sidecar where risc0-steel is used in order to generate a proof for the call of the VerifyDACertV2 function of EigenDA's CertVerifier contract, which performs the necessary checks to make sure a given blob is present.**
**As well as performing the proof of equivalence verifying a proof that the EigenDA commitment commits to the given Blob.**
**The sidecar consists of 2 Endpoints:**
**generate_proof: Which given the blobKey begins the proof generation process**
**get_proof: Which given the blobKey it returns the generated proof or an error in case it hasn't finished**

## Prerequisites

To get started, you need to have Rust installed.

Next, you will also need to have the [`cargo-risczero`](https://dev.risczero.com/api/zkvm/install) tool installed.

Next we need to install cuda 12.6

Install [cuda](https://developer.nvidia.com/cuda-downloads?target_os=Linux&target_arch=x86_64&Distribution=Debian&target_version=12&target_type=runfile_local).
Use the runfile (local) option, use the wget shown to download the script and run it as:

```bash
sudo ./<file>.run
```

## Run the sidecar

### Deployment steps (On this repo):

Compile the contracts

```bash
git submodule update --init --recursive
make build_contracts
```

Export the needed variables (rpcs should have http://, private keys and addresses should have 0x)

```bash
export PRIVATE_KEY=<your_private_key> #The private key you want to use to deploy contracts
export DISPERSER_PRIVATE_KEY=<your_disperser_private_key> #The private key you want to use with the eigenda disperser
export CERT_VERIFIER_ADDR=<your_cert_verifier_address> #Contract that has the VerifyDACertV2 function
export RPC_URL=<your_rpc> #RPC URL of your node
export DISPERSER_RPC=<your_rpc> #RPC of the eigenda disperser
export PAYLOAD_FORM=<your_payload_form> #Either coeff or eval (On EigenDA Holesky use coeff)
export BLOB_VERSION=0 #Blob version used by EigenDA
export EIGENDA_RELAY_REGISTRY_ADDR=<your_relay_registry_addr> #Address of the EigenDA relay registry
export RELAY_CLIENT_KEYS=<your_relay_client_keys> #Keys of the relay client, separated by commas ("0,1,2")
export SIDECAR_URL=<your_sidecar_url> #URL you want this sidecar to run on
```

Deploy the contracts:

```bash
forge script contracts/script/ContractsDeployer.s.sol:ContractsDeployer --rpc-url $RPC_URL --broadcast -vvvv
```

Save the address under `EigenDACertVerifierWrapper deployed at: <address>`
Save the address under `RiscZeroVerifier deployed at: <address>`

```bash
export CERT_VERIFIER_WRAPPER_ADDR=<your_address>
export RISC_ZERO_VERIFIER_ADDR=<you_address>
```

### Run the sidecar (On this repo)

```bash
RUST_LOG=info cargo run --release
```

### Run zksync-era (eigenda-v2-m1 branch on lambdaclass fork):

Install zkstack:

```bash
cd ./zkstack_cli/zkstackup
./install --local
```

On `zksync-era/zkstack_cli/crates/types/src/l1_network.rs`

Modify the address for `risc_zero_verifier` for your address (the one under `RISC_ZERO_VERIFIER_ADDR` env variable).

Reload your terminal, and run on zksync-era root:

```bash
zkstackup --local
```

Install `foundry-zksync` `0.0.2`:

```
curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash
foundryup-zksync --commit 27360d4c8d12beddbb730dae07ad33a206b38f4b
```

Modify `etc/env/file_based/overrides/validium.yaml`:

```yaml
da_client:
  eigenv2m1:
    disperser_rpc: <your_disperser_rpc> #Under DISPERSER_RPC env variable
    eigenda_eth_rpc: <your_eth_rpc> #Under RPC_URL env variable
    authenticated: true
    cert_verifier_addr: <your_cert_verifier_address> #Under CERT_VERIFIER_ADDRESS env variable
    blob_version: <your_blob_version> #Under BLOB_VERSION env variable
    polynomial_form: <your_polynomial_form> #Either COEFF or EVAL
    eigenda_sidecar_rpc: <your_sidecar_rpc> #Under SIDECAR_URL env variable
```

Modify `etc/env/file_based/secrets.yaml`:

```yaml
da:
  eigenv2m1:
    private_key: <your_private_key> #The private key you want to use with the eigenda disperser
```

Run replacing with your l1 rpc:

```bash
zkstack containers --observability true

zkstack chain create \
          --chain-name eigenda \
          --chain-id sequential \
          --prover-mode no-proofs \
          --wallet-creation localhost \
          --l1-batch-commit-data-generator-mode validium \
          --base-token-address 0x0000000000000000000000000000000000000001 \
          --base-token-price-nominator 1 \
          --base-token-price-denominator 1 \
          --set-as-default false

zkstack ecosystem init \
          --deploy-paymaster true \
          --deploy-erc20 true \
          --deploy-ecosystem true \
          --l1-rpc-url <your_l1_rpc> \
          --server-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --server-db-name=zksync_server_localhost_eigenda \
          --chain eigenda \
          --verbose
```

Then run

```bash
zkstack server --chain eigenda
```

On zksync-era you should see blobs being dispatched:

```
2025-03-27T18:20:05.383060Z  INFO zksync_da_dispatcher::da_dispatcher: Dispatched a DA for batch_number: 1, pubdata_size: 5312, dispatch_latency: 24.480322ms

2025-03-27T18:36:19.150242Z  INFO zksync_da_dispatcher::da_dispatcher: Received an inclusion data for a batch_number: 1, inclusion_latency_seconds: 973

2025-03-27T18:36:23.535623Z  INFO EthTxManager::loop_iteration: zksync_eth_sender::eth_tx_manager: Checking tx id: 1, operator_nonce: OperatorNonce { finalized: Nonce(1), latest: Nonce(1) }, tx nonce: 0

2025-03-27T18:36:23.540535Z  INFO EthTxManager::loop_iteration: zksync_eth_sender::eth_tx_manager: eth_tx 1 with hash 0xbe08cdd9ba138548f45c152e1a913784dd5cb2157e2d6323db9fe182aa067e2f for CommitBlocks is confirmed. Gas spent: 267395
```

On the sidecar you should see blobs being verified:

```
Running JSON RPC server
Running proof gen thread
Proof gen thread: received request to prove: bdfef9b13ccd6648534267b80bea88b1b6c75ecfef4468299d32fd646c47c7b9
2025-05-19T15:45:28.038452Z  INFO risc0_steel::host::builder: Environment initialized with block 3861957 (0x2283aafccd1976c63f9dced04f03106629a7b76baccc519c2f6d4bb61ae4b59c)    
2025-05-19T15:45:28.038505Z  INFO risc0_steel::contract::host: Executing preflight calling 'verifyDACertV2((bytes32,uint32),(((uint16,bytes,((uint256,uint256),(uint256[2],uint256[2]),(uint256[2],uint256[2]),uint32),bytes32),bytes,uint32[]),uint32,bytes),(uint32[],(uint256,uint256)[],(uint256,uint256)[],(uint256[2],uint256[2]),(uint256,uint256),uint32[],uint32[],uint32[][]),bytes)' on 0x18c7De1E82513c3F48dFcCa85c64056C637104fb    
Call verifyDACertV2((bytes32,uint32),(((uint16,bytes,((uint256,uint256),(uint256[2],uint256[2]),(uint256[2],uint256[2]),uint32),bytes32),bytes,uint32[]),uint32,bytes),(uint32[],(uint256,uint256)[],(uint256,uint256)[],(uint256[2],uint256[2]),(uint256,uint256),uint32[],uint32[],uint32[][]),bytes) Function on 0x18c7…04fb returns: true
Running the guest with the constructed input...
2025-05-19T15:46:27.505240Z  INFO risc0_zkvm::host::server::exec::executor: execution time: 17.890119705s
Proof gen thread: finished generating proof for Blob Id bdfef9b13ccd6648534267b80bea88b1b6c75ecfef4468299d32fd646c47c7b9
```

## Design

M1 consists of checking the inclusion of the blob and verifying that the data that is committed to is the correct one, this computations would be too heavy/costly to run directly on chain. An offchain implementation is needed in order to prevent this high costs. We resolve this by making a binary capable of running this checks in a provable way.

### Integration

The important components are marked in **bold**

#### Step 1: Sequencer Dispersal and Inclusion data retrieval (Marked in Blue) & Sidecar Proof Generation (Marked in red)

![Step 1](images/step1.png)

1. Zksync's sequencer finishes a batch and wants to disperse its content (**Blob Data**).
2. Zksync's sequencer sends the blob to be dispersed to EigenDA, EigenDA returns the **Blob Key**.
3. Zksync's sequencer sends the **Blob key** to the Sidecar
4. Zksync's sequencer stores the **Blob Key** in its database.
5. Zksync’s sequencer asks for **Inlcusion Data** (encoded **EigenDACert**) to EigenDA
6. Zksync’s sequencer starts waiting for Sidecar **Blob Key** proof to be generated.
7. Sidecar asks EigenDA for **EigenDACert** and **Blob Data** (this step runs parallel to step 4)
8. Sidecar executes Risc0, doing 3 things:

   a. Call to VerifyDACertV2

   b. Proof of Equivalence

   c. Calculation of **EigenDAHash** (keccak of _BlobData_)

   And generates a **Risc0 Proof** of those 3 things.

9. Zksync’s sequencer finishes waiting for proof (step 6), storing the retrieved proof in its database. It then calls the Commit Batches function of Executor (zksync’s DiamondProxy implementation)on Ethereum.

#### Step 2 Commit Batches (Marked in Green)

![Step 2](images/step2.png)

Everything here runs on Ethereum

10. Executor starts commit batches function
11. Executor calls the EigenDAL1Validator checkDA function with **l2DAValidatorOutputHash** and **operatorDAInput** as parameters

    a. **l2DAValidatorOutputHash**: keccak(**stateDiffHash** + **EigenDAHash**)

    b. **operatorDAInput**: **StateDiffHash** + **Inclusion Data** (seal + imageId + journalDigest + eigenDAHash)

    - **stateDiffHash** is the hash of the states diffs, calculated on EigenDAL2Validator and sent to L1 through L2→L1 Logs

12. EigenDAL1Validator calls risc0Verifier `verify` function with **inclusionData.seal**, **inclusionData.imageId**, **inclusionData.journalDigest** as parameters, which is expected to **not revert** upon succesful verification.
13. EigenDAL1Validator checks if keccak(**stateDiffHash** + **inclusionData.EigenDAHash**) equals **l2DAValidatorOutputHash** (meaning that if not, **EigenDAHash** was not correctly calculated by the sidecar)

### What does the Guest do?

There are 3 things we want to achieve with the Risc0 guest. Each one of them is addressed in point 8.

1. We want to check that the blob is available in EigenDA. 8.a
2. We want to check that the commitment commits to that blob. 8.b
3. We want to check that the blob is the same we dispersed on zksync. 8.c

# TODO: Update to V2

#### \* Call to verifyDACertV2 (8.a)

On the host:

Inputs: `rpc_url`, `cert_verifier_wrapper_addr`

```rust
    let call = IVerifyBlob::verifyDACertV2Call {
        batchHeader: eigenda_cert.batch_header.clone().into(),
        blobInclusionInfo: eigenda_cert.blob_inclusion_info.clone().into(),
        nonSignerStakesAndSignature: eigenda_cert.non_signer_stakes_and_signature.clone().into(),
        signedQuorumNumbers: eigenda_cert.signed_quorum_numbers.clone().into(),
    };

    // Create an EVM environment from an RPC endpoint defaulting to the latest block.
    let mut env = EthEvmEnv::builder().rpc(rpc_url.clone()).build().await?;

    // Preflight the call to prepare the input that is required to execute the function in
    // the guest without RPC access. It also returns the result of the call.
    // Risc0 steel creates an ethereum VM using revm, where it simulates the call to verifyDACertV2.
    // So we need to make this preflight call to populate the VM environment with the current state of the chain
    let mut contract = Contract::preflight(cert_verifier_wrapper_addr, &mut env);
    let returns = contract.call_builder(&call).call().await?;

    // Finally, construct the input from the environment.
    let input = env.into_input().await?;
```

We make the preflight call to CertVerifierWrapper to populate the EvmEnv

Output: input (EthEvmInput type)

On the guest:

Inputs: input

```rust
    // Read the input from the guest environment.
    let input: EthEvmInput = env::read();

    // Converts the input into a `EvmEnv` for execution.
    let env = input.into_env();

    // Execute the view call; it returns the result in the type generated by the `sol!` macro.
    let contract = Contract::new(cert_verifier_wrapper_addr, &env);
    let call = IVerifyBlob::verifyDACertV2Call {
        batchHeader: eigenda_cert.batch_header.clone().into(),
        blobInclusionInfo: eigenda_cert.blob_inclusion_info.clone().into(),
        nonSignerStakesAndSignature: eigenda_cert.non_signer_stakes_and_signature.clone().into(),
        signedQuorumNumbers: eigenda_cert.signed_quorum_numbers.clone().into(),
    };
    let returns = contract.call_builder(&call).call();
    // Here we assert that the result of the verifyDACertV2 call is true, meaning it executed correctly
    assert!(returns._0);
```

Here we make the call to verifyBlobV2 inside the risc0 steel VM.

What risc0 steel does is, given the env it generates a revm VM with the given state of the chain.

And it simulates the call to the contract.

We then assert the result of that call being true.

Outputs: **Risc0Proof**

#### \* Proof Of Equivalence (8.b)

On the host:

Inputs: **EigenDACert, BlobData,** SRSPoints

```rust
    let blob = Blob::new(&encoded_data);

    let mut kzg = KZG::new();

    kzg.calculate_and_store_roots_of_unity(blob.len().try_into()?)?;

    let cert_commitment = eigenda_cert
        .blob_inclusion_info
        .blob_certificate
        .blob_header
        .commitment
        .commitment;
```

First we obtain the commitment from the EigenDACert’s blob header:

```rust
    // Calculate the polynomial in evaluation form
    let poly_coeff = blob.to_polynomial_coeff_form();
    let poly_eval = poly_coeff.to_eval_form()?;

    let evaluation_challenge = compute_challenge(&blob, &cert_commitment)?;

    // Compute the proof that the commitment corresponds to the given blob
    let proof = kzg.compute_proof(&poly_eval, &evaluation_challenge, &srs)?;
```

We then calculate the eval polynomial, the evaluation challenge and the proof for that commitment

Output: Proof

Guest:

Inputs: **BlobData**, Proof, EigenDACert (commitment)

```rust
    // Calculate the polynomial in evaluation form
    let poly_coeff = blob.to_polynomial_coeff_form();
    let poly_eval = poly_coeff.to_eval_form().unwrap();

    // Get the commitment from eigenda cert
    let cert_commitment = eigenda_cert.blob_inclusion_info.blob_certificate.blob_header.commitment.commitment;
    // Compute evaluation challenge
    let evaluation_challenge = compute_challenge(&blob, &cert_commitment).unwrap();

    // Evaluate the polynomial at the evaluation challenge
    let y = evaluate_polynomial_in_evaluation_form(&poly_eval, &evaluation_challenge).unwrap();

    let evaled_y = eval(poly_coeff.coeffs(), evaluation_challenge);

    // Assert that the evaluation of the polynomial at the evaluation challenge is equal to the y value
    assert_eq!(y, evaled_y);

    // Verification of the kzg proof for the given commitment, evaluation and evaluation challenge
    let verified = verify_proof(cert_commitment, proof.g1, y, evaluation_challenge).unwrap();
    assert!(verified);
```

We recalculate the evaluation polynomial, the cert commitment from the blob info and the evaluation challenge.
Then we get the evaluation at the challenge point and compare it to the one we calculate using horner's rule.
Then we verify the proof for the commitment at that challenge point

Output: **Risc0Proof**

#### \* EigenDAHash (8.c)

In the guest we also calculate the eigenDAHash

```rust
    // Here we calculate the keccak hash of the data, which we will use on zksync's EigenDAL1Validator to compare it to the hashes there
    let hash = keccak256(&data);

    let mut proof_bytes = vec![];
    proof.g1.serialize_compressed(&mut proof_bytes).unwrap();
    // Public outputs of the guest, eigenDAHash, commitment to the risc0 steel environment, blob info and proof, they are embedded on the risc0 proof
    let output = Output {
        hash: hash.to_vec(),
        env_commitment: env.commitment().abi_encode(),
        inclusion_data: eigenda_cert.to_bytes().unwrap(),
        proof: proof_bytes,
    };
```

And return it as a public output.

We then store this proof on the sidecar database.

Then on zksync’s EigenDAL1Validator, we check the validity of this proof by verifying against the risc0verifier.

The idea of this check is to make sure that the blob we verified on the guest is the same blob we dispersed on zksync.
