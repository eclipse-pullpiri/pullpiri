#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Get arguments - Read carefully below paragraph
MASTER_IP="${1:-}"  # First argument - Pullpiri master IP address
NODE_IP="${2:-}"    # Second argument - Node IP address
# If you want to hardcode the IPs for testing, you can
# uncomment the lines below and comment out the argument parsing above
# MASTER_IP="127.0.0.1"  # First argument - Pullpiri master IP address
# NODE_IP="127.0.0.1"    # Second argument - Node IP address

# Check if both IPs are provided
if [[ -z "${MASTER_IP}" ]] || [[ -z "${NODE_IP}" ]]; then
	echo "ERROR: Both MASTER_IP and NODE_IP arguments are required." >&2
	echo "Usage: $0 MASTER_IP NODE_IP" >&2
	echo "  MASTER_IP: Pullpiri master IP address" >&2
	echo "  NODE_IP: Node IP address" >&2
	exit 1
fi

# Validate MASTER_IP
if [[ "$MASTER_IP" =~ ^(([1-9]?[0-9]|1[0-9][0-9]|2([0-4][0-9]|5[0-5]))\.){3}([1-9]?[0-9]|1[0-9][0-9]|2([0-4][0-9]|5[0-5]))$ ]]; then
	echo "MASTER_IP: '${MASTER_IP}'"
else
	echo "ERROR: Invalid IPv4 address for MASTER_IP - '${MASTER_IP}'"
	exit 1
fi

# Validate NODE_IP
if [[ "$NODE_IP" =~ ^(([1-9]?[0-9]|1[0-9][0-9]|2([0-4][0-9]|5[0-5]))\.){3}([1-9]?[0-9]|1[0-9][0-9]|2([0-4][0-9]|5[0-5]))$ ]]; then
	echo "NODE_IP: '${NODE_IP}'"
else
	echo "ERROR: Invalid IPv4 address for NODE_IP - '${NODE_IP}'"
	exit 1
fi

NODE_NAME=$(hostname)  # Always use system hostname
NODE_ROLE="nodeagent"  # Default node role (master, nodeagent, bluechi)
NODE_TYPE="vehicle"  # Default node type (vehicle, cloud)

# Set architecture
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
	SUFFIX="amd64"
	BUILD_TARGET="x86_64-unknown-linux-musl"
elif [ "$ARCH" = "aarch64" ]; then
	SUFFIX="arm64"
	BUILD_TARGET="aarch64-unknown-linux-musl"
else
	echo "Error: Unsupported architecture '${ARCH}'."
	exit 1
fi

INSTALL_MODE="${INSTALL_MODE:-prod}"

# Make directory and binary
AGENT_BINARY_PATH="/opt/pullpiri/nodeagent"
sudo mkdir -p "$(dirname "${AGENT_BINARY_PATH}")"
BINARY_URL="https://github.com/eclipse-pullpiri/pullpiri/releases/latest/download/nodeagent-linux-${SUFFIX}"

if [[ "${INSTALL_MODE}" == "dev" ]]; then
	BUILD_BINARY_PATH_DEFAULT="${SCRIPT_DIR}/../src/agent/nodeagent/target/${BUILD_TARGET}/release/nodeagent"
	BUILD_BINARY_PATH="${BUILD_BINARY_PATH:-${BUILD_BINARY_PATH_DEFAULT}}"

	if [[ -f "${BUILD_BINARY_PATH}" ]]; then
		sudo cp -f "${BUILD_BINARY_PATH}" "${AGENT_BINARY_PATH}"
		echo "Used locally built binary from ${BUILD_BINARY_PATH}"
	elif [[ ! -f "${AGENT_BINARY_PATH}" ]]; then
		echo "Downloading latest release binary from ${BINARY_URL}"
		curl -fsSL -o nodeagent "${BINARY_URL}" || {
			echo "Error: Failed to download binary from ${BINARY_URL}"
			exit 1
		}
		sudo cp -f nodeagent "${AGENT_BINARY_PATH}"
		rm -f nodeagent
	else
		echo "Using existing installed binary at ${AGENT_BINARY_PATH}"
	fi
else
	CHECKSUM_URL="https://github.com/eclipse-pullpiri/pullpiri/releases/latest/download/SHA256SUMS-nodeagent"
	TMP_CHECKSUMS="$(mktemp)"

	cleanup() {
		rm -f "${TMP_CHECKSUMS}" ./nodeagent
	}
	trap cleanup EXIT

	echo "Fetching latest checksum list from ${CHECKSUM_URL}..."
	curl -fsSL -o "${TMP_CHECKSUMS}" "${CHECKSUM_URL}" || {
		echo "Error: Failed to download checksum list from ${CHECKSUM_URL}"
		exit 1
	}

	EXPECTED_HASH=$(awk -v suffix="${SUFFIX}" '$2 ~ ("nodeagent-linux-" suffix "$") {print $1; exit}' "${TMP_CHECKSUMS}")
	if [[ -z "${EXPECTED_HASH}" ]]; then
		echo "Error: Could not find checksum entry for nodeagent-linux-${SUFFIX}"
		exit 1
	fi

	NEEDS_DOWNLOAD=1
	if sudo test -f "${AGENT_BINARY_PATH}"; then
		CURRENT_HASH=$(sudo sha256sum "${AGENT_BINARY_PATH}" | awk '{print $1}')
		if [[ "${CURRENT_HASH}" == "${EXPECTED_HASH}" ]]; then
			echo "Installed nodeagent matches latest release checksum."
			NEEDS_DOWNLOAD=0
		else
			echo "Installed nodeagent is outdated. Replacing with latest release binary."
			sudo rm -f "${AGENT_BINARY_PATH}"
		fi
	else
		echo "No installed nodeagent found. Downloading latest release binary."
	fi

	if [[ "${NEEDS_DOWNLOAD}" -eq 1 ]]; then
		echo "Downloading binary from ${BINARY_URL}..."
		curl -fsSL -o nodeagent "${BINARY_URL}" || {
			echo "Error: Failed to download binary from ${BINARY_URL}"
			exit 1
		}
		DOWNLOADED_HASH=$(sha256sum nodeagent | awk '{print $1}')
		if [[ "${DOWNLOADED_HASH}" != "${EXPECTED_HASH}" ]]; then
			echo "Error: Downloaded binary checksum mismatch for nodeagent-linux-${SUFFIX}"
			exit 1
		fi
		sudo cp -f nodeagent "${AGENT_BINARY_PATH}"
	fi
fi

sudo chmod +x "${AGENT_BINARY_PATH}"
echo "Binary installed to ${AGENT_BINARY_PATH}"

# Create configuration file
echo "Creating configuration file..."
sudo mkdir -p /etc/pullpiri
cat > /etc/pullpiri/nodeagent.yaml << EOF
nodeagent:
  node_name: "${NODE_NAME}"
  node_type: "${NODE_TYPE}"
  node_role: "${NODE_ROLE}"
  master_ip: "${MASTER_IP}"
  node_ip: "${NODE_IP}"
  grpc_port: 47004
  log_level: "info"
  metrics:
    collection_interval: 5
    batch_size: 50
  system:
    hostname: "${NODE_NAME}"
    platform: "$(uname -s)"
    architecture: "${ARCH}"
EOF

# Create systemd service file
echo "Creating systemd service file..."
cat > /etc/systemd/system/nodeagent.service << EOF
[Unit]
Description=Pullpiri NodeAgent Service
After=network-online.target
Wants=podman.socket

[Service]
Type=simple
ExecStart=/opt/pullpiri/nodeagent --config /etc/pullpiri/nodeagent.yaml
Restart=on-failure
RestartSec=10
Environment=RUST_LOG=info
Environment=MASTER_NODE_IP=${MASTER_IP}
Environment=NODE_IP=${NODE_IP}
Environment=GRPC_PORT=47004

# Security hardening settings
ProtectSystem=full
ProtectHome=true
NoNewPrivileges=true

ReadWritePaths=/etc/pullpiri
ReadWritePaths=/etc/containers/systemd

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd and enable service
echo "Enabling NodeAgent service..."
sudo systemctl daemon-reload
sudo systemctl enable nodeagent.service || {
	echo "Error: Failed to enable NodeAgent service."
	exit 1
}

# Start service
echo "Starting NodeAgent service..."
sudo systemctl start nodeagent.service || {
	echo "Warning: Failed to start NodeAgent service."
	exit 1
}
