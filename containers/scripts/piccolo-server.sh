#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

if [ -n "${1:-}" ]; then
	MASTER_IP="$1"
else
	MASTER_IP="$(hostname -I | awk '{print $1}')"
fi

# Set environment variables
ROCKSDB_VERSION="v11.18.0"
ROCKSDB_IMAGE="ghcr.io/mco-piccolo/pullpiri-rocksdb:${ROCKSDB_VERSION}"

VERSION="latest"
CONTAINER_IMAGE="ghcr.io/eclipse-pullpiri/pullpiri:${VERSION}"
# If you want to use a locally built image, uncomment the line below and comment out the line above
# CONTAINER_IMAGE="localhost/pullpiri:latest"
echo "Running server with image: ${CONTAINER_IMAGE}"

# Create a pod with host networking
podman pod create \
  --name piccolo-server \
  --network host \
  --pid host

# Run rocksdbservice container
podman run -d \
  --pod piccolo-server \
  --name piccolo-rocksdbservice \
  --user 0:0 \
  -e RUST_LOG="info" \
  -v /etc/piccolo/pullpiri_shared_rocksdb:/data:Z \
  ${ROCKSDB_IMAGE} \
  rocksdbservice --path /data --addr 0.0.0.0 --port 47007

# Run apiserver container
podman run -d \
  --pod piccolo-server \
  --name piccolo-apiserver \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/apiserver

# Run policymanager container
podman run -d \
  --pod piccolo-server \
  --name piccolo-policymanager \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/policymanager

# Run monitoringserver container
podman run -d \
  --pod piccolo-server \
  --name piccolo-monitoringserver \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/monitoringserver

# Run logservice container
podman run -d \
  --pod piccolo-server \
  --name piccolo-logservice \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/logservice

# Run settingsservice container
podman run -d \
  --pod piccolo-server \
  --name piccolo-settingsservice \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/settingsservice --bind-address=${MASTER_IP} --bind-port=8080 --log-level=debug

# Run resourcemanager container
podman run -d \
  --pod piccolo-server \
  --name piccolo-resourcemanager \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/resourcemanager