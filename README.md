# ZKSYNC-EIGENDA M1

**The EigenDA sidecar where risc0-steel is used in order to generate a proof for the call of the VerifyBlobV1 function of EigenDA's BlobVerifier contract, which performs the necessary checks to make sure a given blob is present.**
**As well as performing the proof of equivalence verifying a proof that the EigenDA commitment commits to the given Blob.**
**Finally it sends the Risc0 Proof to verify to the EigenDA Registry contract, which stores whether it was correctly verified.**

Note: `verifyBlobV1` will be replaced by the V2 API once the `EigenDAv2` Client is ready
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

And replace `ethereum-package` section for

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
```

Deploy the contracts:

```bash
forge script contracts/script/ContractsDeployer.s.sol:ContractsDeployer --rpc-url $RPC_URL --broadcast -vvvv
```

Save the address under `BlobVerifierWrapper deployed at: <address>`
Save the address under `CertAndBlobVerifier Proxy deployed at: <address>`

```bash
export BLOB_VERIFIER_WRAPPER_ADDR=<your_address>
export EIGENDA_CERT_AND_BLOB_VERIFIER_ADDR=<your_address>
```

### Run zksync-era (m1-eigenda branch on lambdaclass fork):

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
    disperser_rpc: http://<disperser: grpc> #On avs-devnet ports
    settlement_layer_confirmation_depth: 0
    eigenda_eth_rpc: http://<el-1-besu-lighthouse: rpc> #On avs-devnet ports
    eigenda_svc_manager_address: <eigenDAServiceManager> #On avs-devnet addresses
    wait_for_finalization: false
    authenticated: false
    points_source_path: ./resources
    eigenda_cert_and_blob_verifier_addr: <CertAndBlobVerifier> #Under CERT_AND_BLOB_VERIFIER_ADDR env variable
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

## Design

M1 consists of checking the inclusion of the blob and verifying that the data that is committed to is the correct one, this computations would be too heavy/costly to run directly on chain. An offchain implementation is needed in order to prevent this high costs. We resolve this by making a binary capable of running this checks in a provable way.

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
