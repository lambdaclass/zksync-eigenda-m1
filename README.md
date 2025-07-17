# Zksync-EigenDA proving service

**Warning: This Proving service only works on a x86 machine with cuda support**

**The EigenDA Proving service where risc0-steel is used in order to generate a proof for the call of the checkDACert function of EigenDA's CertVerifier contract, which performs the necessary checks to make sure a given blob is present.**
**As well as performing the proof of equivalence verifying a proof that the EigenDA commitment commits to the given Blob.**
**The Proving service consists of 2 Endpoints:**
**generate_proof: Which given the blobKey begins the proof generation process**
**get_proof: Which given the blobKey it returns the generated proof or an error in case it hasn't finished**

## Deployment

### Hardware requirements

You will need cuda 12.6 installed to run the prover service.

Install [cuda](https://developer.nvidia.com/cuda-downloads?target_os=Linux&target_arch=x86_64&Distribution=Debian&target_version=12&target_type=runfile_local).
Use the runfile (local) option, use the wget shown to download the script and run it as:

```bash
sudo ./<file>.run
```

### Dependencies

Note: This Proving service requires using an Ethereum RPC, if this RPC fails (for example on an `eth_getProof`), the whole proving generation for that specific blob will fail. You should choose an RPC that's not prone to failing. Public RPC's often fail.

### Run the Proving service locally

Copy the example env file to `.env` and fill in the needed variables (RPCs should include http://, private keys and addresses should start with 0x)

```bash
cp .env.example .env
```

```bash
make start-deps # Starts the containers that the Proving service uses
RUST_LOG=info cargo run --release
```

Use `make stop-deps` to stop the dependency containers.

### Deploy the Proving service via Docker

TODO




## Local development

Make sure to `git clone` recursively or `git submodule update --init --recursive` if already cloned to get the contracts submodules.

### Dev dependencies

Install [mise](https://mise.jdx.dev/getting-started.html) and follow getting-started instructions to activate it for your shell. Then simply run:
```
mise install
mise run install-rust-tools
```
Some rust tools are not easily available on mise, so we install them via a mise task. See [mise.toml](mise.toml) for more details.

#### Risc0 Contracts and Tooling

Make sure the parameters passed to the risc zero verifier are up to date, you can find the most recent ones on https://github.com/risc0/risc0-ethereum/blob/main/contracts/src/groth16/ControlID.sol (You shouldn't need to change them if the RiscZero version is not changed here, but if you use a pre-deployed verifier it could be a source of errors)

### Hardware requirements

Use RISC0_DEV_MODE=true to skip risc0 GPU proof generation and instead execute the code on your cpu.
TODO: does this actually work?

### Start dependencies

```bash
anvil
make deploy-risc0-verifier-contract-anvil
# Write the deployed risc0 verifier address to your .env file
export RISC_ZERO_VERIFIER_ADDR=<DEPLOYED_ADDRESS>
make start-deps # start other containers
cargo run
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
  client: Eigen
  disperser_rpc: <your_disperser_rpc> #Under DISPERSER_RPC env variable
  eigenda_eth_rpc: <your_eth_rpc> #Under RPC_URL env variable
  cert_verifier_router_addr: <your_cert_verifier_address> #Under CERT_VERIFIER_ROUTER_ADDRESS env variable
  operator_state_retriever_addr: <your_operator_state_retriever_addr>
  registry_coordinator_addr: <your_registry_coordinator_addr>
  blob_version: <your_blob_version> #Under BLOB_VERSION env variable
  eigenda_proving_service_rpc: <your_proving_service_rpc> #Under PROVING_SERVICE_URL env variable
```

Modify `etc/env/file_based/secrets.yaml`:

```yaml
da:
  client: Eigen
  private_key: <your_private_key> #The private key you want to use with the eigenda disperser
```

Modify `etc/env/file_based/general.yaml`:

```yaml
eth:
  sender:
    gas_limit_mode: MAXIMUM
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
2025-06-23T19:42:05.370222Z  INFO zksync_da_dispatcher::da_dispatcher: Dispatched a DA for batch_number: 1, pubdata_size: 5312, dispatch_latency: 1.245608661s
2025-06-23T19:42:10.866138Z  INFO zksync_da_dispatcher::da_dispatcher: Finality check for a batch_number: 1 is successful
2025-06-23T19:57:23.619783Z  INFO zksync_da_dispatcher::da_dispatcher: Received an inclusion data for a batch_number: 1, inclusion_latency_seconds: 918
2025-06-23T19:57:24.666505Z  INFO NamedFuture{name="eth_tx_manager"}:EthTxManager::loop_iteration: zksync_eth_sender::eth_tx_manager: Checking tx id: 1, operator_nonce: OperatorNonce { finalized: Nonce(1), latest: Nonce(1), fast_finality: Nonce(1) }, tx nonce: 1
2025-06-23T19:57:25.676224Z  INFO NamedFuture{name="eth_tx_manager"}:EthTxManager::loop_iteration: zksync_eth_sender::eth_tx_manager: eth_tx 1 with hash 0xde1c0716058369b15190ec07a791b65d1565168f4ae88429e2f14652bb6f8918 for CommitBlocks is Finalized. Gas spent: 495881
```

On the proving service you should see blobs being verified:

```
2025-06-23T18:23:55.862758Z  INFO host: Starting EigenDA Proving Service
2025-06-23T18:23:56.611650Z  INFO host: Starting metrics server on port 9100
2025-06-23T18:23:56.611818Z  INFO host: Running JSON RPC server
2025-06-23T18:41:47.425199Z  INFO host: Received request to generate proof for Blob Id cf61a127c3604b6f9cf6a04b16902c682b134ec52097a588172edd181038c871
2025-06-23T18:41:49.409353Z  INFO host: Proof generation thread: retrieved request to prove: cf61a127c3604b6f9cf6a04b16902c682b134ec52097a588172edd181038c871
2025-06-23T18:41:57.585907Z  INFO risc0_steel::host::builder: Environment initialized with block 4052233 (0x4dddcf0064ca55f9a6bcdd4fc9cf739e34306e94225cf8ac57af9471945e5d9a)    
2025-06-23T18:41:57.585959Z  INFO risc0_steel::contract::host: Executing preflight calling 'checkDACert(bytes)'    
2025-06-23T18:42:25.370682Z  INFO host::guest_caller: Call checkDACert(bytes) Function on 0xDD73…Ffbd returns: 1
2025-06-23T18:42:33.637261Z  INFO host::guest_caller: Running the guest with the constructed input...
2025-06-23T18:42:43.248916Z  INFO risc0_zkvm::host::server::exec::syscall::verify2: SYS_VERIFY_INTEGRITY2: (af7ebdeb4a22996426538a857fc4e9d61f71504845afbba17918b5c1700b81b9, abd93866a6878528f29ffc6ea6d9e428cc9ad020a540dd11f1d45e5e9bb6db71)
2025-06-23T18:42:43.298544Z  INFO risc0_zkvm::host::server::exec::executor: execution time: 9.168113797s
```

