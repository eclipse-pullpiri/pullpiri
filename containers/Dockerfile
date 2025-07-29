# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

#################################
# === Build Stage ===
#################################
FROM rust:1.85.0-slim AS builder

ARG TARGETARCH

ENV TARGETARCH=${TARGETARCH}
WORKDIR /piccolo

# Copy shared source and both components
COPY ./src/common/ /piccolo/common/
COPY ./src/server/ /piccolo/server/
COPY ./src/agent/ /piccolo/agent/
COPY ./src/player/ /piccolo/player

# Install build dependencies
RUN apt update -y && \
    apt install -y libdbus-1-dev pkg-config protobuf-compiler libssl-dev

# Build server binaries
WORKDIR /piccolo/server
RUN cargo build --release

# Build agent binaries
WORKDIR /piccolo/agent
RUN cargo build --release

# Build player binaries
WORKDIR /piccolo/player
RUN cargo build --release

# Prepare glibc shared libraries
WORKDIR /dummy
RUN if [ "$TARGETARCH" = "amd64" ]; then \
        ITEMARCH="x86_64"; \
        mkdir -p /dummy/lib64 /dummy/${ITEMARCH}-linux-gnu/ && \
        cp -v /lib64/ld-linux-x86-64.so.2 /dummy/lib64/ || true && \
        cp -v /lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 /dummy/${ITEMARCH}-linux-gnu/ || true; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
        ITEMARCH="aarch64"; \
        mkdir -p /dummy/${ITEMARCH}-linux-gnu/ && \
        cp -v /lib/aarch64-linux-gnu/ld-linux-aarch64.so.1 /dummy/${ITEMARCH}-linux-gnu/ || true && \
        cp -v /lib/ld-linux-aarch64.so.1 /dummy/ || true; \
    fi && \
    for lib in \
        libc.so.6 libcap.so.2 libdbus-1.so.3 libgcc_s.so.1 \
        libgcrypt.so.20 libgpg-error.so.0 liblzma.so.5 \
        liblz4.so.1 libm.so.6 libsystemd.so.0 libzstd.so.1; do \
        cp -v /lib/${ITEMARCH}-linux-gnu/$lib* /dummy/${ITEMARCH}-linux-gnu/ || true; \
    done

#################################
# === Runtime Stage ===
#################################
FROM alpine:3.21.3

ARG TARGETARCH
ENV TARGETARCH=${TARGETARCH}

WORKDIR /piccolo

# Copy shared libraries
COPY --from=builder /dummy /lib

# Setup dynamic linker for glibc
RUN if [ "$TARGETARCH" = "amd64" ]; then \
        mkdir -p /lib64 && \
        cp -v /lib/lib64/ld-linux-x86-64.so.2 /lib64/ || true; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
        echo "Using aarch64 runtime"; \
    else \
        echo "Unsupported architecture: $TARGETARCH" && exit 1; \
    fi

# Copy built binaries
COPY --from=builder /piccolo/server/target/release/apiserver /piccolo/
COPY --from=builder /piccolo/server/target/release/monitoringserver /piccolo/
COPY --from=builder /piccolo/server/target/release/policymanager /piccolo/
COPY --from=builder /piccolo/agent/target/release/nodeagent /piccolo/
COPY --from=builder /piccolo/player/target/release/actioncontroller /piccolo/
COPY --from=builder /piccolo/player/target/release/filtergateway /piccolo/
COPY --from=builder /piccolo/player/target/release/statemanager /piccolo/

# Copy runtime settings
COPY ./src/settings.yaml .

CMD [ "sh" ]
