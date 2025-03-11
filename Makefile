BUILD_DIR=contracts

.PHONY: all build_contracts

all: build_contracts

build_contracts:
	cd $(BUILD_DIR) && forge build

deploy-risc0-verifier: clean
	git clone https://github.com/risc0/risc0-ethereum.git
	echo '--- a/contracts/script/DeployVerifier.s.sol' > /tmp/my_patch.diff; \
	echo '+++ b/contracts/script/DeployVerifier.s.sol' >> /tmp/my_patch.diff; \
	echo '@@ -33,7 +33,7 @@ contract DeployVerifier is Script {' >> /tmp/my_patch.diff; \
	echo '' >> /tmp/my_patch.diff; \
	echo '         vm.startBroadcast(deployerKey);' >> /tmp/my_patch.diff; \
	echo '' >> /tmp/my_patch.diff; \
	echo '-        IRiscZeroVerifier verifier = new RiscZeroGroth16Verifier(ControlID.CONTROL_ROOT, ControlID.BN254_CONTROL_ID);' >> /tmp/my_patch.diff; \
	echo '+        IRiscZeroVerifier verifier = new RiscZeroGroth16Verifier(hex"8cdad9242664be3112aba377c5425a4df735eb1c6966472b561d2855932c0469", ControlID.BN254_CONTROL_ID);' >> /tmp/my_patch.diff; \
	echo '         console2.log("Deployed RiscZeroGroth16Verifier to", address(verifier));' >> /tmp/my_patch.diff; \
	echo '' >> /tmp/my_patch.diff; \
	echo '         vm.stopBroadcast();' >> /tmp/my_patch.diff;
	cd risc0-ethereum && git apply /tmp/my_patch.diff && git submodule update --init --recursive && ETH_WALLET_PRIVATE_KEY=$(ETH_WALLET_PRIVATE_KEY) forge script contracts/script/DeployVerifier.s.sol:DeployVerifier --rpc-url $(RPC_URL) --broadcast -vvvv
	rm -f /tmp/my_patch.diffg
	rm -rf risc0-ethereum

clean:
	rm -rf risc0-ethereum
	rm -f /tmp/my_patch.diff
