#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# SET Piccolo Master node IP address - Read carefully below paragraph
if [ -n "${1:-}" ]; then
	MASTER_IP="$1"
else
	MASTER_IP="$(hostname -I | awk '{print $1}')"
fi
# If you want to hardcode the IPs for testing, you can
# uncomment the lines below and comment out the argument parsing above
# MASTER_IP="127.0.0.1"  # First argument - Piccolo master IP address

# Make rocksdb folder
mkdir -p /etc/piccolo/pullpiri_shared_rocksdb
chown 1001:1001 /etc/piccolo/pullpiri_shared_rocksdb

# Make /etc/piccolo folder
mkdir -p /etc/piccolo

# Make logd socket folder
mkdir -p /run/piccololog

# Create settings.yaml file in /etc/piccolo/
echo "Creating settings.yaml file..."
cat > /etc/piccolo/settings.yaml << EOF
host:
  name: HPC
  ip: ${MASTER_IP}
  type: vehicle
  role: master
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
EOF

"${SCRIPT_DIR}/scripts/piccolo-server.sh" ${MASTER_IP}
"${SCRIPT_DIR}/scripts/piccolo-player.sh" ${MASTER_IP}

sleep 1

"${SCRIPT_DIR}/install-agent.sh" ${MASTER_IP} ${MASTER_IP}