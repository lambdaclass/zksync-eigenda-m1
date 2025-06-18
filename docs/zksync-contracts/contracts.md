In this [PR](https://github.com/matter-labs/era-contracts/pull/1405) the following contracts are added to zksync:

# [<span style="color:pink">EigenDAL1DAValidator.sol</span>](https://github.com/matter-labs/era-contracts/pull/1405/files#diff-c8ffe58186030899035f2943942d2a933d6d90566917a34e74495335c085cad6)

Implements the <span style="color:pink">checkDA</span> function, where it receives the <span style="color:pink">operatorDAInput</span> containing the EigenDA Inclusion Data. This is conformed by the risc zero proof (seal, imageID and journalDigest) plus the hash of the data dispersed to eigenda, calculated on the sidecar.

```solidity
struct EigenDAInclusionData {
    bytes seal;
    bytes32 imageId;
    bytes32 journalDigest;
    bytes32 eigenDAHash;
}
```

```solidity
function checkDA(
    uint256, // _chainId
    uint256, // _batchNumber,
    bytes32 l2DAValidatorOutputHash, // keccak(stateDiffHash, eigenDAHash) Calculated on EigenDAL2DAValidator and passed through L2->L1 Logs
    bytes calldata operatorDAInput, // stateDiffHash + inclusion_data (abi encoded EigenDAInclusionData)
    uint256 maxBlobsSupported
) external override returns (L1DAValidatorOutput memory output)
```

This contract checks against a <span style="color:pink">RiscZeroVerifier</span> if the Risc Zero Proof is correct.

```solidity
// Decode the inclusion data from the operatorDAInput
EigenDAInclusionData memory inclusionData = abi.decode(operatorDAInput[32:], (EigenDAInclusionData));

// Verify the risczero proof
risc0Verifier.verify(inclusionData.seal, inclusionData.imageId, inclusionData.journalDigest);
```

It also checks that the hash calculated on the sidecar is correct.

```solidity
// Check that the eigenDAHash from the Inclusion Data (originally calculated on Risc0 guest) is correct
if (l2DAValidatorOutputHash != keccak256(abi.encodePacked(stateDiffHash, inclusionData.eigenDAHash)))
    revert InvalidValidatorOutputHash();
```

Todo: We also need to check the Steel Commitment, this is contained in the <span style="color:pink">journalDigest</span>.

It is basically a comparison between the commitment  and the <span style="color:pink">blockHash</span> .

You can find more info [here](https://docs.beboundless.xyz/developers/steel/commitments#validation-of-steel-commitments).

# [<span style="color:pink">EigenDAL2DAValidator.sol</span>](https://github.com/matter-labs/era-contracts/pull/1405/files#diff-41149852d9965ba83ff78ea4f039ca5e74ec542cb5aead78166720895c2e184a)


Implements the <span style="color:pink">validatePubdata</span> function which calculates the <span style="color:pink">fullPubdataHash</span> which is then passed through L2→L1 Logs, and used to compare it against the sidecar generated hash on the <span style="color:pink">EigenDAL1DAValidator</span>

```solidity
/// EigenDA L2 DA validator. It will create a commitment to the pubdata that can later be verified during settlement.
contract EigenDAL2DAValidator is IL2DAValidator, StateDiffL2DAValidator {
    function validatePubdata(
        // The rolling hash of the user L2->L1 logs.
        bytes32,
        // The root hash of the user L2->L1 logs.
        bytes32,
        // The chained hash of the L2->L1 messages
        bytes32 _chainedMessagesHash,
        // The chained hash of uncompressed bytecodes sent to L1
        bytes32 _chainedBytecodesHash,
        // Operator data, that is related to the DA itself
        bytes calldata _totalL2ToL1PubdataAndStateDiffs
    ) external returns (bytes32 outputHash) {
        (bytes32 stateDiffHash, bytes calldata _totalPubdata, ) = _produceStateDiffPubdata(
            _chainedMessagesHash,
            _chainedBytecodesHash,
            _totalL2ToL1PubdataAndStateDiffs
        );

        bytes32 fullPubdataHash = keccak256(_totalPubdata);
        outputHash = keccak256(abi.encodePacked(stateDiffHash, fullPubdataHash));
    }
}
```

The rest of the changes on that PR are changes needed to deploy this new contracts.
