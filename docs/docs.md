M1 consists of checking the inclusion of the blob and verifying that the data that is committed to is the correct one, this computations would be too heavy/costly to run directly on chain. An offchain implementation is needed in order to prevent this high costs. We resolve this by making a binary capable of running this checks in a provable way.

This folder contains documentation for

- [Zksync-contracts](./zksync-contracts/contracts.md)
- [Zksync-era sequencer](./zksync-era/sequencer.md)
- [Sidecar](./sidecar/sidecar.md)
- [Components integration](./components-integration/components-integration.md)
