# ZKSYNC-EIGENDA M1

**A proof of concept where risc0-steel is used in order to generate a proof for the call of the VerifyBlobV1 function of EigenDA's BlobVerifier contract, which performs the necessary checks to make sure a given blob is present**

## Prerequisites

To get started, you need to have Rust installed.

Next, you will also need to have the [`cargo-risczero`](https://dev.risczero.com/api/zkvm/install) tool installed.

Next we need to install cuda 12.6

Install [cuda](https://developer.nvidia.com/cuda-downloads?target_os=Linux&target_arch=x86_64&Distribution=Debian&target_version=12&target_type=runfile_local).
Use the runfile (local) option, use the wget shown to download the script and run it as:

```bash
sudo ./<file>.run
```



## Run the example

### First run the eigenda devnet:

Install devnet: 

Clone [avs-devnet](https://github.com/Layr-Labs/avs-devnet) repository and install the `avs-devnet` tool by running

```bash
make deps      # installs dependencies
make install   # installs the project
```

Go to [Avs-Devnet repo](https://github.com/Layr-Labs/avs-devnet/blob/main/examples/eigenda.yaml) and follow the steps to run the EigenDA devnet, before running `avs-devnet start`, add the following line on `contracts/script/SetUpEigenDA.s.sol` on eigenda:

Line 214: `vm.serializeAddress(output,"blobVerifier", address(eigenDABlobVerifier));`

After runnning the devnet run

```bash
avs-devnet get-ports
avs-devnet get-address eigenda_addresses: 
```

Save ports for `el-1-besu-lighthouse: rpc` and `disperser: grpc`

Save addresses of `blobVerifier` and `eigenDAServiceManager`

### Run zksync-era (eigenda-m1 branch on lambdaclass fork):

Install zkstack:

```bash
cd ./zkstack_cli/zkstackup
./install --local
```

Reload your terminal, and run on zksync-era root:

```bash
zkstackup --local
```

Install foundry-zksync 0.0.2:

```
curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash
foundryup-zksync --commit 27360d4c8d12beddbb730dae07ad33a206b38f4b
```

Modify `etc/env/file_based/overrides/validium.yaml`:

```
da_client:
  eigen:
    disperser_rpc: http://<disperser: grpc>
    settlement_layer_confirmation_depth: 0
    eigenda_eth_rpc: http://<el-1-besu-lighthouse: rpc>
    eigenda_svc_manager_address: <eigenDAServiceManager>
    wait_for_finalization: false
    authenticated: false
    path: ./resources
```

Copy the resources folder inside eigenda to zksync-era root

Modify `etc/env/file_based/secrets.yaml`:

```
da:
  eigen:
    private_key: <your_private_key>
```

Run

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
          --l1-rpc-url http://127.0.0.1:8545 \
          --server-db-url=postgres://postgres:notsecurepassword@localhost:5432 \
          --server-db-name=zksync_server_localhost_eigenda \
          --chain eigenda \
          --verbose
```

Then run
```
zkstack server --chain eigenda
```


### Run this example (back on this repo):

Compile the contracts

```bash
git submodule update --init
make build_contracts
```

Deploy the blobVerifierWrapper:

```bash
PRIVATE_KEY=<your_private_key> BLOB_VERIFIER_ADDRESS=<your_blob_verifier_address> forge script contracts/script/BlobVerifierWrapperDeployer.s.sol:BlobVerifierWrapperDeployer --rpc-url <your_rpc_url> --broadcast -vvvv
```

For testing purpouses on devnet you can use:
```bash
PRIVATE_KEY=0x3eb15da85647edd9a1159a4a13b9e7c56877c4eb33f614546d4db06a51868b1c BLOB_VERIFIER_ADDRESS=0x00CfaC4fF61D52771eF27d07c5b6f1263C2994A1 forge script contracts/script/BlobVerifierWrapperDeployer.s.sol:BlobVerifierWrapperDeployer --rpc-url http://127.0.0.1:<your_port> --broadcast -vvvv
```

Update the CONTRACT address on ```host/src/main.rs``` and ```methods/guest/src/main.rs``` for the one just deployed if needed.

The address on CALLER is a known address from zksync, it should be changed to the needed one in the real use case.

If necessary, deploy the `Risc0ProofVerifierWrapper`:

```bash
PRIVATE_KEY=<your_pk> RISC0_VERIFIER_ADDRESS=<your_address> forge script contracts/script/Risc0ProofVerifierWrapperDeployer.s.sol:Risc0ProofVerifierWrapperDeployer --rpc-url <your_rpc> --broadcast -vvvv
```

There is already one deployed in holesky: `0x25b0F3F5434924821Ad73Eed8C7D81Db87DB0a15`

To run the example execute the following command:

```bash
RPC_URL=<your_rpc> PRIVATE_KEY=<your_private_key> PROOF_VERIFIER_RPC=<your_rpc> RUST_LOG=info cargo run --release
```


