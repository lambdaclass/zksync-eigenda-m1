BUILD_DIR=contracts

.PHONY: all build_contracts

all: build_contracts containers

containers:
	docker compose up -d

build_contracts:
	cd $(BUILD_DIR) && forge build

clean:
	cd $(BUILD_DIR) && forge clean
	docker compose down
