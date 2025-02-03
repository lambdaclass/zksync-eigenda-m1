# ZKSYNC-EIGENDA M1

**A proof of concept where risc0-steel is used in order to generate a proof for the call of the VerifyBlobV1 function of EigenDA's BlobVerifier contract, which performs the necessary checks to make sure a given blob is present**

## Prerequisites

To get started, you need to have Rust installed. If you haven't done so, follow the instructions [here][install-rust].

Next, you will also need to have the `cargo-risczero` tool installed following the instructions [here][install-risczero].

You'll also need access to an Ethereum Sepolia RPC endpoint. You can for example use [ethereum-sepolia-rpc.publicnode.com](https://ethereum-sepolia-rpc.publicnode.com) or a commercial RPC provider like [Alchemy](https://www.alchemy.com/).

## Run the example

Compile the contracts:

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

Update the CONTRACT address on ```host/src/main.rs``` and ```methods/guest/src/main.rs``` if needed.

The address on CALLER is a known address from zksync, it should be changed to the needed one in the real use case.

Disperse a blob on the devnet and replace the CALL in both host and guest with the real values for that dispersal.

To run the example execute the following command:

```bash
RPC_URL=<your_rpc> RUST_LOG=info cargo run --release
```


