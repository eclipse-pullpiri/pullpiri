# SPDX-License-Identifier: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

############################
# === Build Stage ===
############################
# Use official slim Rust image for minimal build footprint
FROM rust:1.78.0-slim AS builder

# Accept a component name via build argument
ARG COMPONENT
ENV COMPONENT=${COMPONENT}

# Set working directory inside container
WORKDIR /pullpiri

# Copy shared and component-specific source code into container
COPY ./src/common /pullpiri/common
COPY ./src/${COMPONENT} /pullpiri/${COMPONENT}

# Install necessary system libraries and build the component
RUN apt update -y && \
    apt install -y \
      libdbus-1-dev \
      pkg-config \
      protobuf-compiler \
      libssl-dev && \
    cd /pullpiri/${COMPONENT} && \
    cargo build --release

############################
# === Runtime Stage ===
############################
# Use lightweight Alpine image for minimal runtime footprint
FROM alpine:3.20.3

# Set component environment (reused for directory references)
ARG COMPONENT
ENV COMPONENT=${COMPONENT}

# Set runtime working directory
WORKDIR /pullpiri

# Copy shared runtime config (if needed by all components)
COPY ./src/settings.yaml .

# Copy only the built binaries from the builder stage to reduce image size
COPY --from=builder /pullpiri/${COMPONENT}/target/release /pullpiri/

# Default shell to interactive mode (can be overridden by CMD/ENTRYPOINT)
CMD ["sh"]