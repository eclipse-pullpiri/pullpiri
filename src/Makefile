# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

.PHONY: default build
build:
	cargo build --manifest-path=observer/Cargo.toml
	cargo build --manifest-path=player/Cargo.toml
	cargo build --manifest-path=server/Cargo.toml

.PHONY: clean
clean:
	cargo clean --manifest-path=common/Cargo.toml
	cargo clean --manifest-path=observer/Cargo.toml
	cargo clean --manifest-path=player/Cargo.toml
	cargo clean --manifest-path=server/Cargo.toml
	cargo clean --manifest-path=tools/Cargo.toml

.PHONY: tool
tool:
	cargo build --manifest-path=tools/Cargo.toml

.PHONY: fmt
fmt:
	cd common && cargo fmt
	cd observer && cargo fmt
	cd player && cargo fmt
	cd server && cargo fmt
	cd tools && cargo fmt

.PHONY: clippy
clippy:
	cd common && cargo clippy
	cd observer && cargo clippy
	cd player && cargo clippy
	cd server && cargo clippy
	cd tools && cargo clippy