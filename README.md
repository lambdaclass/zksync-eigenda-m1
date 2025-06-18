# Zksync-EigenDA proving sidecar

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
export DATABASE_URL=<proof_database_url> #URL of the database where the proofs will be stored
export METRICS_URL=<your_metrics_url> #URL where you want the metrics to be exported, the example granafa expects it to be on port 9100
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
make containers # Creates the containers that the sidecar uses
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
  client: EigenDAV2Secure
  version: V2Secure
  disperser_rpc: <your_disperser_rpc> #Under DISPERSER_RPC env variable
  eigenda_eth_rpc: <your_eth_rpc> #Under RPC_URL env variable
  authenticated: true
  settlement_layer_confirmation_depth: 0 #Value needed for V1 compatibility, you can leave this one
  eigenda_svc_manager_address: 0xD4A7E1Bd8015057293f0D0A557088c286942e84b #Value needed for V1 compatibility, you can leave this one
  wait_for_finalization: false #Value needed for V1 compatibility, you can leave this one
  points: #Value needed for V1 compatibility, you can leave this one
    source: Url
    g1_url: https://github.com/Layr-Labs/eigenda-proxy/raw/2fd70b99ef5bf137d7bbca3461cf9e1f2c899451/resources/g1.point
    g2_url: https://github.com/Layr-Labs/eigenda-proxy/raw/2fd70b99ef5bf137d7bbca3461cf9e1f2c899451/resources/g2.point.powerOf2
  cert_verifier_addr: <your_cert_verifier_address> #Under CERT_VERIFIER_ADDRESS env variable
  blob_version: <your_blob_version> #Under BLOB_VERSION env variable
  polynomial_form: <your_polynomial_form> #Either coeff or eval
  eigenda_sidecar_rpc: <your_sidecar_rpc> #Under SIDECAR_URL env variable
```

Modify `etc/env/file_based/secrets.yaml`:

```yaml
da:
  client: EigenDA
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
2025-03-27T18:20:05.383060Z  INFO zksync_da_dispatcher::da_dispatcher: Dispatched a DA for batch_number: 1, pubdata_size: 5312, dispatch_latency: 24.480322ms

2025-03-27T18:36:19.150242Z  INFO zksync_da_dispatcher::da_dispatcher: Received an inclusion data for a batch_number: 1, inclusion_latency_seconds: 973

2025-03-27T18:36:23.535623Z  INFO EthTxManager::loop_iteration: zksync_eth_sender::eth_tx_manager: Checking tx id: 1, operator_nonce: OperatorNonce { finalized: Nonce(1), latest: Nonce(1) }, tx nonce: 0

2025-03-27T18:36:23.540535Z  INFO EthTxManager::loop_iteration: zksync_eth_sender::eth_tx_manager: eth_tx 1 with hash 0xbe08cdd9ba138548f45c152e1a913784dd5cb2157e2d6323db9fe182aa067e2f for CommitBlocks is confirmed. Gas spent: 267395
```

On the sidecar you should see blobs being verified:

```
Running JSON RPC server
Running proof gen thread
Proof gen thread: retrieved request to prove: bdfef9b13ccd6648534267b80bea88b1b6c75ecfef4468299d32fd646c47c7b9
2025-05-19T15:45:28.038452Z  INFO risc0_steel::host::builder: Environment initialized with block 3861957 (0x2283aafccd1976c63f9dced04f03106629a7b76baccc519c2f6d4bb61ae4b59c)
2025-05-19T15:45:28.038505Z  INFO risc0_steel::contract::host: Executing preflight calling 'verifyDACertV2((bytes32,uint32),(((uint16,bytes,((uint256,uint256),(uint256[2],uint256[2]),(uint256[2],uint256[2]),uint32),bytes32),bytes,uint32[]),uint32,bytes),(uint32[],(uint256,uint256)[],(uint256,uint256)[],(uint256[2],uint256[2]),(uint256,uint256),uint32[],uint32[],uint32[][]),bytes)' on 0x18c7De1E82513c3F48dFcCa85c64056C637104fb
Call verifyDACertV2((bytes32,uint32),(((uint16,bytes,((uint256,uint256),(uint256[2],uint256[2]),(uint256[2],uint256[2]),uint32),bytes32),bytes,uint32[]),uint32,bytes),(uint32[],(uint256,uint256)[],(uint256,uint256)[],(uint256[2],uint256[2]),(uint256,uint256),uint32[],uint32[],uint32[][]),bytes) Function on 0x18c7…04fb returns: true
Running the guest with the constructed input...
2025-05-19T15:46:27.505240Z  INFO risc0_zkvm::host::server::exec::executor: execution time: 17.890119705s
Proof gen thread: finished generating proof for Blob Id bdfef9b13ccd6648534267b80bea88b1b6c75ecfef4468299d32fd646c47c7b9
```

### Clean the sidecar containers

If you want to clean the sidecar containers over different executions (Mostly during development)

```bash
make clean
```

