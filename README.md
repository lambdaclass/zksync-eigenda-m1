# ZKSYNC-EIGENDA M1

**The EigenDA sidecar where risc0-steel is used in order to generate a proof for the call of the VerifyBlobV1 function of EigenDA's BlobVerifier contract, which performs the necessary checks to make sure a given blob is present.**
**As well as performing the proof of equivalence verifying a proof that the EigenDA commitment commits to the given Blob.**
**Finally it sends the Risc0 Proof to verify to the EigenDA Registry contract, which stores whether it was correctly verified.**
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

### First run the eigenda devnet:

Install devnet: 

Clone [avs-devnet](https://github.com/Layr-Labs/avs-devnet) repository and install the `avs-devnet` tool by running

```bash
make deps      # installs dependencies
make install   # installs the project
```

Go to [Avs-Devnet repo](https://github.com/Layr-Labs/avs-devnet/blob/main/examples/eigenda.yaml) and follow the steps to run the EigenDA devnet, before running `avs-devnet start`:

Add the following line on `contracts/script/SetUpEigenDA.s.sol` on eigenda:

Line 214: `vm.serializeAddress(output,"blobVerifier", address(eigenDABlobVerifier));`

Replace line 28 on `avs-devnet/kurtosis_package/keys.star` for

`shared_utils.send_funds(plan, context, info["address"], "10000ether")`

Add

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

And replace `ehereum-package` section for

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

### Deployment steps (On this repo):

Compile the contracts

```bash
git submodule update --init --recursive
make build_contracts
```

Export the needed variables (rpcs should have http://, private keys and addresses should have 0x)
```bash
export PRIVATE_KEY=<your_private_key> #The private key you want to use to deploy contracts and call to VerifyBlobV1
export DISPERSER_PRIVATE_KEY=<your_disperser_private_key> #The private key you want to use with the eigenda disperser
export BLOB_VERIFIER_ADDRESS=<your_blob_verifier_address> #On avs-devnet addresses
export RPC_URL=<your_rpc> #On avs-devnet ports
export DISPERSER_RPC=<your_rpc> #On avs-devnet ports
export SVC_MANAGER_ADDR=<your_address> #On avs-devnet addresses
export CALLER_ADDR=<your_address> #Address you want to use to call the `VerifyBlobV1` function
```

Deploy the `blobVerifierWrapper`:

```bash
forge script contracts/script/BlobVerifierWrapperDeployer.s.sol:BlobVerifierWrapperDeployer --rpc-url $RPC_URL --broadcast -vvvv
```

Save the address under `Contract Address: <address>`

```bash
export BLOB_VERIFIER_WRAPPER_ADDR=<your_address>
```

Deploy the `Risc0Groth16Verifier`:
```bash
ETH_WALLET_PRIVATE_KEY=$PRIVATE_KEY forge script contracts/script/DeployRiscZeroGroth16Verifier.s.sol:DeployRiscZeroGroth16Verifier --rpc-url $RPC_URL --broadcast -vvvv
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

### Run zksync-era (eigenda-m1-post-merge branch on lambdaclass fork):

Install zkstack:

```bash
cd ./zkstack_cli/zkstackup
./install --local
```

On `zksync-era/zkstack_cli/crates/types/src/l1_network.rs`

Modify the address for `eigenda_registry` for your address (the one under `EIGENDA_REGISTRY_ADDR` env variable).

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
    disperser_rpc: http://<disperser: grpc> #On avs-devnet ports
    settlement_layer_confirmation_depth: 0
    eigenda_eth_rpc: http://<el-1-besu-lighthouse: rpc> #On avs-devnet ports
    eigenda_svc_manager_address: <eigenDAServiceManager> #On avs-devnet addresses
    wait_for_finalization: false
    authenticated: false
    points_source_path: ./resources
    eigenda_registry_addr: <eigenDARegistry> #Under EIGENDA_REGISTRY_ADDR env variable
```

**Copy the resources folder inside eigenda to zksync-era root**

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
          --l1-rpc-url http://127.0.0.1:<your_l1_rpc> \
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
