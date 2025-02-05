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

use alloy_primitives::{address, Address, Bytes, FixedBytes, U256};
use alloy_sol_types::{sol, SolCall, SolType};
use anyhow::{Context, Result};
use clap::Parser;
use erc20_methods::ERC20_GUEST_ELF;
use ethabi::{ParamType, Token};
use host::blob_info::{BlobQuorumParam, G1Commitment};
use risc0_steel::{ethereum::EthEvmEnv, Commitment, Contract};
use risc0_zkvm::{default_executor, ExecutorEnv};
use tokio_postgres::NoTls;
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

// impl std::fmt::Debug for QuorumBlobParam {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         f.debug_struct("QuorumBlobParam")
//             .field("quorumNumber", &self.quorumNumber)
//             .field("adversaryThresholdPercentage", &self.adversaryThresholdPercentage)
//             .field("confirmationThresholdPercentage", &self.confirmationThresholdPercentage)
//             .field("chunkLength", &self.chunkLength)
//             .finish()
//     }
// }

// // Manually implement Debug for `G1Point`
// impl std::fmt::Debug for G1Point {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         f.debug_struct("G1Point")
//             .field("x", &self.x)
//             .field("y", &self.y)
//             .finish()
//     }
// }

// impl std::fmt::Debug for BlobHeader {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         f.debug_struct("BlobHeader")
//             .field("commitment", &self.commitment)
//             .field("dataLength", &self.dataLength)
//             .field("quorumBlobParams", &self.quorumBlobParams)
//             .finish()
//     }
// }

// impl std::fmt::Debug for BatchHeader {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         f.debug_struct("BatchHeader")
//             .field("blobHeadersRoot", &self.blobHeadersRoot)
//             .field("quorumNumbers", &self.quorumNumbers)
//             .field("signedStakeForQuorums", &self.signedStakeForQuorums)
//             .field("referenceBlockNumber", &self.referenceBlockNumber)
//             .finish()
//     }
// }

// impl std::fmt::Debug for BatchMetadata {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         f.debug_struct("BatchMetadata")
//             .field("batchHeader", &self.batchHeader)
//             .field("signatoryRecordHash", &self.signatoryRecordHash)
//             .field("confirmationBlockNumber", &self.confirmationBlockNumber)
//             .finish()
//     }
// }

// impl std::fmt::Debug for BlobVerificationProof {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         f.debug_struct("BlobVerificationProof")
//             .field("batchId", &self.batchId)
//             .field("blobIndex", &self.blobIndex)
//             .field("batchMetadata", &self.batchMetadata)
//             .field("inclusionProof", &self.inclusionProof)
//             .field("quorumIndices", &self.quorumIndices)
//             .finish()
//     }
// }

impl From<G1Point> for G1Commitment {
    fn from(point: G1Point) -> Self {
        G1Commitment {
            x: point.x.to_be_bytes_vec(),
            y: point.y.to_be_bytes_vec(),
        }
    }
}

impl From<QuorumBlobParam> for host::blob_info::BlobQuorumParam {
    fn from(param: QuorumBlobParam) -> Self {
        host::blob_info::BlobQuorumParam {
            quorum_number: param.quorumNumber as u32,
            adversary_threshold_percentage: param.adversaryThresholdPercentage as u32,
            confirmation_threshold_percentage: param.confirmationThresholdPercentage as u32,
            chunk_length: param.chunkLength,
        }
    }
}

impl From<BlobHeader> for host::blob_info::BlobHeader {
    fn from(header: BlobHeader) -> Self {
        host::blob_info::BlobHeader {
            commitment: header.commitment.into(),
            data_length: header.dataLength,
            blob_quorum_params: header
                .quorumBlobParams
                .iter()
                .map(|param| BlobQuorumParam::from(param.clone()))
                .collect(),
        }
    }
}

impl From<BatchHeader> for host::blob_info::BatchHeader {
    fn from(header: BatchHeader) -> Self {
        host::blob_info::BatchHeader {
            batch_root: header.blobHeadersRoot.to_vec(),
            quorum_numbers: header.quorumNumbers.to_vec(),
            quorum_signed_percentages: header.signedStakeForQuorums.to_vec(),
            reference_block_number: header.referenceBlockNumber,
        }
    }
}

impl From<BatchMetadata> for host::blob_info::BatchMetadata {
    fn from(metadata: BatchMetadata) -> Self {
        host::blob_info::BatchMetadata {
            batch_header: host::blob_info::BatchHeader::from(metadata.batchHeader),
            signatory_record_hash: metadata.signatoryRecordHash.to_vec(),
            confirmation_block_number: metadata.confirmationBlockNumber,
            fee: vec![],
            batch_header_hash: vec![],
        }
    }
}

impl From<BlobVerificationProof> for host::blob_info::BlobVerificationProof {
    fn from(proof: BlobVerificationProof) -> Self {
        host::blob_info::BlobVerificationProof {
            batch_id: proof.batchId,
            blob_index: proof.blobIndex,
            batch_medatada: host::blob_info::BatchMetadata::from(proof.batchMetadata),
            inclusion_proof: proof.inclusionProof.to_vec(),
            quorum_indexes: proof.quorumIndices.to_vec(),
        }
    }
}

/// Address of the deployed contract to call the function on (USDT contract on Sepolia).
const CONTRACT: Address = address!("c551b009C1CE0b6efD691E23998AEFd4103680D3"); //TODO: Add the address of the deployed contract.
/// Address of the caller.
const CALLER: Address = address!("e706e60ab5Dc512C36A4646D719b889F398cbBcB");

/// Simple program to show the use of Ethereum contract data inside the guest.
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// URL of the RPC endpoint
    #[arg(short, long, env = "RPC_URL")]
    rpc_url: Url,
}

/*
{ blob_header: BlobHeader { commitment: G1Commitment {
 x: [45, 163, 103, 107, 115, 104, 190, 164, 41, 43, 142, 248, 220, 94, 73, 59, 97, 143, 72, 126, 15, 42, 253, 124, 61, 153, 201, 125, 98, 181, 166, 183],
 y: [28, 34, 218, 19, 64, 28, 232, 2, 67, 176, 202, 174, 47, 109, 8, 82, 142, 50, 113, 145, 89, 96, 95, 110, 150, 176, 89, 199, 156, 203, 37, 217] },
 data_length: 172, blob_quorum_params: [BlobQuorumParam
 { quorum_number: 0, adversary_threshold_percentage: 33, confirmation_threshold_percentage: 55, chunk_length: 1 },
   BlobQuorumParam { quorum_number: 1, adversary_threshold_percentage: 33, confirmation_threshold_percentage: 55, chunk_length: 1 }] },
   blob_verification_proof: BlobVerificationProof {
   batch_id: 5, blob_index: 14, batch_medatada: BatchMetadata { batch_header: BatchHeader {
   batch_root: [140, 16, 0, 179, 206, 12, 33, 157, 172, 132, 187, 157, 72, 93, 160, 175, 206, 235, 185, 213, 180, 10, 45, 11, 147, 151, 173, 18, 149, 112, 112, 140],
   quorum_numbers: [0, 1], quorum_signed_percentages: [100, 100], reference_block_number: 330 },
   signatory_record_hash: [196, 252, 126, 62, 61, 79, 109, 122, 7, 34, 213, 99, 227, 20, 65, 77, 159, 119, 30, 123, 176, 50, 176, 229, 12, 62, 199, 111, 245, 144, 143, 225], fee: [0],
   confirmation_block_number: 372,
   batch_header_hash: [40, 123, 220, 251, 188, 171, 175, 6, 207, 110, 116, 34, 157, 25, 119, 19, 76, 103, 227, 38, 54, 34, 30, 92, 202, 33, 129, 73, 68, 153, 32, 64] },
   inclusion_proof: [36, 109, 130, 123, 93, 243, 227, 27, 197, 163, 32, 132, 160, 31, 141, 140, 124, 225, 11, 212, 194, 220, 194, 212, 97, 115, 106, 164, 43, 131, 114, 193, 72, 255, 203, 148, 113, 127, 22, 227, 208, 91, 216, 45, 20, 214, 190, 22, 9, 174, 1, 52, 237, 84, 187, 105, 131, 169, 125, 35, 160, 201, 123, 30, 236, 24, 26, 172, 246, 91, 10, 207, 253, 183, 85, 17, 59, 99, 244, 240, 158, 154, 167, 109, 219, 196, 181, 30, 127, 20, 72, 214, 214, 121, 17, 221, 73, 137, 98, 107, 92, 228, 227, 219, 233, 195, 102, 114, 23, 168, 116, 163, 140, 223, 209, 45, 207, 224, 70, 188, 195, 209, 245, 219, 211, 101, 198, 242, 13, 124, 117, 9, 88, 117, 193, 224, 59, 166, 83, 196, 200, 228, 140, 191, 135, 226, 106, 24, 121, 182, 15, 96, 87, 140, 36, 88, 184, 85, 130, 238],
   quorum_indexes: [0, 1] } }
 */
fn extract_tuple(token: &Token) -> anyhow::Result<&Vec<Token>> {
    match token {
        Token::Tuple(inner) => Ok(inner),
        _ => Err(anyhow::anyhow!("Not a tuple")),
    }
}

fn extract_array(token: &Token) -> anyhow::Result<Vec<Token>> {
    match token {
        Token::Array(tokens) => Ok(tokens.clone()),
        _ => Err(anyhow::anyhow!("Not a uint")),
    }
}

fn extract_uint32(token: &Token) -> anyhow::Result<u32> {
    match token {
        Token::Uint(value) => Ok(value.as_u32()),
        _ => Err(anyhow::anyhow!("Not a uint")),
    }
}

fn extract_uint8(token: &Token) -> anyhow::Result<u8> {
    match token {
        Token::Uint(value) => Ok(value.as_u32() as u8),
        _ => Err(anyhow::anyhow!("Not a uint")),
    }
}

fn extract_fixed_bytes<const N: usize>(token: &Token) -> anyhow::Result<FixedBytes<32>> {
    match token {
        Token::FixedBytes(bytes) => Ok(FixedBytes::from_slice(bytes)),
        _ => Err(anyhow::anyhow!("Not fixed bytes")),
    }
}

fn extract_bytes(token: &Token) -> anyhow::Result<Bytes> {
    match token {
        Token::Bytes(bytes) => Ok(Bytes::from_iter(bytes)),
        _ => Err(anyhow::anyhow!("Not bytes")),
    }
}

fn decode_blob_info(
    inclusion_data: Vec<u8>,
) -> Result<(BlobHeader, BlobVerificationProof), anyhow::Error> {
    let param_types = vec![ParamType::Tuple(vec![
        // BlobHeader
        ParamType::Tuple(vec![
            ParamType::Tuple(vec![ParamType::Uint(256), ParamType::Uint(256)]), // G1Commitment
            ParamType::Uint(32),                                                // data_length
            ParamType::Array(Box::new(ParamType::Tuple(vec![
                ParamType::Uint(32),
                ParamType::Uint(32),
                ParamType::Uint(32),
                ParamType::Uint(32),
            ]))), // BlobQuorumParam
        ]),
        // BlobVerificationProof
        ParamType::Tuple(vec![
            ParamType::Uint(32), // batch_id
            ParamType::Uint(32), // blob_index
            ParamType::Tuple(vec![
                ParamType::Tuple(vec![
                    ParamType::FixedBytes(32),
                    ParamType::Bytes,
                    ParamType::Bytes,
                    ParamType::Uint(32),
                ]), // BatchHeader
                ParamType::FixedBytes(32), // signatory_record_hash
                ParamType::Uint(32),       // confirmation_block_number
                ParamType::Bytes,          // batch_header_hash
                ParamType::Bytes,          // fee
            ]), // BatchMetadata
            ParamType::Bytes,    // inclusion_proof
            ParamType::Bytes,    // quorum_indexes
        ]),
    ])];

    let decoded = ethabi::decode(&param_types, &inclusion_data)?;
    let blob_info = extract_tuple(&decoded[0])?;

    // Extract BlobHeader
    let blob_header_tokens = extract_tuple(&blob_info[0])?;
    let commitment_tokens = extract_tuple(&blob_header_tokens[0])?;

    let x = commitment_tokens[0].clone().into_uint().unwrap();
    let y = commitment_tokens[1].clone().into_uint().unwrap();

    let mut x_bytes = [0u8; 32];
    let mut y_bytes = [0u8; 32];
    x.to_big_endian(&mut x_bytes);
    y.to_big_endian(&mut y_bytes);

    let data_length = extract_uint32(&blob_header_tokens[1])?;
    let blob_quorum_params_tokens = extract_array(&blob_header_tokens[2])?;

    let blob_quorum_params: Vec<QuorumBlobParam> = blob_quorum_params_tokens
        .iter()
        .map(|param| {
            let tuple = extract_tuple(param)?;
            Ok(QuorumBlobParam {
                quorumNumber: extract_uint8(&tuple[0])?,
                adversaryThresholdPercentage: extract_uint8(&tuple[1])?,
                confirmationThresholdPercentage: extract_uint8(&tuple[2])?,
                chunkLength: extract_uint32(&tuple[3])?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    let blob_header = BlobHeader {
        commitment: G1Point {
            x: U256::from_be_bytes(x_bytes),
            y: U256::from_be_bytes(y_bytes),
        },
        dataLength: data_length,
        quorumBlobParams: blob_quorum_params,
    };

    // Extract BlobVerificationProof
    let blob_verification_tokens = extract_tuple(&blob_info[1])?;

    let batch_id = extract_uint32(&blob_verification_tokens[0])?;
    let blob_index = extract_uint32(&blob_verification_tokens[1])?;

    let batch_metadata_tokens = extract_tuple(&blob_verification_tokens[2])?;
    let batch_header_tokens = extract_tuple(&batch_metadata_tokens[0])?;

    let batch_header = BatchHeader {
        blobHeadersRoot: extract_fixed_bytes::<32>(&batch_header_tokens[0])?,
        quorumNumbers: extract_bytes(&batch_header_tokens[1])?,
        signedStakeForQuorums: extract_bytes(&batch_header_tokens[2])?,
        referenceBlockNumber: extract_uint32(&batch_header_tokens[3])?,
    };

    let batch_metadata = BatchMetadata {
        batchHeader: batch_header,
        signatoryRecordHash: extract_fixed_bytes::<32>(&batch_metadata_tokens[1])?,
        confirmationBlockNumber: extract_uint32(&batch_metadata_tokens[2])?,
    };

    let blob_verification_proof = BlobVerificationProof {
        batchId: batch_id,
        blobIndex: blob_index,
        batchMetadata: batch_metadata,
        inclusionProof: extract_bytes(&blob_verification_tokens[3])?,
        quorumIndices: extract_bytes(&blob_verification_tokens[4])?,
    };

    Ok((blob_header, blob_verification_proof))
}

#[tokio::main]
async fn main() -> Result<()> {
    let (client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres password=notsecurepassword dbname=zksync_server_localhost_eigenda", 
        NoTls,
    ).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let rows = client
        .query("SELECT inclusion_data FROM data_availability", &[])
        .await?;

    for row in rows {
        let inclusion_data: Vec<u8> = row.get(0);
        let (blob_header, blob_verification_proof) = decode_blob_info(inclusion_data)?;

        let call = IVerifyBlob::verifyBlobV1Call {
            blobHeader: blob_header.clone(),
            blobVerificationProof: blob_verification_proof.clone(),
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

        // Preflight the call to prepare the input that is required to execute the function in
        // the guest without RPC access. It also returns the result of the call.
        let mut contract = Contract::preflight(CONTRACT, &mut env);
        let returns = contract.call_builder(&call).from(CALLER).call().await?;
        println!(
            "Call {} Function by {:#} on {:#} returns: {}",
            IVerifyBlob::verifyBlobV1Call::SIGNATURE,
            CALLER,
            CONTRACT,
            returns._0
        );

        // Finally, construct the input from the environment.
        let input = env.into_input().await?;

        let blob_info = host::blob_info::BlobInfo {
            blob_header: blob_header.into(),
            blob_verification_proof: blob_verification_proof.into(),
        };

        println!("Running the guest with the constructed input...");
        let session_info = {
            let env = ExecutorEnv::builder()
                .write(&input)
                .unwrap()
                .write(&blob_info)
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
    }
    Ok(())
}
