BUILD_DIR=verifierWrapper/deployer

.PHONY: all build_contracts

all: build_contracts

build_contracts:
	cd $(BUILD_DIR) && forge build
