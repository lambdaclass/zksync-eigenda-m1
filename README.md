# ZKSYNC-EIGENDA M1
**Warning: This sidecar only works on a x86 machine with cuda support**

**The EigenDA sidecar where risc0-steel is used in order to generate a proof for the call of the VerifyDACertV2 function of EigenDA's CertVerifier contract, which performs the necessary checks to make sure a given blob is present.**
**As well as performing the proof of equivalence verifying a proof that the EigenDA commitment commits to the given Blob.**
**Finally it sends the Risc0 Proof to verify to the EigenDA Cert and Blob Verifier contract, which stores whether it was correctly verified.**

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
export PRIVATE_KEY=<your_private_key> #The private key you want to use to deploy contracts and call to VerifyDACertV2
export DISPERSER_PRIVATE_KEY=<your_disperser_private_key> #The private key you want to use with the eigenda disperser
export CERT_VERIFIER_ADDR=<your_cert_verifier_address> #Contract that has the VerifyDACertV2 function
export RPC_URL=<your_rpc> #RPC URL of your node
export DISPERSER_RPC=<your_rpc> #RPC of the eigenda disperser
export PAYLOAD_FORM=<your_payload_form> #Either coeff or eval (On EigenDA Holesky use coeff)
export BLOB_VERSION=0 #Blob version used by EigenDA
export EIGENDA_RELAY_REGISTRY_ADDR=<your_relay_registry_addr> #Address of the EigenDA relay registry
```

Deploy the contracts:

```bash
forge script contracts/script/ContractsDeployer.s.sol:ContractsDeployer --rpc-url $RPC_URL --broadcast -vvvv
```

Save the address under `EigenDACertVerifierWrapper deployed at: <address>`
Save the address under `CertAndBlobVerifier Proxy deployed at: <address>`

```bash
export CERT_VERIFIER_WRAPPER_ADDR=<your_address>
export EIGENDA_CERT_AND_BLOB_VERIFIER_ADDR=<your_address>
```

### Run zksync-era (eigenda-v2 branch on lambdaclass fork):

Install zkstack:

```bash
cd ./zkstack_cli/zkstackup
./install --local
```

On `zksync-era/zkstack_cli/crates/types/src/l1_network.rs`

Modify the address for `eigenda_cert_and_blob_verifier` for your address (the one under `EIGENDA_CERT_AND_BLOB_VERIFIER_ADDR` env variable).

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
  eigen:
    disperser_rpc: <your_disperser_rpc> #Under DISPERSER_RPC env variable
    eigenda_eth_rpc: <your_eth_rpc> #Under RPC_URL env variable
    authenticated: true
    cert_verifier_addr: <your_cert_verifier_address> #Under CERT_VERIFIER_ADDRESS env variable
    blob_version: <your_blob_version> #Under BLOB_VERSION env variable
    polynomial_form: <your_polynomial_form> #Either COEFF or EVAL
    eigenda_cert_and_blob_verifier_addr: <your_cert_and_blob_verifier> #Under CERT_AND_BLOB_VERIFIER_ADDR env variable
```

Modify `etc/env/file_based/secrets.yaml`:

```yaml
da:
  eigen:
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

### Run the sidecar (On this repo)

```bash
VERIFICATION_PRIVATE_KEY=$PRIVATE_KEY API_URL=<your_url> START_BATCH=1 RUST_LOG=info cargo run --release
```

For a local server, you can get your api url under `chains/<your_chain>/configs/general.yaml` on the `zksync-era` repository

```yaml
api:
  web3_json_rpc:
    http_url:
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
2025-03-27T18:24:32.719190Z  INFO risc0_steel::host::builder: Environment initialized with block 439 (0x6ee5f4f28b13bff98e6530fd2f0dd961c56873117d5441b6fec033b3b5e4d61e)    
2025-03-27T18:24:32.719231Z  INFO risc0_steel::contract::host: Executing preflight calling 'verifyBlobV1(((uint256,uint256),uint32,(uint8,uint8,uint8,uint32)[]),(uint32,uint32,((bytes32,bytes,bytes,uint32),bytes32,uint32),bytes,bytes))' on 0xc551b009C1CE0b6efD691E23998AEFd4103680D3    
Call verifyBlobV1(((uint256,uint256),uint32,(uint8,uint8,uint8,uint32)[]),(uint32,uint32,((bytes32,bytes,bytes,uint32),bytes32,uint32),bytes,bytes)) Function by 0x1DCc…0b95 on 0xc551…80D3 returns: true
Running the guest with the constructed input...
View call result: true
2025-03-27T18:24:41.214118Z  INFO risc0_zkvm::host::server::exec::executor: execution time: 8.072798649s
Proof of data inclusion for batch with inclusion data 000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000
4000000000000000000000000000000000000000000000000000000000000001e01bcf76a9b990969aef14ab7605786cd41ea68f508ef6158452b2c0e8db11
0e7f097db87c020f660d9d6263c1d238a37de364093b9ddb9dfe325b5b17150ee8860000000000000000000000000000000000000000000000000000000000
0000ac000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000
000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
000000002100000000000000000000000000000000000000000000000000000000000000370000000000000000000000000000000000000000000000000000
000000000001000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000
000000000000210000000000000000000000000000000000000000000000000000000000000037000000000000000000000000000000000000000000000000
000000000000000100000000000000000000000000000000000000000000000000000000000000050000000000000000000000000000000000000000000000
00000000000000001700000000000000000000000000000000000000000000000000000000000000a000000000000000000000000000000000000000000000
000000000000000002c00000000000000000000000000000000000000000000000000000000000000380000000000000000000000000000000000000000000
00000000000000000000a0799f2e238ad4baf63a1b6618c9264dbafee7e931ca88d222c6ed5c01d7ab48bc0000000000000000000000000000000000000000
00000000000000000000018700000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000
000000000000000000000001e016d52386a9df84fb2ccf697cfbf2d951421d1e312acb7d139ce13a648fc3c16c000000000000000000000000000000000000
000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000
00000000000000000000000000015d000000000000000000000000000000000000000000000000000000000000000200010000000000000000000000000000
000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002646400000000000000000000000000
000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000203448a121631dd4c3aac7477e76da
513ff6b61d9637dced5008a0bffb86df69d4000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000
0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a0c46014ae0a195606cdd34db0
68b6222ed7b3a643cf110f87555d53726960855f987b8acdb91df78e58fe82c14c16f746117499cfb632155f001b963f6de7f9ca7f03551ecb5c1cb8621766
8afe2db4ff9eea9821c92fb1f82cfffc7ea9d4b44284d6202155ff2bb26a44d792a252174065a8ddb7b486d19530d1f1b25bfb35dcbdcdaffe8ac6a470d799
4aa2b7d84cc8cded5756d0d7be39fcb64fb170e270370000000000000000000000000000000000000000000000000000000000000002000100000000000000
0000000000000000000000000000000000000000000000 verified on L1. Tx hash: 
0xfc6c86703c1794b95bdff296f85c6879890ee05b5bd31e6104927b56c30fa8fc
```

## Design

M1 consists of checking the inclusion of the blob and verifying that the data that is committed to is the correct one, this computations would be too heavy/costly to run directly on chain. An offchain implementation is needed in order to prevent this high costs. We resolve this by making a binary capable of running this checks in a provable way.

### Integration

The important components are marked in **bold**

#### Step 1 Dispersal (Marked in Blue)

![Step 1](images/step1.png)

1. Zksync's sequencer finishes a batch and wants to disperse its content (**Blob Data**).
2. Zksync's sequencer sends the blob to be dispersed to EigenDA, EigenDA returns the **Blob Key**.
3. Zksync's sequencer stores the **Blob Key** in its database

#### Step 2 Proof generation (Marked in Red)

![Step 2](images/step2.png)

4. Zksync’s sequencer asks for **Inlcusion Data** (encoded **EigenDACert**) to EigenDA
5. Zksync’s sequencer starts waiting for batch to be verified.
6. Sidecar asks zksync’s sequencer for **Blob Key**.
7. Sidecar asks EigenDA for **EigenDACert** and **Blob Data**
8. Sidecar executes Risc0, doing 3 things

    a. Call to VerifyDACertV2 *

    b. Proof of Equivalence *

    c. Calculation of **EigenDAHash** (keccak of **BlobData**) *
    
    And generates a **Risc0 Proof** of those 3 things
    
9. Sidecar calls the EigenDACertAndBlobVerifier verify function on Ethereum
10. EigenDACertAndBlobVerifier verifies the **Risc0 Proof** and stores in its mappings:

    a. finishedBatches: **Inclusion Data** → true (meaning the proof generation for the given **Inclusion Data** finished)

    b. verifiedBatches: **Inclusion Data** → true (meaning the proof was correctly verified)

    c. hashes: **Inclusion Data** → **EigenDAHash**

11. Zksync’s sequencer finishes waiting for batch to be verified by checking EigenDARegistry verifiedBatches mapping on Ethereum, with **Inclusion Data** as key, and stores the **Inclusion Data** in its database, and calls the Commit Batches function of Executor (zksync’s DiamondProxy implementation)on Ethereum.

#### Step 3 Commit Batches (Marked in Green)

![Step 3](images/step3.png)

Everything here runs on Ethereum

12. Executor starts commit batches function
13. Executor calls the EigenDAL1Validator checkDA function with **l2DAValidatorOutputHash** and **operatorDAInput** as parameters

    a. **l2DAValidatorOutputHash**: keccak(**stateDiffHash** + **EigenDAHash**)

    b. **operatorDAInput**: **StateDiffHash** + **Inclusion Data**
    
    * **stateDiffHash**  is the hash of the states diffs, calculated on EigenDAL2Validator and sent to L1 through L2→L1 Logs
    
14. EigenDAL1Validator calls EigenDACertAndBlobVerifier isVerified function with **Inclusion Data** as parameter, which returns whether it was correctly verified along with the **EigenDAHash**
15. EigenDAL1Validator checks if keccak(**stateDiffHash** + **EigenDAHash**) equals **l2DAValidatorOutputHash** (meaning that if not, **EigenDAHash** was not correctly calculated by the sidecar)

### What does the Guest do?

There are 3 things we want to achieve with the Risc0 guest. Each one of them is addressed in point 8.

1. We want to check that the blob is available in EigenDA. 8.a
2. We want to check that the commitment commits to that blob. 8.b
3. We want to check that the blob is the same we dispersed on zksync. 8.c

# TODO: Update to V2
#### * Call to VerifyBlobV1 (8.a)

On the host:

Inputs: `rpc_url`, `blob_verifier_wrapper_addr`

```rust
   let call = IVerifyBlob::verifyBlobV1Call {
        blobHeader: blob_header.clone(),
        blobVerificationProof: blob_verification_proof.clone(),
    };

    // Create an EVM environment from an RPC endpoint defaulting to the latest block.
    let mut env = EthEvmEnv::builder().rpc(rpc_url.clone()).build().await?;

    // Preflight the call to prepare the input that is required to execute the function in
    // the guest without RPC access. It also returns the result of the call.
    let mut contract = Contract::preflight(blob_verifier_wrapper_addr, &mut env);
    
    let returns = contract
        .call_builder(&call)
        .from(caller_addr)
        .call()
        .await?;
        
    let input = env.into_input().await?;
```

We make the preflight call to BlobVerifierWrapper to populate the EvmEnv

Output: input (EthEvmInput type)

On the guest:

Inputs: input

```rust
  let input: EthEvmInput = env::read();

  // Converts the input into a `EvmEnv` for execution.
  let env = input.into_env();
 
 
  // Execute the view call; it returns the result in the type generated by the `sol!` macro.
  let contract = Contract::new(blob_verifier_wrapper_addr, &env);
	let call = IVerifyBlob::verifyBlobV1Call {
	    blobHeader: blob_info.blob_header.into(),
	    blobVerificationProof: blob_info.blob_verification_proof.into(),
		};
	let returns = contract.call_builder(&call).from(caller_addr).call();
	assert!(returns._0); 
```

Here we make the call to verifyBlobV1 inside the risc0 steel VM.

What risc0 steel does is, given the env it generates a revm VM with the given state of the chain.

And it simulates the call to the contract.

We then assert the result of that call being true.

Outputs: **Risc0Proof**

#### * Proof Of Equivalence (8.b)

On the host:

Inputs: **BlobInfo, BlobData,** SRSPoints

```rust
    let blob = Blob::from_raw_data(&data);

    let mut kzg = KZG::new();

    kzg.calculate_and_store_roots_of_unity(blob.len().try_into()?)?;

    let x: [u8; 32] = blob_header.commitment.x.to_be_bytes();
    let y: [u8; 32] = blob_header.commitment.y.to_be_bytes();

    let x_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&x));
    let y_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&y));

    let cert_commitment = G1Affine::new(x_fq, y_fq);
```

First we obtain the commitment from the BlobInfo’s blob header

```rust
    // Calculate the polynomial in evaluation form
    let poly_coeff = blob.to_polynomial_coeff_form();
    let poly_eval = poly_coeff.to_eval_form()?;

    let evaluation_challenge = compute_challenge(&blob, &cert_commitment)?;

    // Compute the proof that the commitment corresponds to the given blob
    let proof = kzg.compute_proof(&poly_eval,&evaluation_challenge,&srs)?;
```

We then calculate the eval polynomial, the evaluation challenge and the proof for that commitment

Output: Proof

Guest:

Inputs: **BlobData**, Proof, BlobInfo (commitment)

```rust
    // Calculate the polynomial in evaluation form
    let poly_coeff = blob.to_polynomial_coeff_form();
    let poly_eval = poly_coeff.to_eval_form().unwrap();

    // Get the commitment from blob info
    let x_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&blob_info.blob_header.commitment.x));
    let y_fq = Fq::from(num_bigint::BigUint::from_bytes_be(&blob_info.blob_header.commitment.y));

    let cert_commitment = G1Affine::new(x_fq, y_fq);

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

#### * EigenDAHash (8.c)

In the guest we also calculate the eigenDAHash

```rust
let hash = keccak256(&data);
    let output = Output {
        hash: hash.to_vec(),
        env: env.commitment().abi_encode(),
    };
```

And return it as a public output.

We then send that hash to the EigenDARegistry contract, which stores it in its mapping

Then on zksync’s EigenDAL1Validator, we check that hash against the one obtained through L2→L1 Logs.

The idea of this check is to make sure that the blob we verified on the guest is the same blob we dispersed on zksync

### OLD DOCUMENTATION

### Data Flow

This diagram shows how data flows through each point of the solution

![Data Flow](images/data-flow.png)

### How

![Flow Chart](images/flow-chart.png)

This flow shows a `zksync-eigenda` sidecar that uses `Risc0`, where we query the Era chain Web API for each `blobID` dispersed to `EigenDA`,
with which we query `EigenDA` for the `blobInfo` and blob data. Then we do a preflight call the `BlobVerifierWrapper` contract, which creates the EVM state passed to Risc0 Steel.
Then we start the guest execution, where `verifyBlobV1` is called and the proof of equivalence is verified, generating a groth16 proof, which we verify on the EigenDA Registry.

Note: `verifyBlobV1` will be replaced by the V2 API once the `EigenDAv2` Client is ready

#### Key points

- The sidecar uses [risc0 steel](https://github.com/risc0/risc0-ethereum/tree/main/crates/steel), a prover capable of running EVM code offchain, which consists of two entities:
    - [The host](https://dev.risczero.com/terminology#host-program), which communicates with the “outside world”:
        - With the `Zksync Era Web API` in order to retrieve the latests blob which inclusion data is still to be proven.
        - With `Ethereum:`
            - To make a preflight call, which constructs the EVM environment needed by the guest executed secondly, by collecting the necessary state and merkle storage proofs neeed.
            - To verify the proof generated by the guest on chain.
        - With `EigenDA` to get the `BlobInfo` and `BlobData`
    - [The guest](https://dev.risczero.com/terminology#guest-program), which is responsible for generating the proof:
        - The guest program executed first verifies the bn254 proof generated from calculating the proof of equivalence **in the host**.
        - The guest program executed second makes a “view call” to query EVM state, as stated above, the data needed for this query is passed from the host, as `input`.
- We compute the proof of equivalence outside the guest program to greatly reduce computation costs (otherwise each proof takes around fifty hours to generate).
- The `zksync-era` chain Web API is periodically queried by our sidecar to check for new blobs which inclusion data is still unverified.
- We create a *Blob Verifier Wrapper* Contract because steel mandates that we query a “view call” that returns a value of *some* type (in this cases `external view returns (bool)`).

### M1 Full Flow

![Full Flow](images/full-flow.png)

1. The `Zksync Era` node dispatches the blob to `EigenDA`, and gets the certificate (`BlobInfo`).
2. The sidecar queries the node for the latest blob id.
3. The sidecar queries `EigenDA` disperser for the `blobInfo` and data related to the last Blob obtained.
4. The sidecar performs both the Proof of Equivalence and the `verifyBlob` call and generates a risc0 proof, which sends to the `EigenDA` registry to verify. The `EigenDA` registry verifies it by calling the groth16 verifier and stores wether it was correctly verified or not.
5. The zksync node calls the EigenDA Registry in order to check if the blob has been verified.
6. The node calls the `commit_batches` function of `Executor.sol`, which calls `checkDA` to the `L1DAValidator`, it asks the `EigenDA` Registry if the given blob was correctly verified, and if it was it continues execution.
