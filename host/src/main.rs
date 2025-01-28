// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use alloy_primitives::{address, Address, U256};
use alloy_sol_types::{sol, SolCall, SolType};
use anyhow::{Context, Result};
use clap::Parser;
use erc20_methods::ERC20_GUEST_ELF;
use risc0_steel::{
    ethereum::{EthEvmEnv},
    Commitment, Contract,
};
use risc0_zkvm::{default_executor, ExecutorEnv};
use tracing_subscriber::EnvFilter;
use url::Url;

sol! {
    struct QuorumBlobParam {
        uint8 quorumNumber;
        uint8 adversaryThresholdPercentage;
        uint8 confirmationThresholdPercentage; 
        uint32 chunkLength; 
    }

    struct G1Point {
        uint256 x;
        uint256 y;
    }
    
    struct BlobHeader {
        G1Point commitment; 
        uint32 dataLength; 
        QuorumBlobParam[] quorumBlobParams; 
    }
    
    struct ReducedBatchHeader {
        bytes32 blobHeadersRoot;
        uint32 referenceBlockNumber;
    }
    
    struct BatchHeader {
        bytes32 blobHeadersRoot;
        bytes quorumNumbers; 
        bytes signedStakeForQuorums; 
        uint32 referenceBlockNumber;
    }
        
    struct BatchMetadata {
        BatchHeader batchHeader; 
        bytes32 signatoryRecordHash; 
        uint32 confirmationBlockNumber; 
    }
    
    struct BlobVerificationProof {
        uint32 batchId;
        uint32 blobIndex;
        BatchMetadata batchMetadata;
        bytes inclusionProof;
        bytes quorumIndices;
    }

    /// VerifyBlobV1 function signature.
    /// This must match the signature in the guest.
    interface IVerifyBlob {
        function verifyBlobV1(BlobHeader calldata blobHeader, BlobVerificationProof calldata blobVerificationProof) external view returns (bool);
    }
}



/// Address of the deployed contract to call the function on (USDT contract on Sepolia).
const CONTRACT: Address = address!("00CfaC4fF61D52771eF27d07c5b6f1263C2994A1"); //TODO: Add the address of the deployed contract.
/// Address of the caller.
const CALLER: Address = address!("f08A50178dfcDe18524640EA6618a1f965821715");

/// Simple program to show the use of Ethereum contract data inside the guest.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
}

#[tokio::main]
async fn main() -> Result<()> {

    let CALL: IVerifyBlob::verifyBlobV1Call = IVerifyBlob::verifyBlobV1Call {
        blobHeader: BlobHeader {
            commitment: G1Point {
                x: U256::from(0),
                y: U256::from(0),
            },
            dataLength: 0,
            quorumBlobParams: vec![QuorumBlobParam {
                quorumNumber: 0,
                adversaryThresholdPercentage: 0,
                confirmationThresholdPercentage: 0,
                chunkLength: 0,
            }],
        },
        blobVerificationProof: BlobVerificationProof {
            batchId: 0,
            blobIndex: 0,
            batchMetadata: BatchMetadata {
                batchHeader: BatchHeader {
                    blobHeadersRoot: U256::from(0).into(),
                    quorumNumbers: vec![0].into(),
                    signedStakeForQuorums: vec![0].into(),
                    referenceBlockNumber: 0,
                },
                signatoryRecordHash: U256::from(0).into(),
                confirmationBlockNumber: 0,
            },
            inclusionProof: vec![0].into(),
            quorumIndices: vec![0].into(),
        }
    };

    // Initialize tracing. In order to view logs, run `RUST_LOG=info cargo run`
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    // Parse the command line arguments.
    let args = Args::parse();

    // Create an EVM environment from an RPC endpoint defaulting to the latest block.
    let mut env = EthEvmEnv::builder().rpc(args.rpc_url).build().await?;
    //  The `with_chain_spec` method is used to specify the chain configuration.
    //env = env.with_chain_spec(&ETH_SEPOLIA_CHAIN_SPEC);

    // Preflight the call to prepare the input that is required to execute the function in
    // the guest without RPC access. It also returns the result of the call.
    let mut contract = Contract::preflight(CONTRACT, &mut env);
    let returns = contract.call_builder(&CALL).from(CALLER).call().await?;
    println!(
        "Call {} Function by {:#} on {:#} returns: {}",
        IVerifyBlob::verifyBlobV1Call::SIGNATURE,
        CALLER,
        CONTRACT,
        returns._0
    );

    // Finally, construct the input from the environment.
    let input = env.into_input().await?;

    println!("Running the guest with the constructed input...");
    let session_info = {
        let env = ExecutorEnv::builder()
            .write(&input)
            .unwrap()
            .build()
            .context("failed to build executor env")?;
        let exec = default_executor();
        exec.execute(env, ERC20_GUEST_ELF)
            .context("failed to run executor")?
    };

    // The journal should be the ABI encoded commitment.
    let commitment = Commitment::abi_decode(session_info.journal.as_ref(), true)
        .context("failed to decode journal")?;
    println!("{:?}", commitment);

    Ok(())
}
