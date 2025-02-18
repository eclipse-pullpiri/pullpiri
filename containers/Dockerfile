# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

FROM rust:1.78.0-slim AS builder
WORKDIR /piccolo

COPY ./src/Cargo.lock \
     ./src/Cargo.toml /piccolo/
COPY ./src/api /piccolo/api
COPY ./src/apiserver/ /piccolo/apiserver
COPY ./src/common/ /piccolo/common
COPY ./src/gateway/ /piccolo/gateway
COPY ./src/importer/ /piccolo/importer
COPY ./src/statemanager/ /piccolo/statemanager
COPY ./src/vendor/ /piccolo/vendor

RUN apt update -y && \
    apt upgrade -y && \
    apt install -y libdbus-1-dev pkg-config protobuf-compiler libssl-dev && \
    cargo build --release


FROM alpine:3.20.3
WORKDIR /piccolo

COPY ./src/settings.yaml .

COPY --from=builder \
    /piccolo/target/release/apiserver \
    /piccolo/target/release/gateway \
    /piccolo/target/release/statemanager ./

COPY --from=builder /lib64 /lib64
COPY --from=builder \
    /lib/x86_64-linux-gnu/ld-linux-x86-64.so.2 \
    /lib/x86_64-linux-gnu/libgcc_s.so.1 \
    /lib/x86_64-linux-gnu/libdbus-1.so.3 \
    /lib/x86_64-linux-gnu/libdbus-1.so.3.32.4 \
    /lib/x86_64-linux-gnu/libm.so.6 \
    /lib/x86_64-linux-gnu/libc.so.6 \
    /lib/x86_64-linux-gnu/libsystemd.so.0 \
    /lib/x86_64-linux-gnu/libsystemd.so.0.35.0 \
    /lib/x86_64-linux-gnu/libcap.so.2 \
    /lib/x86_64-linux-gnu/libcap.so.2.66 \
    /lib/x86_64-linux-gnu/libgcrypt.so.20 \
    /lib/x86_64-linux-gnu/libgcrypt.so.20.4.1 \
    /lib/x86_64-linux-gnu/liblzma.so.5 \
    /lib/x86_64-linux-gnu/liblzma.so.5.4.1 \
    /lib/x86_64-linux-gnu/libzstd.so.1 \
    /lib/x86_64-linux-gnu/libzstd.so.1.5.4 \
    /lib/x86_64-linux-gnu/liblz4.so.1 \
    /lib/x86_64-linux-gnu/liblz4.so.1.9.4 \
    /lib/x86_64-linux-gnu/libgpg-error.so.0 \
    /lib/x86_64-linux-gnu/libgpg-error.so.0.33.1 /lib/x86_64-linux-gnu/

CMD sh
