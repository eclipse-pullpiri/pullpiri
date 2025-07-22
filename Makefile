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
#	-mkdir -p /etc/piccolo/yaml
#	-mkdir -p /etc/containers/systemd/piccolo/
#	-mkdir -p /etc/containers/systemd/piccolo/etcd-data/
#	-podman-compose -f examples/nginx/docker-compose.yaml up -d

.PHONY: install
install:
	-mkdir -p /etc/piccolo/yaml
	-mkdir -p /etc/containers/systemd/piccolo/
	-mkdir -p /etc/containers/systemd/piccolo/etcd-data/
	-cp -r ./src/settings.yaml /etc/containers/systemd/piccolo/
	-cp -r ./containers/piccolo-*.* /etc/containers/systemd/piccolo/
	systemctl daemon-reload
	systemctl start piccolo-server
	systemctl start piccolo-agent
	systemctl start piccolo-player

.PHONY: uninstall
uninstall:
	-systemctl stop piccolo-agent
	-systemctl stop piccolo-player
	-systemctl stop piccolo-server
	systemctl daemon-reload
	-rm -rf /etc/piccolo/yaml
	-rm -rf /etc/containers/systemd/*

#.PHONY: post
#post:
#	-rm -rf /etc/piccolo/yaml
#	-rm -rf /etc/containers/systemd/*
#	systemctl daemon-reload
#	-podman-compose -f examples/nginx/docker-compose.yaml down

.PHONY: tools
tools:
	cargo build --manifest-path=src/tools/Cargo.toml --release