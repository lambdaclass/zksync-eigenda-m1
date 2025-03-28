# Contracts

This directory contains the necessary contracts for the Blob Verification to be performed, this are:

## BlobVerifierWrapper

It wrapps the `BlobVerifier` contract from eigenda, since we need for the `verifyBlobV1` function to return a value.

## EigenDARegistry

This calls the Risc0 Groth16 verifier contract, which verifies the proof, and then stores whether they were correctly verified, along with the hash of the blob for a given inclusion data.

# Scripts

There are also scripts that are used to make use of the contracts easier.

## BlobVerifierWrapperDeployer

Deploys the `BlobVerifierWrapper`

## DeployRiscZeroGroth16Verifier

Deploys the risc0 groth16 verifier using the risc0-ethereum contracts

## EigenDARegistryDeployer

Deploys the `EigenDARegistry`
