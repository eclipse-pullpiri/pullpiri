# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

.PHONY: default build
build:
	cargo build --manifest-path=src/common/Cargo.toml
	cargo build --manifest-path=src/observer/Cargo.toml
	cargo build --manifest-path=src/player/Cargo.toml
	cargo build --manifest-path=src/server/Cargo.toml
	cargo build --manifest-path=src/tools/Cargo.toml

.PHONY: release
release:
	cargo build --manifest-path=src/common/Cargo.toml --release
	cargo build --manifest-path=src/observer/Cargo.toml --release
	cargo build --manifest-path=src/player/Cargo.toml --release
	cargo build --manifest-path=src/server/Cargo.toml --release
	cargo build --manifest-path=src/tools/Cargo.toml --release

.PHONY: clean
clean:
	cargo clean --manifest-path=src/common/Cargo.toml
	cargo clean --manifest-path=src/observer/Cargo.toml
	cargo clean --manifest-path=src/player/Cargo.toml
	cargo clean --manifest-path=src/server/Cargo.toml
	cargo clean --manifest-path=src/tools/Cargo.toml

.PHONY: image
image:
	podman build -t localhost/pullpiri-observer:latest -f containers/Dockerfile-observer .
	podman build -t localhost/pullpiri-player:latest -f containers/Dockerfile-player .
	podman build -t localhost/pullpiri-server:latest -f containers/Dockerfile-server .

# command for dev

.PHONY: builder
builder:
#	podman run --privileged --rm tonistiigi/binfmt --install all
#	podman buildx build --platform linux/amd64,linux/arm64 -t localhost/pullpiribuilder:latest -f containers/builder/Dockerfile-pullpiribuilder .
#	podman buildx build --platform linux/amd64,linux/arm64 -t localhost/pullpirirelease:latest -f containers/builder/Dockerfile-pullpirirelease .
	podman build -t localhost/pullpiribuilder:latest -f containers/builder/Dockerfile-pullpiribuilder .
	podman build -t localhost/pullpirirelease:latest -f containers/builder/Dockerfile-pullpirirelease .

.PHONY: pushbuilder
pushbuilder:
	docker buildx create --name container-builder --driver docker-container --bootstrap --use
	docker run --privileged --rm tonistiigi/binfmt --install all
	docker buildx build --push --platform linux/amd64,linux/arm64 -t ghcr.io/eclipse-pullpiri/pullpiribuilder:latest -f containers/builder/Dockerfile-pullpiribuilder .
	docker buildx build --push --platform linux/amd64,linux/arm64 -t ghcr.io/eclipse-pullpiri/pullpirirelease:latest -f containers/builder/Dockerfile-pullpirirelease .

.PHONY: pre
pre:
	-mkdir -p /root/piccolo_yaml
	-cp -r examples/resources/* /root/piccolo_yaml/
	-mkdir -p /etc/containers/systemd/piccolo/
	-mkdir -p /etc/containers/systemd/piccolo/etcd-data/
	-podman-compose -f examples/nginx/docker-compose.yaml up -d

.PHONY: install
install:
	-cp -r ./src/settings.yaml /etc/containers/systemd/piccolo/
	-cp -r ./containers/piccolo.* /etc/containers/systemd/piccolo/
	systemctl daemon-reload
	systemctl start piccolo

.PHONY: uninstall
uninstall:
	-systemctl stop piccolo
	systemctl daemon-reload

.PHONY: post
post:
	-rm -rf /root/piccolo_yaml
	-rm -rf /etc/containers/systemd/*
	-podman-compose -f examples/nginx/docker-compose.yaml down
	systemctl daemon-reload

.PHONY: tools
tools:
	cargo build --manifest-path=src/tools/Cargo.toml --release