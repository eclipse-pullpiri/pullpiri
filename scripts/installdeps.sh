#!/bin/bash
set -euo pipefail

# Enable JSON test output even on stable Rust
export RUSTC_BOOTSTRAP=1

echo "🛠️ Updating package lists..."
sudo apt-get update -y

echo "📦 Installing common development packages..."
common_packages=(
  libdbus-1-dev
  git
  make
  gcc
  protobuf-compiler
  build-essential
  pkg-config
  curl
  libssl-dev
  nodejs
  jq
  npm
)
DEBIAN_FRONTEND=noninteractive sudo apt-get install -y "${common_packages[@]}"
echo "✅ Base packages installed successfully"

# ----------------------------------------
# 🦀 Install rustup, Clippy, Rustfmt, and cargo-deny
# ----------------------------------------
echo "🦀 Installing Rust toolchain..."
if ! command -v rustup &>/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

# Ensure PATH is correctly set
export PATH="$HOME/.cargo/bin:$PATH"

# Install required Rust components
echo "🔧 Installing Clippy and Rustfmt..."
rustup component add clippy
rustup component add rustfmt

# Install cargo-deny
if ! command -v cargo-deny &>/dev/null; then
  echo "🔐 Installing cargo-deny..."
  cargo install cargo-deny
fi

# Install cargo2junit
if ! command -v cargo2junit &>/dev/null; then
  echo "🔐 Installing cargo2junit..."
  cargo install cargo2junit
fi

# Show installed versions
echo "📌 Installed Rust toolchain versions:"
cargo --version
cargo clippy --version
cargo fmt --version
cargo deny --version
echo "✅ Rust toolchain installed successfully."

# ----------------------------------------
# 📦 Install etcd & etcdctl
# ----------------------------------------

echo "🔧 Installing etcd and etcdctl..."
ETCD_VER="v3.5.11"
ETCD_PKG="etcd-${ETCD_VER}-linux-amd64"
ETCD_URL="https://github.com/etcd-io/etcd/releases/download/${ETCD_VER}/${ETCD_PKG}.tar.gz"

curl -L "$ETCD_URL" -o etcd.tar.gz
tar xzvf etcd.tar.gz
sudo cp "${ETCD_PKG}/etcd" /usr/local/bin/
sudo cp "${ETCD_PKG}/etcdctl" /usr/local/bin/
sudo chmod +x /usr/local/bin/etcd /usr/local/bin/etcdctl
rm -rf etcd.tar.gz "${ETCD_PKG}"

echo "✅ etcd and etcdctl installed."

# ----------------------------------------
# 🚀 Start etcd in background
# ----------------------------------------

echo "🚀 Starting etcd..."
nohup etcd \
  --name s1 \
  --data-dir /tmp/etcd-data \
  --initial-advertise-peer-urls http://localhost:2380 \
  --listen-peer-urls http://127.0.0.1:2380 \
  --advertise-client-urls http://localhost:2379 \
  --listen-client-urls http://127.0.0.1:2379 > etcd.log 2>&1 &

ETCD_PID=$!
echo "🆔 etcd started with PID $ETCD_PID"

# ----------------------------------------
# ⏳ Wait for etcd to become healthy
# ----------------------------------------

echo "⏳ Waiting for etcd to be healthy..."
for i in {1..10}; do
  if etcdctl --endpoints=http://localhost:2379 endpoint health &>/dev/null; then
    echo "✅ etcd is healthy and ready."
    break
  else
    echo "⌛ Waiting... ($i)"
    sleep 2
  fi
done

# Final check before continuing
if ! etcdctl --endpoints=http://localhost:2379 endpoint health &>/dev/null; then
  echo "::error ::etcd did not become healthy in time!"
  cat etcd.log
  exit 1
fi

# ----------------------------------------
# 🐳 Install Docker and Docker Compose
# ----------------------------------------

echo "🐳 Installing Docker CLI and Docker Compose..."

# Install Docker dependencies
sudo apt-get update -y
sudo apt-get install -y \
    ca-certificates \
    curl \
    gnupg \
    lsb-release

# Add Docker’s official GPG key
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg

# Set up Docker stable repository for Ubuntu Jammy
echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
  https://download.docker.com/linux/ubuntu jammy stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null

# Update and install Docker packages
sudo apt-get update -y
sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

# Verify installation
docker --version
docker compose version

echo "✅ Docker and Docker Compose installed."

echo "🎉 All dependencies installed and etcd is running!"
