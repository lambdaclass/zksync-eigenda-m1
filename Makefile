.PHONY: all

start-deps:
	docker compose up -d

stop-deps:
	docker compose down

deploy-risc0-verifier-contract-anvil:
	forge script contracts/script/ContractsDeployer.s.sol:ContractsDeployer --rpc-url localhost:8545 --broadcast -vvvv