#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

rm -rf /etc/pullpiri/*
rm -rf /run/pullpirilog

podman pod stop -t 0 pullpiri-player
podman pod rm -f --ignore pullpiri-player
podman pod stop -t 0 pullpiri-server
podman pod rm -f --ignore pullpiri-server

sleep 1

"${SCRIPT_DIR}/uninstall-agent.sh"