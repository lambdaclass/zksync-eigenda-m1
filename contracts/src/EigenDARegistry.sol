// ViewBypass.sol
pragma solidity ^0.8.20;

import {UUPSUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/UUPSUpgradeable.sol";
import {Ownable2StepUpgradeable} from "openzeppelin-contracts-upgradeable/contracts/access/Ownable2StepUpgradeable.sol";
import {Initializable} from "openzeppelin-contracts-upgradeable/contracts/proxy/utils/Initializable.sol";


interface IRiscZeroVerifier {
    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest) external view;
}

// Sends proofs to verify to the Risc0 groth16 verifier, storing whether they are verified, along with the hash of the blob
contract EigenDARegistry is Initializable, UUPSUpgradeable, Ownable2StepUpgradeable {
    
    IRiscZeroVerifier public risc0verifier;
    mapping (bytes => bool) public finishedBatches;
    mapping (bytes => bool) public verifiedBatches;
    mapping (bytes => bytes32) public hashes;

     /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    function initialize(address _risc0verifier, address _owner) public initializer {
        __Ownable2Step_init();
        __UUPSUpgradeable_init();
        _transferOwnership(_owner);
        risc0verifier = IRiscZeroVerifier(_risc0verifier);
    }

    function verify(bytes calldata seal, bytes32 imageId, bytes32 journalDigest, bytes32 eigendaHash, bytes calldata inclusion_data) public onlyOwner {
        try risc0verifier.verify(seal, imageId, journalDigest) {
            finishedBatches[inclusion_data] = true;
            verifiedBatches[inclusion_data] = true;
            hashes[inclusion_data] = eigendaHash;
        } catch {
            finishedBatches[inclusion_data] = true;
            verifiedBatches[inclusion_data] = false;
        }
    }

    function isVerified(bytes calldata inclusion_data) external view returns (bool, bytes32) {
        return (verifiedBatches[inclusion_data], hashes[inclusion_data]);
    }

    /// @dev Restricts upgrade permission to the contract owner
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {}
}
