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
const CONTRACT: Address = address!("c551b009C1CE0b6efD691E23998AEFd4103680D3"); //TODO: Add the address of the deployed contract.
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

/*
BlobInfo { blob_header: Some(BlobHeader { commitment: Some(G1Commitment { 
x: [12, 186, 9, 125, 109, 210, 169, 156, 194, 186, 128, 197, 149, 245, 1, 26, 152, 248, 20, 82, 96, 121, 119, 77, 6, 231, 87, 57, 109, 236, 229, 6], 
y: [25, 112, 8, 182, 0, 220, 192, 72, 181, 0, 250, 41, 58, 57, 112, 13, 142, 117, 223, 8, 102, 64, 23, 218, 128, 78, 241, 214, 177, 112, 226, 254] }), 
data_length: 110, 
blob_quorum_params: [BlobQuorumParam 
{ quorum_number: 0, adversary_threshold_percentage: 33, confirmation_threshold_percentage: 55, chunk_length: 1 }, 
 BlobQuorumParam { quorum_number: 1, adversary_threshold_percentage: 33, confirmation_threshold_percentage: 55, chunk_length: 1 }] }), 
 blob_verification_proof: Some(BlobVerificationProof { batch_id: 19, blob_index: 8, batch_metadata: 
 Some(BatchMetadata { batch_header: Some(BatchHeader { 
 batch_root: [20, 47, 92, 250, 77, 23, 62, 102, 216, 235, 221, 23, 88, 160, 217, 28, 129, 31, 42, 247, 48, 141, 144, 83, 1, 13, 169, 152, 100, 190, 210, 175],
  quorum_numbers: [0, 1],
   quorum_signed_percentages: [100, 100], 
   reference_block_number: 891 }), 
   signatory_record_hash: [41, 90, 143, 44, 252, 38, 61, 131, 25, 111, 95, 188, 197, 5, 222, 100, 76, 19, 218, 98, 158, 176, 27, 181, 104, 156, 198, 142, 254, 154, 93, 143], 
   fee: [0], confirmation_block_number: 933, 
   batch_header_hash: [91, 231, 237, 123, 189, 166, 123, 148, 163, 128, 38, 12, 247, 184, 14, 151, 243, 99, 170, 219, 28, 183, 238, 187, 129, 40, 147, 151, 56, 131, 88, 179] }), 
   inclusion_proof: [165, 134, 96, 207, 173, 228, 119, 175, 205, 26, 7, 113, 84, 249, 87, 182, 3, 96, 46, 22, 176, 138, 50, 168, 68, 117, 242, 131, 71, 170, 54, 197, 29, 214, 247, 238, 40, 81, 192, 7, 46, 223, 94, 249, 182, 134, 129, 124, 169, 3, 192, 87, 1, 237, 46, 234, 237, 0, 211, 7, 132, 96, 25, 132, 35, 65, 216, 18, 241, 1, 236, 85, 5, 101, 219, 255, 56, 72, 32, 111, 73, 157, 183, 241, 51, 117, 16, 249, 220, 50, 72, 133, 254, 154, 107, 55, 128, 235, 55, 208, 115, 109, 149, 80, 235, 235, 208, 51, 224, 63, 148, 100, 161, 148, 201, 71, 57, 8, 58, 152, 64, 174, 85, 30, 151, 98, 154, 26, 49, 132, 28, 60, 104, 249, 74, 135, 188, 109, 5, 191, 181, 72, 111, 140, 103, 75, 138, 239, 153, 238, 248, 160, 197, 137, 53, 105, 45, 56, 60, 68], 
   quorum_indexes: [0, 1] }) }
 */

#[tokio::main]
async fn main() -> Result<()> {

    let CALL: IVerifyBlob::verifyBlobV1Call = IVerifyBlob::verifyBlobV1Call {
        blobHeader: BlobHeader {
            commitment: G1Point {
                x: U256::from_be_bytes([12, 186, 9, 125, 109, 210, 169, 156, 194, 186, 128, 197, 149, 245, 1, 26, 152, 248, 20, 82, 96, 121, 119, 77, 6, 231, 87, 57, 109, 236, 229, 6]),
                y: U256::from_be_bytes([25, 112, 8, 182, 0, 220, 192, 72, 181, 0, 250, 41, 58, 57, 112, 13, 142, 117, 223, 8, 102, 64, 23, 218, 128, 78, 241, 214, 177, 112, 226, 254]),
            },
            dataLength: 110,
            quorumBlobParams: vec![QuorumBlobParam {
                quorumNumber: 0,
                adversaryThresholdPercentage: 33,
                confirmationThresholdPercentage: 55,
                chunkLength: 1,
            },
            QuorumBlobParam {
                quorumNumber: 1,
                adversaryThresholdPercentage: 33,
                confirmationThresholdPercentage: 55,
                chunkLength: 1,
            }],
        },
        blobVerificationProof: BlobVerificationProof {
            batchId: 19,
            blobIndex: 8,
            batchMetadata: BatchMetadata {
                batchHeader: BatchHeader {
                    blobHeadersRoot: U256::from_be_bytes([20, 47, 92, 250, 77, 23, 62, 102, 216, 235, 221, 23, 88, 160, 217, 28, 129, 31, 42, 247, 48, 141, 144, 83, 1, 13, 169, 152, 100, 190, 210, 175]).into(),
                    quorumNumbers: vec![0,1].into(),
                    signedStakeForQuorums: vec![100,100].into(),
                    referenceBlockNumber: 891,
                },
                signatoryRecordHash: U256::from_be_bytes([41, 90, 143, 44, 252, 38, 61, 131, 25, 111, 95, 188, 197, 5, 222, 100, 76, 19, 218, 98, 158, 176, 27, 181, 104, 156, 198, 142, 254, 154, 93, 143]).into(),
                confirmationBlockNumber: 933,
            },
            inclusionProof: vec![165, 134, 96, 207, 173, 228, 119, 175, 205, 26, 7, 113, 84, 249, 87, 182, 3, 96, 46, 22, 176, 138, 50, 168, 68, 117, 242, 131, 71, 170, 54, 197, 29, 214, 247, 238, 40, 81, 192, 7, 46, 223, 94, 249, 182, 134, 129, 124, 169, 3, 192, 87, 1, 237, 46, 234, 237, 0, 211, 7, 132, 96, 25, 132, 35, 65, 216, 18, 241, 1, 236, 85, 5, 101, 219, 255, 56, 72, 32, 111, 73, 157, 183, 241, 51, 117, 16, 249, 220, 50, 72, 133, 254, 154, 107, 55, 128, 235, 55, 208, 115, 109, 149, 80, 235, 235, 208, 51, 224, 63, 148, 100, 161, 148, 201, 71, 57, 8, 58, 152, 64, 174, 85, 30, 151, 98, 154, 26, 49, 132, 28, 60, 104, 249, 74, 135, 188, 109, 5, 191, 181, 72, 111, 140, 103, 75, 138, 239, 153, 238, 248, 160, 197, 137, 53, 105, 45, 56, 60, 68].into(),
            quorumIndices: vec![0,1].into(),
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
