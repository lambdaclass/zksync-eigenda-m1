# Design

M1 consists of checking the inclusion of the blob and verifying that the data that is committed to is the correct one, this computations would be too heavy/costly to run directly on chain. An offchain implementation is needed in order to prevent this high costs. We resolve this by making a binary capable of running this checks in a provable way.

# Integration

The important components are marked in **bold**

## Step 1: Sequencer Dispersal and Inclusion data retrieval (Marked in Blue) & Sidecar Proof Generation (Marked in red)

![Step 1](../../images/step1.png)

1. Zksync's sequencer finishes a batch and wants to disperse its content (**Blob Data**).
2. Zksync's sequencer sends the blob to be dispersed to EigenDA, EigenDA returns the **Blob Key**.
3. Zksync's sequencer sends the **Blob key** to the Sidecar.
4. Zksync's sequencer stores the **Blob Key** in its database.
5. Zksync’s sequencer asks for **Inlcusion Data** (encoded **EigenDACert**) to EigenDA.
6. Zksync’s sequencer starts waiting for Sidecar **Blob Key** proof to be generated.
7. Sidecar asks EigenDA for **EigenDACert** and **Blob Data** (this step runs parallel to step 4)
8. Sidecar executes Risc0, doing 3 things:
    a. Call to checkDACert
    b. Proof of Equivalence
    c. Calculation of **EigenDAHash** (keccak of *BlobData*)
    
    And generates a **Risc0 Proof** of those 3 things.
    
9. Zksync’s sequencer finishes waiting for proof (step 6), storing the retrieved proof in its database. It then calls the Commit Batches function of Executor (zksync’s DiamondProxy implementation)on Ethereum.

## Step 2 Commit Batches (Marked in Green)

Everything here runs on Ethereum

![Step 2](../../images/step2.png)

1. Executor starts commit batches function.
2. Executor calls the EigenDAL1DAValidator checkDA function with **l2DAValidatorOutputHash** and **operatorDAInput** as parameter
    1. **l2DAValidatorOutputHash**: keccak(**stateDiffHash** + **EigenDAHash**)
    2. **operatorDAInput**: **StateDiffHash** + **Inclusion Data** (seal + imageId + journalDigest + eigenDAHash)
    
    💡 (**stateDiffHash** is the hash of the states diffs, calculated on EigenDAL2Validator and sent to L1 through L2→L1 Logs)
    
3. EigenDAL1DAValidator calls risc0Verifier `verify` function with **inclusionData.seal**, **inclusionData.imageId**, **inclusionData.journalDigest** as parameters, which is expected to **not revert** upon succesful verification.
4. EigenDAL1DAValidator checks if keccak(**stateDiffHash** + **inclusionData.EigenDAHash**) equals **l2DAValidatorOutputHash** (meaning that if not, **EigenDAHash** was not correctly calculated by the sidecar).
