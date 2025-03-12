BUILD_DIR=contracts

.PHONY: all build_contracts

all: build_contracts

build_contracts:
	cd $(BUILD_DIR) && forge build
