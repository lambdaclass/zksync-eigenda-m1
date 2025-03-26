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

Also replace line 28 on `avs-devnet/kurtosis_package/keys.star` for

`shared_utils.send_funds(plan, context, info["address"], "10000ether")`

And add

```yaml
- name: zksync_rich_1
  address: "0x36615Cf349d7F6344891B1e7CA7C72883F5dc049"
- name: zksync_rich_2
  address: "0xa61464658AfeAf65CccaaFD3a512b69A83B77618"
- name: zksync_rich_3
  address: "0x0D43eB5B8a47bA8900d84AA36656c92024e9772e"
- name: zksync_rich_4
  address: "0xA13c10C0D5bd6f79041B9835c63f91de35A15883"
- name: zksync_rich_5
  address: "0x8002cD98Cfb563492A6fB3E7C8243b7B9Ad4cc92"
- name: zksync_rich_6
  address: "0x4F9133D1d3F50011A6859807C837bdCB31Aaab13"
- name: zksync_rich_7
  address: "0xbd29A1B981925B94eEc5c4F1125AF02a2Ec4d1cA"
- name: zksync_rich_8
  address: "0xedB6F5B4aab3dD95C7806Af42881FF12BE7e9daa"
- name: zksync_rich_9
  address: "0xe706e60ab5Dc512C36A4646D719b889F398cbBcB"
- name: zksync_rich_10
  address: "0xE90E12261CCb0F3F7976Ae611A29e84a6A85f424"
```

To `keys:` section of `devnet.yaml`

As well as replacing ehereum-package section for

```yaml
# ethereum-package configuration
ethereum_package:
  additional_services:
    - blockscout
  network_params:
    # NOTE: turning this to 1s causes "referenceBlockNumber is in future" errors
    seconds_per_slot: 3
    network_id: "9"
```


After runnning the devnet run

```bash
avs-devnet get-ports
avs-devnet get-address eigenda_addresses: 
```

Save ports for `el-1-besu-lighthouse: rpc` and `disperser: grpc`

Save addresses of `blobVerifier` and `eigenDAServiceManager`

### Run zksync-era (eigenda-m1-post-merge branch on lambdaclass fork):

Install zkstack:

```bash
cd ./zkstack_cli/zkstackup
./install --local
```

On `zksync-era/zkstack_cli/crates/types/src/l1_network.rs`

Modify the address for `eigenda_registry` for your address.

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
    points_source_path: ./resources
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
          --l1-rpc-url http://127.0.0.1:<your_rpc> \
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

Export the needed variables (rpcs should have http://, private keys and addresses should have 0x)
```bash
export PRIVATE_KEY=<your_private_key>
export DISPERSER_PRIVATE_KEY=<your_disperser_private_key>
export BLOB_VERIFIER_ADDRESS=<your_blob_verifier_address>
export RPC_URL=<your_rpc>
export DISPERSER_RPC=<your_rpc>
export SVC_MANAGER_ADDR=<your_address>
export CALLER_ADDR=<your_address>
```
Disperser private key should be the one you have listed to use with the eigenda disperser
Caller address is the address you want to use to call the `VerifyBlobV1` function

Deploy the blobVerifierWrapper:

```bash
forge script contracts/script/BlobVerifierWrapperDeployer.s.sol:BlobVerifierWrapperDeployer --rpc-url $RPC_URL --broadcast -vvvv
```

Save the address under `Contract Address: <address>`

```bash
export BLOB_VERIFIER_WRAPPER_ADDR=<your_address>
```

The address on CALLER is a known address from zksync, it should be changed to the needed one in the real use case.

Deploy the `Risc0Groth16Verifier`:
```bash
make deploy-risc0-verifier ETH_WALLET_PRIVATE_KEY=$PRIVATE_KEY RPC_URL=$RPC_URL
```

Save the address under `Contract Address: <address>`

```bash
export RISC0_VERIFIER_ADDRESS=<your_address>
```

Deploy the `EigenDARegistry`:

```bash
forge script contracts/script/EigenDARegistryDeployer.s.sol:EigenDARegistryDeployer --rpc-url $RPC_URL --broadcast -vvvv
```

Save the address under `Contract Address: <address>`

```bash
export EIGENDA_REGISTRY_ADDR=<your_address>
```

To run the example execute the following command:

```bash
VERIFICATION_PRIVATE_KEY=$PRIVATE_KEY RUST_LOG=info cargo run --release
```


