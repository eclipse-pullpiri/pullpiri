# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    containernetworking-plugins \
    curl \
    fuse-overlayfs \
    gcc \
    git \
    jq \
    libdbus-1-dev \
    libssl-dev \
    make \
    nodejs \
    npm \
    pkg-config \
    podman \
    protobuf-compiler \
    slirp4netns \
    uidmap \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
