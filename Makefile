BUILD_DIR=contracts

.PHONY: all build_contracts

all: build_contracts database

database:
	docker compose up -d

build_contracts:
	cd $(BUILD_DIR) && forge build
