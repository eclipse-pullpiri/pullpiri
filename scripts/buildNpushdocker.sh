#!/bin/bash
# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

set -e

LOG_FILE="dockerbuild_results.log"

echo "Running Docker Build..." | tee -a $LOG_FILE

#git config --global --add safe.directory /__w/PULLPIRI/PULLPIRI
git_hash=$(git rev-parse --short "$GITHUB_SHA")
git_branch=${GITHUB_REF#refs/heads/}
docker build -t sdv.lge.com/demo/${git_branch}/pullpiri:${git_hash} -f containers/Dockerfile . | tee -a $LOG_FILE
# Is tagging required?
# docker tag sdv.lge.com/demo/${git_branch}/pullpiri:${git_hash} sdv.lge.com/demo/${git_branch}/pullpiri:latest | tee -a $LOG_FILE
docker push sdv.lge.com/demo/${git_branch}/pullpiri:${git_hash} | tee -a $LOG_FILE

if [[ "$FAILED" -gt 0 ]]; then
    echo "::error ::Docker build and push failed! Check logs." | tee -a $LOG_FILE
    exit 1
fi

echo "Docker pushed successfully!" | tee -a $LOG_FILE
