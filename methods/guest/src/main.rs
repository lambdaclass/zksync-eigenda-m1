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
const CALLER: Address = address!("e706e60ab5Dc512C36A4646D719b889F398cbBcB");

fn main() {


    let CALL: IVerifyBlob::verifyBlobV1Call = IVerifyBlob::verifyBlobV1Call {
        blobHeader: BlobHeader {
            commitment: G1Point {
                x: U256::from_be_bytes([24, 169, 164, 102, 107, 160, 232, 179, 235, 137, 210, 187, 41, 80, 125, 253, 139, 173, 199, 13, 1, 202, 187, 76, 194, 248, 111, 119, 72, 11, 18, 57]),
                y: U256::from_be_bytes([36, 23, 31, 142, 207, 119, 161, 176, 17, 168, 92, 30, 153, 172, 247, 0, 49, 158, 53, 162, 100, 199, 15, 59, 191, 73, 208, 167, 100, 195, 235, 100]),
            },
            dataLength: 172,
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
            batchId: 7,
            blobIndex: 2,
            batchMetadata: BatchMetadata {
                batchHeader: BatchHeader {
                    blobHeadersRoot: U256::from_be_bytes([196, 174, 24, 254, 20, 180, 213, 225, 117, 122, 48, 201, 24, 133, 138, 147, 63, 121, 141, 0, 219, 225, 211, 235, 234, 106, 246, 117, 125, 12, 248, 228]).into(),
                    quorumNumbers: vec![0,1].into(),
                    signedStakeForQuorums: vec![100,100].into(),
                    referenceBlockNumber: 411,
                },
                signatoryRecordHash: U256::from_be_bytes([254, 21, 202, 76, 140, 76, 68, 247, 165, 151, 115, 92, 149, 210, 175, 251, 11, 113, 131, 122, 72, 171, 7, 17, 212, 145, 50, 88, 64, 246, 246, 190]).into(),
                confirmationBlockNumber: 452,
            },
            inclusionProof: vec![147, 50, 188, 194, 143, 168, 26, 54, 9, 187, 208, 22, 1, 40, 156, 176, 116, 204, 136, 143, 155, 94, 59, 6, 16, 121, 87, 4, 172, 198, 181, 117, 29, 210, 56, 81, 44, 108, 216, 99, 54, 8, 148, 87, 5, 252, 149, 13, 39, 229, 222, 241, 152, 102, 210, 68, 104, 102, 95, 9, 162, 100, 57, 123, 141, 39, 81, 14, 44, 37, 89, 111, 181, 30, 5, 86, 0, 198, 228, 1, 253, 156, 136, 44, 200, 63, 159, 180, 144, 142, 158, 230, 134, 157, 109, 22, 70, 170, 188, 137, 243, 129, 174, 254, 159, 239, 140, 38, 186, 120, 145, 254, 206, 186, 32, 84, 130, 160, 25, 86, 8, 129, 81, 33, 36, 91, 123, 122, 137, 249, 0, 148, 175, 28, 22, 175, 153, 149, 72, 14, 224, 165, 247, 100, 2, 134, 114, 81, 104, 141, 47, 114, 42, 205, 219, 24, 57, 11, 248, 149].into(),
            quorumIndices: vec![0,1].into(),
        }
    };
    // Read the input from the guest environment.
    let input: EthEvmInput = env::read();

    // Converts the input into a `EvmEnv` for execution. The `with_chain_spec` method is used
    // to specify the chain configuration. It checks that the state matches the state root in the
    // header provided in the input.
    let env = input.into_env();
    // Commit the block hash and number used when deriving `EvmEnv` to the journal.
    env::commit_slice(&env.commitment().abi_encode());

    // Execute the view call; it returns the result in the type generated by the `sol!` macro.
    let contract = Contract::new(CONTRACT, &env);
    let returns = contract.call_builder(&CALL).from(CALLER).call();
    println!("View call result: {}", returns._0);
}
