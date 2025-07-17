# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

############################
# === Build Stage ===
############################
FROM rust:1.85.0-slim AS builder

ARG COMPONENT
ARG TARGETARCH

ENV COMPONENT=${COMPONENT}
ENV TARGETARCH=${TARGETARCH}

WORKDIR /pullpiri

# Copy shared and component-specific source code into container
COPY ./src/common /pullpiri/common
COPY ./src/${COMPONENT} /pullpiri/${COMPONENT}

# Install necessary libraries and build component
RUN apt update -y && \
    apt install -y \
      libdbus-1-dev \
      pkg-config \
      protobuf-compiler \
      libssl-dev && \
    cd /pullpiri/${COMPONENT} && \
    cargo build --release

# Prepare glibc shared libraries for static alpine runtime
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
    else \
        echo "Unsupported architecture: $TARGETARCH" && exit 1; \
    fi && \
    for lib in \
        libc.so.6 libcap.so.2 libdbus-1.so.3 libgcc_s.so.1 \
        libgcrypt.so.20 libgpg-error.so.0 liblzma.so.5 \
        liblz4.so.1 libm.so.6 libsystemd.so.0 libzstd.so.1; do \
        cp -v /lib/${ITEMARCH}-linux-gnu/$lib* /dummy/${ITEMARCH}-linux-gnu/ || true; \
    done

############################
# === Runtime Stage ===
############################
FROM alpine:3.21.3

ARG COMPONENT
ARG TARGETARCH

ENV COMPONENT=${COMPONENT}
ENV TARGETARCH=${TARGETARCH}

WORKDIR /pullpiri

# Copy runtime shared libraries
COPY --from=builder /dummy /lib

# Ensure dynamic linker is in correct place for glibc-based binaries
RUN if [ "$TARGETARCH" = "amd64" ]; then \
        mkdir -p /lib64 && \
        cp -v /lib/lib64/ld-linux-x86-64.so.2 /lib64/ || true; \
    elif [ "$TARGETARCH" = "arm64" ]; then \
        echo "Using aarch64 runtime"; \
    else \
        echo "Unsupported architecture: $TARGETARCH" && exit 1; \
    fi

# Copy built Rust binary
COPY --from=builder /pullpiri/${COMPONENT}/target/release/ /pullpiri/

# Copy shared runtime config
COPY ./src/settings.yaml .

# Entry point
CMD ["sh"]
