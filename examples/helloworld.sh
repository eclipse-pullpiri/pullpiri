#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

BODY=$(< ./resources/hello-world.yaml)
HOST_IP=$(hostname -I | awk '{print $1}')

curl -X POST "http://${HOST_IP}:47099/api/artifact" \
--header 'Content-Type: text/plain' \
--data "${BODY}"
