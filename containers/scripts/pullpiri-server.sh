#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

if [ -n "${1:-}" ]; then
	MASTER_IP="$1"
else
	MASTER_IP="$(hostname -I | awk '{print $1}')"
fi

INSTALL_MODE="${INSTALL_MODE:-prod}"

# Set environment variables
ROCKSDB_VERSION="v11.18.0"
ROCKSDB_IMAGE="ghcr.io/mco-piccolo/pullpiri-rocksdb:${ROCKSDB_VERSION}"

# If PULLPIRI_IMAGE is set, it takes precedence over mode-based defaults.
if [[ -n "${PULLPIRI_IMAGE:-}" ]]; then
  CONTAINER_IMAGE="${PULLPIRI_IMAGE}"
elif [[ "${INSTALL_MODE}" == "dev" ]]; then
  CONTAINER_IMAGE="localhost/pullpiri:latest"
else
  VERSION="latest"
  CONTAINER_IMAGE="ghcr.io/eclipse-pullpiri/pullpiri:${VERSION}"
fi
echo "Running server with image: ${CONTAINER_IMAGE}"

# Create a pod with host networking
podman pod create \
  --name pullpiri-server \
  --network host \
  --pid host

# Run rocksdbservice container
podman run -d \
  --pod pullpiri-server \
  --name pullpiri-rocksdbservice \
  --user 0:0 \
  -e RUST_LOG="info" \
  -v /etc/pullpiri/pullpiri_shared_rocksdb:/data:Z \
  ${ROCKSDB_IMAGE} \
  rocksdbservice --path /data --addr 0.0.0.0 --port 47007

# Run apiserver container
# Build apiserver command with optional node_configurations.yaml mount
APISERVER_MOUNTS="-v /etc/pullpiri/settings.yaml:/etc/pullpiri/settings.yaml:Z -v /run/pullpirilog/:/run/pullpirilog/"
if [ -f /etc/pullpiri/node_configurations.yaml ]; then
	APISERVER_MOUNTS="${APISERVER_MOUNTS} -v /etc/pullpiri/node_configurations.yaml:/etc/pullpiri/node_configurations.yaml:Z"
fi

podman run -d \
  --pod pullpiri-server \
  --name pullpiri-apiserver \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  ${APISERVER_MOUNTS} \
  ${CONTAINER_IMAGE} \
  /pullpiri/apiserver

# Run policymanager container
podman run -d \
  --pod pullpiri-server \
  --name pullpiri-policymanager \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/pullpiri/settings.yaml:/etc/pullpiri/settings.yaml:Z \
  -v /run/pullpirilog/:/run/pullpirilog/ \
  ${CONTAINER_IMAGE} \
  /pullpiri/policymanager

# Run resourcemanager container (Dynamic Resource Scaling, #514 / #526)
podman run -d \
  --pod pullpiri-server \
  --name pullpiri-resourcemanager \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/pullpiri/settings.yaml:/etc/pullpiri/settings.yaml:Z \
  -v /run/pullpirilog/:/run/pullpirilog/ \
  ${CONTAINER_IMAGE} \
  /pullpiri/resourcemanager

# Run monitoringserver container
podman run -d \
  --pod pullpiri-server \
  --name pullpiri-monitoringserver \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/pullpiri/settings.yaml:/etc/pullpiri/settings.yaml:Z \
  -v /run/pullpirilog/:/run/pullpirilog/ \
  ${CONTAINER_IMAGE} \
  /pullpiri/monitoringserver

# Run logservice container
podman run -d \
  --pod pullpiri-server \
  --name pullpiri-logservice \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/pullpiri/settings.yaml:/etc/pullpiri/settings.yaml:Z \
  -v /run/pullpirilog/:/run/pullpirilog/ \
  ${CONTAINER_IMAGE} \
  /pullpiri/logservice

# Run settingsservice container
podman run -d \
  --pod pullpiri-server \
  --name pullpiri-settingsservice \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/pullpiri/settings.yaml:/etc/pullpiri/settings.yaml:Z \
  -v /run/pullpirilog/:/run/pullpirilog/ \
  ${CONTAINER_IMAGE} \
  /pullpiri/settingsservice --bind-address=${MASTER_IP} --bind-port=8080 --log-level=debug