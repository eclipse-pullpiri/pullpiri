#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# SET Pullpiri Master node IP address - Read carefully below paragraph
if [ -n "${1:-}" ]; then
	MASTER_IP="$1"
else
	MASTER_IP="$(hostname -I | awk '{print $1}')"
fi
HOST_NAME="$(hostname)"
# If you want to hardcode the IPs for testing, you can
# uncomment the lines below and comment out the argument parsing above
# MASTER_IP="127.0.0.1"  # First argument - Pullpiri master IP address

# Make rocksdb folder
mkdir -p /etc/pullpiri/pullpiri_shared_rocksdb
chown 1001:1001 /etc/pullpiri/pullpiri_shared_rocksdb

# Make /etc/pullpiri folder
mkdir -p /etc/pullpiri

# Make logd socket folder
mkdir -p /run/pullpirilog

# Create settings.yaml file in /etc/pullpiri/
echo "Creating settings.yaml file..."
cat > /etc/pullpiri/settings.yaml << EOF
host:
  name: ${HOST_NAME}
  ip: ${MASTER_IP}
  type: vehicle
  role: master
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
EOF

"${SCRIPT_DIR}/scripts/pullpiri-server.sh" ${MASTER_IP}
"${SCRIPT_DIR}/scripts/pullpiri-player.sh" ${MASTER_IP}

sleep 1

"${SCRIPT_DIR}/install-agent.sh" ${MASTER_IP} ${MASTER_IP}