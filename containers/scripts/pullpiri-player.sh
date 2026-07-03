#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

if [ -n "${1:-}" ]; then
	MASTER_IP="$1"
else
	MASTER_IP="$(hostname -I | awk '{print $1}')"
fi

# Set environment variables
VERSION="latest"
CONTAINER_IMAGE="ghcr.io/eclipse-pullpiri/pullpiri:${VERSION}"
# If you want to use a locally built image, uncomment the line below and comment out the line above
# CONTAINER_IMAGE="localhost/pullpiri:latest"
echo "Running player with image: ${CONTAINER_IMAGE}"

# Create a pod with host networking
podman pod create \
  --name piccolo-player \
  --network host \
  --pid host

# Run filtergateway container
podman run -d \
  --pod piccolo-player \
  --name piccolo-filtergateway \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/filtergateway

# Run actioncontroller container
podman run -d \
  --pod piccolo-player \
  --name piccolo-actioncontroller \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/actioncontroller

# Run statemanager container
podman run -d \
  --pod piccolo-player \
  --name piccolo-statemanager \
  -e ROCKSDB_SERVICE_URL="http://${MASTER_IP}:47007" \
  -v /etc/piccolo/settings.yaml:/etc/piccolo/settings.yaml:Z \
  -v /run/piccololog/:/run/piccololog/ \
  ${CONTAINER_IMAGE} \
  /piccolo/statemanager