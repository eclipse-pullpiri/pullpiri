#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

rustup component add clippy rustfmt

if ! command -v cargo-deny >/dev/null 2>&1; then
  cargo install cargo-deny
fi

if ! command -v cargo2junit >/dev/null 2>&1; then
  cargo install cargo2junit
fi

echo "=== Codespace toolchain summary ==="
cat /etc/os-release | grep -E '^(NAME|VERSION)='
rustc --version
cargo --version
podman --version
