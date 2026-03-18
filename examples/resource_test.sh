#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0


BODY=$(< ./resources/network-volume-example.yaml)
# BODY=$(< ./resources/helloworld_resource.yaml)
URL="http://192.168.50.112:47099/api/artifact"

curl -X POST "${URL}" \
--header 'Content-Type: text/plain' \
--data "${BODY}"
