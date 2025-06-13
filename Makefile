BUILD_DIR=contracts

.PHONY: all

all: containers

containers:
	docker compose up -d

clean:
	docker compose down
