# Contracts

This directory contains the necessary contracts for the Blob Verification to be performed, this are:

## BlobVerifierWrapper

It wrapps the `BlobVerifier` contract from eigenda, since we need for the `verifyBlobV1` function to return a value.

## EigenDARegistry

This wrapps the Risc0 Groth16 verifier contract, this is in order to make the function `non-view`, so we can create a transaction when calling it.

# Scripts

There are also scripts that are used to make use of the contracts easier.

## BlobVerifierWrapperDeployer

Deploys the `BlobVerifierWrapper`

## EigenDARegistryDeployer

Deploys the `EigenDARegistry`
