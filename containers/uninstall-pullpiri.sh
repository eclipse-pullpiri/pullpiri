#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf /etc/piccolo/*
rm -rf /run/piccololog

podman pod stop -t 0 piccolo-player
podman pod rm -f --ignore piccolo-player
podman pod stop -t 0 piccolo-server
podman pod rm -f --ignore piccolo-server

sleep 1

"${SCRIPT_DIR}/uninstall-agent.sh"