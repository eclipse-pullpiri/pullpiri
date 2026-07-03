#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

SYSTEMD_FILE="/etc/systemd/system/nodeagent.service"
YAML_FILE="/etc/pullpiri/nodeagent.yaml"

sudo systemctl stop nodeagent.service
sudo systemctl daemon-reload

if [ -f "$SYSTEMD_FILE" ]; then
	rm -f "$SYSTEMD_FILE"
fi
sudo systemctl daemon-reload

if [ -f "$YAML_FILE" ]; then
	rm -f "$YAML_FILE"
fi