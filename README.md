# ZKSYNC-EIGENDA M1

**A proof of concept where risc0-steel is used in order to generate a proof for the call of the VerifyBlobV1 function of EigenDA's BlobVerifier contract, which performs the necessary checks to make sure a given blob is present**

## Prerequisites

To get started, you need to have Rust installed. If you haven't done so, follow the instructions [here][install-rust].

Next, you will also need to have the `cargo-risczero` tool installed following the instructions [here][install-risczero].

You'll also need access to an Ethereum Sepolia RPC endpoint. You can for example use [ethereum-sepolia-rpc.publicnode.com](https://ethereum-sepolia-rpc.publicnode.com) or a commercial RPC provider like [Alchemy](https://www.alchemy.com/).

## Run the example

### First run the eigenda devnet:

Go to [Avs-Devnet repo](https://github.com/Layr-Labs/avs-devnet/blob/main/examples/eigenda.yaml) and follow the steps to run the devnet.

Run 

```bash
avs-devnet get-ports
avs-devnet get-address eigenda_addresses: 
```

Save ports for `el-1-besu-lighthouse: rpc` and `disperser: grpc`

Save addresses of `blobVerifier` and `eigenDAServiceManager`

### Run zksync-era:

Modify `etc/env/file_based/overrides/validium.yaml`:

```
da_client:
  eigen:
    disperser_rpc: <disperser: grpc>
    settlement_layer_confirmation_depth: 0
    eigenda_eth_rpc: <el-1-besu-lighthouse: rpc>
    eigenda_svc_manager_address: <eigenDAServiceManager>
    wait_for_finalization: false
    authenticated: false
    points_source: ./resources
    g1_url: https://github.com/Layr-Labs/eigenda-proxy/raw/2fd70b99ef5bf137d7bbca3461cf9e1f2c899451/resources/g1.point
    g2_url: https://github.com/Layr-Labs/eigenda-proxy/raw/2fd70b99ef5bf137d7bbca3461cf9e1f2c899451/resources/g2.point.powerOf2
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

This will init zksync, you then need to start the server, which will disperse a blob after some time, you need the specific information of that blob for this poc example to work, the best way would be to modify the following in zksync-era to print that blob:

On `core/Cargo.toml` line 225 change the branch for `print-blob-info` and delete `core/Cargo.lock`.

Then run
```
zkstack server --chain eigenda
```


### Run this example (back on this repo):

Compile the contracts

```bash
make build_contracts
```

Deploy the verifierWrapper:

```bash
PRIVATE_KEY=<your_private_key> BLOB_VERIFIER_ADDRESS=<your_blob_verifier_address> forge script verifierWrapper/deployer/script/Deployer.s.sol:Deployer --rpc-url <your_rpc_url> --broadcast -vvvv
```

For testing purpouses on devnet you can use:
```bash
PRIVATE_KEY=0x3eb15da85647edd9a1159a4a13b9e7c56877c4eb33f614546d4db06a51868b1c BLOB_VERIFIER_ADDRESS=0x00CfaC4fF61D52771eF27d07c5b6f1263C2994A1 forge script verifierWrapper/deployer/script/Deployer.s.sol:Deployer --rpc-url http://127.0.0.1:<your_port> --broadcast -vvvv
```

Update the CONTRACT address on ```host/src/main.rs``` and ```methods/guest/src/main.rs``` for the one just deployed if needed.

The address on CALLER is a known address from zksync, it should be changed to the needed one in the real use case.

To run the example execute the following command:

```bash
RPC_URL=<your_rpc> RUST_LOG=info cargo run --release
```


