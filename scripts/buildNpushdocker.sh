#!/bin/bash

set -e

LOG_FILE="dockerbuild_results.log"

echo "Running Docker Build..." | tee -a $LOG_FILE

#git config --global --add safe.directory /__w/PICCOLO/PICCOLO
git_hash=$(git rev-parse --short "$GITHUB_SHA")
git_branch=${GITHUB_REF#refs/heads/}
docker build -t sdv.lge.com/demo/${git_branch}/piccolo:${git_hash} -f containers/Dockerfile . | tee -a $LOG_FILE
# Is tagging required?
# docker tag sdv.lge.com/demo/${git_branch}/piccolo:${git_hash} sdv.lge.com/demo/${git_branch}/piccolo:latest | tee -a $LOG_FILE
docker push sdv.lge.com/demo/${git_branch}/piccolo:${git_hash} | tee -a $LOG_FILE

if [[ "$FAILED" -gt 0 ]]; then
    echo "::error ::Docker build and push failed! Check logs." | tee -a $LOG_FILE
    exit 1
fi

echo "Docker pushed successfully!" | tee -a $LOG_FILE
