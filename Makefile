NETWORK ?= testnet

.PHONY: build test deploy clean

build:
	cargo build --target wasm32-unknown-unknown --release

test:
	cargo test

deploy:
	@echo "Deploying contracts to $(NETWORK)..."
	./scripts/deploy.sh $(NETWORK)

clean:
	cargo clean
	rm -rf target/wasm32-unknown-unknown
