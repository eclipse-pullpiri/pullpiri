# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

.PHONY: default build
build:
	cargo build --manifest-path=src/Cargo.toml

.PHONY: release
release:
	cargo build --manifest-path=src/Cargo.toml --release

.PHONY: clean
clean:
	cargo clean --manifest-path=src/Cargo.toml
	cargo clean --manifest-path=src/agent/nodeagent/Cargo.toml
	cargo clean --manifest-path=src/server/rocksdbservice/Cargo.toml
	cargo clean --manifest-path=src/tools/Cargo.toml

.PHONY: image
image:
	podman build -t localhost/pullpiri:latest -f containers/Dockerfile .

.PHONY: rocksdb-image
rocksdb-image:
	podman build -t localhost/pullpiri-rocksdb:latest -f src/server/rocksdbservice/Dockerfile .

.PHONY: nodeagent-bin
nodeagent-bin:
#	cargo build --manifest-path=src/agent/nodeagent/Cargo.toml --release --target=aarch64-unknown-linux-musl
	cargo build --manifest-path=src/agent/nodeagent/Cargo.toml --release --target=x86_64-unknown-linux-musl
	@echo "NodeAgent binary built at:"
	@echo "  ./target/release/nodeagent"

.PHONY: all-images
all-images: image rocksdb-image
	@echo "Built all container images:"
	@echo "  - localhost/pullpiri:latest (main services)"
	@echo "  - localhost/pullpiri-rocksdb:latest (RocksDB service)"

.PHONY: install
install:
	-./containers/install-piccolo.sh

.PHONY: uninstall
uninstall:
	-./containers/uninstall-piccolo.sh

# DO NOT USE THIS COMMAND IN PRODUCTION
#.PHONY: rocksdb-image
#rocksdb-image:
#	docker buildx create --name container-builder --driver docker-container --bootstrap --use
#	docker run --privileged --rm tonistiigi/binfmt --install all
#	docker buildx build --push --platform linux/amd64,linux/arm64 -t ghcr.io/mco-piccolo/pullpiri-rocksdb:v11.18.0 -f src/server/rocksdbservice/Dockerfile .

.PHONY: dev-install
dev-install:
	-./containers/devonly/install-piccolo.sh

.PHONY: dev-uninstall
dev-uninstall:
	-./containers/devonly/uninstall-piccolo.sh

.PHONY: tools
tools:
	cargo build --manifest-path=src/tools/Cargo.toml --release
	@echo ""
	@echo "=== Data Inspection ==="
	@echo "make build-inspector          - Build RocksDB Inspector tool"
	@echo "make inspect-rocksdb          - Inspect all RocksDB data"
	@echo "make verify-helloworld-data   - Verify helloworld test data"

.PHONY: test-rocksdb-service
test-rocksdb-service:
	@echo "Testing gRPC RocksDB Service..."
	@echo "Service should be running on localhost:47007"
	@echo "Use grpcurl to test the service:"
	@echo "  grpcurl -plaintext localhost:47007 rocksdbservice.RocksDbService/Health"
	@echo ""
	@echo "Building and running a simple test..."
	@cd src/server/rocksdbservice && cargo run -- --help
