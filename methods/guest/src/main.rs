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

#![allow(unused_doc_comments)]
#![no_main]

use alloy_primitives::{address, Address, U256};
use alloy_sol_types::sol;
use risc0_steel::{
    ethereum::{EthEvmInput},
    Contract,
};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

/// Specify the function to call using the [`sol!`] macro.
/// This parses the Solidity syntax to generate a struct that implements the `SolCall` trait.
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
const CONTRACT: Address = address!("c551b009C1CE0b6efD691E23998AEFd4103680D3");
/// Address of the caller. If not provided, the caller will be the [CONTRACT].
const CALLER: Address = address!("f08A50178dfcDe18524640EA6618a1f965821715");

fn main() {


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
    // Read the input from the guest environment.
    let input: EthEvmInput = env::read();

    // Converts the input into a `EvmEnv` for execution. The `with_chain_spec` method is used
    // to specify the chain configuration. It checks that the state matches the state root in the
    // header provided in the input.
    let env = input.into_env();//.with_chain_spec(&ETH_SEPOLIA_CHAIN_SPEC);
    // Commit the block hash and number used when deriving `EvmEnv` to the journal.
    env::commit_slice(&env.commitment().abi_encode());

    // Execute the view call; it returns the result in the type generated by the `sol!` macro.
    let contract = Contract::new(CONTRACT, &env);
    let returns = contract.call_builder(&CALL).from(CALLER).call();
    println!("View call result: {}", returns._0);
}
