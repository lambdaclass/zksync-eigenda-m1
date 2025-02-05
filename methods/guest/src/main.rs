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

use alloy_primitives::{address, Address, Bytes, FixedBytes, U256};
use alloy_sol_types::sol;
use erc20_guest::blob_info::{BlobInfo, BlobQuorumParam};
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

impl From<BlobQuorumParam> for QuorumBlobParam {
    fn from(param: BlobQuorumParam) -> Self {
        QuorumBlobParam {
            quorumNumber: param.quorum_number as u8,
            adversaryThresholdPercentage: param.adversary_threshold_percentage as u8,
            confirmationThresholdPercentage: param.confirmation_threshold_percentage as u8,
            chunkLength: param.chunk_length,
        }
    }
}

impl From<erc20_guest::blob_info::BlobHeader> for BlobHeader {
    fn from(blob_header: erc20_guest::blob_info::BlobHeader) -> Self {
        let x: [u8;32] = blob_header.commitment.x.try_into().expect("slice with incorrect length");
        let y: [u8;32]  = blob_header.commitment.y.try_into().expect("slice with incorrect length");
        BlobHeader {
            commitment: G1Point {
                x: U256::from_be_bytes(x),
                y: U256::from_be_bytes(y),
            },
            dataLength: blob_header.data_length,
            quorumBlobParams: blob_header.blob_quorum_params.iter().map(|param| QuorumBlobParam::from(param.clone())).collect(),
        }
    }
}

impl From<erc20_guest::blob_info::G1Commitment > for G1Point {
    fn from(commitment: erc20_guest::blob_info::G1Commitment) -> Self {
        let x: [u8;32] = commitment.x.try_into().expect("slice with incorrect length");
        let y: [u8;32]  = commitment.y.try_into().expect("slice with incorrect length");
        G1Point {
            x: U256::from_be_bytes(x),
            y: U256::from_be_bytes(y),
        }
    }
}

impl From<erc20_guest::blob_info::BatchHeader> for BatchHeader {
    fn from(batch_header: erc20_guest::blob_info::BatchHeader) -> Self {
        let root: [u8;32] = batch_header.batch_root.try_into().expect("slice with incorrect length");
        BatchHeader {
            blobHeadersRoot: FixedBytes::from(root),
            quorumNumbers: Bytes::from(batch_header.quorum_numbers),
            signedStakeForQuorums: Bytes::from(batch_header.quorum_signed_percentages),
            referenceBlockNumber: batch_header.reference_block_number,
        }
    }
}

impl From<erc20_guest::blob_info::BatchMetadata> for BatchMetadata {
    fn from(batch_metadata: erc20_guest::blob_info::BatchMetadata) -> Self {
        let header: BatchHeader = BatchHeader::from(batch_metadata.batch_header);
        let signatory_record_hash: [u8;32] = batch_metadata.signatory_record_hash.try_into().expect("slice with incorrect length");
        BatchMetadata {
            batchHeader: header,
            signatoryRecordHash: FixedBytes::from(signatory_record_hash),
            confirmationBlockNumber: batch_metadata.confirmation_block_number,
        }
    }
}

impl From<erc20_guest::blob_info::BlobVerificationProof> for BlobVerificationProof {
    fn from(blob_verification_proof: erc20_guest::blob_info::BlobVerificationProof) -> Self {
        let metadata: BatchMetadata = BatchMetadata::from(blob_verification_proof.batch_medatada);
        BlobVerificationProof {
            batchId: blob_verification_proof.batch_id,
            blobIndex: blob_verification_proof.blob_index,
            batchMetadata: metadata,
            inclusionProof: Bytes::from(blob_verification_proof.inclusion_proof),
            quorumIndices: Bytes::from(blob_verification_proof.quorum_indexes),
        }
    }
}

fn main() {
    // Read the input from the guest environment.
    let input: EthEvmInput = env::read();
    let blob_info: BlobInfo = env::read();

    // Converts the input into a `EvmEnv` for execution. The `with_chain_spec` method is used
    // to specify the chain configuration. It checks that the state matches the state root in the
    // header provided in the input.
    let env = input.into_env();
    // Commit the block hash and number used when deriving `EvmEnv` to the journal.
    env::commit_slice(&env.commitment().abi_encode());

    // Execute the view call; it returns the result in the type generated by the `sol!` macro.
    let contract = Contract::new(CONTRACT, &env);
    let call = IVerifyBlob::verifyBlobV1Call{blobHeader: blob_info.blob_header.into(), blobVerificationProof: blob_info.blob_verification_proof.into()};
    let returns = contract.call_builder(&call).from(CALLER).call();
    println!("View call result: {}", returns._0);
}
