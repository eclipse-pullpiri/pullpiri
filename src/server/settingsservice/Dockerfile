# SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
# SPDX-License-Identifier: Apache-2.0

FROM rust:1.70 as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libdbus-1-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the entire project for build context
COPY . .

# Build the settings service
RUN cd src/server/settingsservice && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libdbus-1-3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false settingsservice

# Copy binary from builder
COPY --from=builder /app/src/server/target/release/settingsservice /usr/local/bin/
COPY --from=builder /app/src/server/target/release/settings-cli /usr/local/bin/

# Create configuration directory
RUN mkdir -p /etc/piccolo && chown settingsservice:settingsservice /etc/piccolo

# Switch to non-root user
USER settingsservice

# Expose the default port
EXPOSE 8080

# Set default command
CMD ["settingsservice", "--bind-address", "0.0.0.0", "--bind-port", "8080"]