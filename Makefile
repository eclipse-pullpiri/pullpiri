# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

.PHONY: default build
build:
	cargo build --manifest-path=src/common/Cargo.toml
	cargo build --manifest-path=src/agent/Cargo.toml
	cargo build --manifest-path=src/player/Cargo.toml
	cargo build --manifest-path=src/server/Cargo.toml
	cargo build --manifest-path=src/tools/Cargo.toml

.PHONY: release
release:
	cargo build --manifest-path=src/common/Cargo.toml --release
	cargo build --manifest-path=src/agent/Cargo.toml --release
	cargo build --manifest-path=src/player/Cargo.toml --release
	cargo build --manifest-path=src/server/Cargo.toml --release
	cargo build --manifest-path=src/tools/Cargo.toml --release

.PHONY: clean
clean:
	cargo clean --manifest-path=src/common/Cargo.toml
	cargo clean --manifest-path=src/agent/Cargo.toml
	cargo clean --manifest-path=src/player/Cargo.toml
	cargo clean --manifest-path=src/server/Cargo.toml
	cargo clean --manifest-path=src/tools/Cargo.toml

.PHONY: image
image:
	podman build -t localhost/pullpiri:latest -f containers/Dockerfile .

# command for DEVELOPMENT ONLY
.PHONY: builder
builder:
#	podman run --privileged --rm tonistiigi/binfmt --install all
#	podman buildx build --platform linux/amd64,linux/arm64 -t localhost/pullpiribuilder:latest -f containers/builder/Dockerfile-pullpiribuilder .
#	podman buildx build --platform linux/amd64,linux/arm64 -t localhost/pullpirirelease:latest -f containers/builder/Dockerfile-pullpirirelease .
	podman build -t localhost/pullpiribuilder:latest -f containers/dev/Dockerfile-pullpiribuilder .
	podman build -t localhost/pullpirirelease:latest -f containers/dev/Dockerfile-pullpirirelease .

# command for DEVELOPMENT ONLY
.PHONY: devimage
devimage:
	podman build -t localhost/pullpiri:dev -f containers/dev/Dockerfile .

# DO NOT USE THIS COMMAND IN PRODUCTION
# command for project owner
#.PHONY: pushbuilder
#pushbuilder:
#	docker buildx create --name container-builder --driver docker-container --bootstrap --use
#	docker run --privileged --rm tonistiigi/binfmt --install all
#	docker buildx build --push --platform linux/amd64,linux/arm64 -t ghcr.io/eclipse-pullpiri/pullpiribuilder:latest -f containers/builder/Dockerfile-pullpiribuilder .
#	docker buildx build --push --platform linux/amd64,linux/arm64 -t ghcr.io/eclipse-pullpiri/pullpirirelease:latest -f containers/builder/Dockerfile-pullpirirelease .

#.PHONY: pre
#pre:
#	-mkdir -p /etc/pullpiri/yaml
#	-mkdir -p /etc/containers/systemd/pullpiri/
#	-mkdir -p /etc/containers/systemd/pullpiri/etcd-data/
#	-podman-compose -f examples/nginx/docker-compose.yaml up -d

.PHONY: install
install:
	-mkdir -p /etc/pullpiri/yaml
	-mkdir -p /etc/containers/systemd/pullpiri/
	-mkdir -p /etc/containers/systemd/pullpiri/etcd-data/
	-cp -r ./src/settings.yaml /etc/containers/systemd/pullpiri/
	-cp -r ./containers/pullpiri-*.* /etc/containers/systemd/pullpiri/
	systemctl daemon-reload
	systemctl start pullpiri-server
	systemctl start pullpiri-agent
	systemctl start pullpiri-player

.PHONY: uninstall
uninstall:
	-systemctl stop pullpiri-agent
	-systemctl stop pullpiri-player
	-systemctl stop pullpiri-server
	systemctl daemon-reload
	-rm -rf /etc/pullpiri/yaml
	-rm -rf /etc/containers/systemd/*

#.PHONY: post
#post:
#	-rm -rf /etc/pullpiri/yaml
#	-rm -rf /etc/containers/systemd/*
#	systemctl daemon-reload
#	-podman-compose -f examples/nginx/docker-compose.yaml down

.PHONY: tools
tools:
	cargo build --manifest-path=src/tools/Cargo.toml --release