<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Build from Source

This guide explains how to build Pullpiri from source code. It covers creating the Docker/Podman container image, compiling all Rust binaries, and installing the built images on your target system.

> **Back to:** [Getting Started](./getting-started.md)

---

## Prerequisites

### System Requirements

| Software | Version | Purpose |
|----------|---------|---------|
| [Podman](https://podman.io/) | ≥ 4.0.0 | Container build and runtime |
| Git | any | Source code checkout |
| make | any | Build orchestration |
| Disk space | ≥ 20 GB | Build artifacts and images |

> **Note:** The Rust toolchain, protobuf compiler, and all other build dependencies are automatically installed inside the Docker build stage — you do **not** need to install them on your host machine.

### Install Podman

**Ubuntu 22.04 / 24.04:**

```bash
sudo apt update
sudo apt install -y podman git make
```

**CentOS Stream 9 / RHEL:**

```bash
sudo dnf install -y podman git make gcc
```

### Open Required Ports

Pullpiri uses the following TCP ports. Before deploying, configure your firewall to allow them:

| Port | Service |
|------|---------|
| 8080 | Settings Service (REST) |
| 47001 | Action Controller (gRPC) |
| 47002 | Filter Gateway (gRPC) |
| 47003 | Monitoring Server (gRPC) |
| 47004 | NodeAgent (gRPC) |
| 47005 | Policy Manager (gRPC) |
| 47006 | State Manager (gRPC) |
| 47007 | RocksDB Service (gRPC) |
| 47098 | API Server (gRPC) |
| 47099 | API Server (REST) |

**Ubuntu (ufw):**

First check whether ufw is active:

```bash
sudo ufw status
# Status: active   → run the commands below
# Status: inactive → firewall is off, ports are already open; skip this step
```

If ufw is active, open the required ports:

```bash
sudo ufw allow 8080/tcp
sudo ufw allow 47001:47007/tcp
sudo ufw allow 47098:47099/tcp
sudo ufw reload
sudo ufw status numbered
```

> **Note:** If ufw is inactive on your system (default on many Ubuntu installs), all ports are already reachable — no firewall changes are needed.

**CentOS Stream 9 / RHEL (firewalld):**

First check whether firewalld is running:

```bash
sudo systemctl is-active firewalld
# active   → run the commands below
# inactive → firewall is off, ports are already open; skip this step
```

If firewalld is active, open the required ports:

```bash
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --permanent --add-port=47001-47007/tcp
sudo firewall-cmd --permanent --add-port=47098-47099/tcp
sudo firewall-cmd --reload
sudo firewall-cmd --list-ports
```

---

## Step 1: Clone the Repository

```bash
git clone https://github.com/eclipse-pullpiri/pullpiri.git
cd pullpiri
```

---

## Step 2: Understand the Dockerfile

The project provides a multi-stage `containers/Dockerfile` that handles the entire build process:

```
containers/Dockerfile
```

### Build Stages

#### Stage 1 – Builder (`rust:1.88.0-slim`)

The builder stage installs all build-time dependencies and compiles all Rust binaries in release mode:

- Installs system libraries: `libdbus-1-dev`, `protobuf-compiler`, `libssl-dev`, `clang`, `cmake`, etc.
- Copies source code from `src/`
- Runs `cargo build --release` to produce all server and player binaries
- Copies architecture-specific shared libraries (glibc) for the runtime stage

#### Stage 2 – Runtime (`alpine:3.21.3`)

The runtime stage is a minimal Alpine image containing only the compiled binaries and required shared libraries:

- Copies glibc shared libraries from the builder stage
- Copies compiled binaries: `apiserver`, `monitoringserver`, `policymanager`, `settingsservice`, `logservice`, `actioncontroller`, `filtergateway`, `statemanager`
- Produces a small, production-ready image

### Supported Architectures

| Architecture | Docker Platform |
|-------------|----------------|
| x86_64 | `linux/amd64` |
| aarch64 | `linux/arm64` |

The `TARGETARCH` build argument is automatically set when using `podman buildx` or `docker buildx` for multi-architecture builds.

---

## Step 3: Build the Container Image

### Build the Main Pullpiri Image

```bash
# Build for the current host architecture
make image
# Equivalent to:
# podman build -t localhost/pullpiri:latest -f containers/Dockerfile .
```

> **Caution:** A successful build requires at least **20 GB** of free disk space and takes approximately **10–20 minutes** depending on your hardware and network speed (first build downloads Rust crates).

### Build the RocksDB Service Image

```bash
make rocksdb-image
# Equivalent to:
# podman build -t localhost/pullpiri-rocksdb:latest -f src/server/rocksdbservice/Dockerfile .
```

### Build All Images

```bash
make all-images
```

After the build, verify the images:

```bash
podman images | grep pullpiri
# localhost/pullpiri           latest   ...
# localhost/pullpiri-rocksdb   latest   ...
```

---

## Step 4: Build the NodeAgent Binary (Optional)

The `nodeagent` is deployed on remote vehicle nodes and must be compiled as a static binary using musl libc for maximum portability.

```bash
make nodeagent-bin
```

The binary will be located at:

```
src/agent/nodeagent/target/x86_64-unknown-linux-musl/release/nodeagent
```

> **Note:** Building with musl requires the `x86_64-unknown-linux-musl` target to be installed. The build script handles this automatically via `scripts/installdeps.sh`.

> **Note:** `nodeagent` is excluded from the main workspace (`src/Cargo.toml`) and has its own independent build. Only the musl static binary is suitable for deployment on remote vehicle nodes.

---

## Step 5: Build Rust Binaries Directly (Development)

For local development and testing, you can build the Rust binaries directly without using containers. This requires installing all build dependencies on your host.

### Install Build Dependencies

```bash
bash scripts/installdeps.sh
```

The script installs:
- Rust toolchain (`rustup`, `cargo`, `clippy`, `rustfmt`)
- `protobuf-compiler`
- `libdbus-1-dev`, `libssl-dev`, `pkg-config`
- Docker and Docker Compose
- `cargo-deny`, `cargo2junit`
- Common tools: `git`, `make`, `gcc`, `nodejs`, `jq`, `npm`

> **Important:** `scripts/installdeps.sh` installs **Docker**, not Podman. For container image builds (`make image`), the `Makefile` uses `podman build`, so you must install Podman separately before running `make image`. See [Install Podman](#prerequisites).

> **Note:** Installation takes approximately 8–10 minutes on first run.

### Build All Components

```bash
export PATH="$HOME/.cargo/bin:$PATH"
make build
# Equivalent to:
# cargo build --manifest-path=src/Cargo.toml
```

### Build a Specific Component

```bash
# Server components
cargo build --manifest-path=src/server/apiserver/Cargo.toml

# Player components
cargo build --manifest-path=src/player/filtergateway/Cargo.toml

# Agent
cargo build --manifest-path=src/agent/nodeagent/Cargo.toml
```

### Build Output

After a successful build, binaries are located in:

```
src/target/debug/
├── apiserver
├── monitoringserver
├── policymanager
├── settingsservice
├── logservice
├── actioncontroller
├── filtergateway
└── statemanager

src/agent/nodeagent/target/debug/
└── nodeagent
```

---

## Step 6: Install Using Built Images

After building the container images locally, use the install script to deploy them:

```bash
# Prepare system directories
sudo mkdir -p /etc/pullpiri
sudo mkdir -p /run/pullpirilog

# Deploy using locally built images
sudo bash containers/install-pullpiri.sh
```

The install script:
1. Creates `/etc/pullpiri/settings.yaml` with auto-detected host name and IP.
2. Starts the `pullpiri-server` Podman pod (RocksDB, APIServer, PolicyManager, MonitoringServer, LogService, SettingsService).
3. Starts the `pullpiri-player` Podman pod (FilterGateway, ActionController, StateManager).
4. Downloads the `nodeagent` binary from GitHub Releases, places it at `/opt/pullpiri/nodeagent`, and registers it as a **systemd service** (`nodeagent.service`).

> **Important:** By default, the install script uses the remote image `ghcr.io/eclipse-pullpiri/pullpiri:latest`. To use your locally built image, edit `containers/scripts/pullpiri-server.sh` and `containers/scripts/pullpiri-player.sh`:
>
> ```bash
> # Change this line:
> CONTAINER_IMAGE="ghcr.io/eclipse-pullpiri/pullpiri:${VERSION}"
> # To:
> CONTAINER_IMAGE="localhost/pullpiri:latest"
> ```

### Verify Installation

```bash
podman pod ps
# pullpiri-server   Running
# pullpiri-player   Running

podman ps
# pullpiri-rocksdbservice   Running
# pullpiri-apiserver        Running
# pullpiri-policymanager    Running
# pullpiri-monitoringserver Running
# pullpiri-logservice       Running
# pullpiri-settingsservice  Running
# pullpiri-filtergateway    Running
# pullpiri-actioncontroller Running
# pullpiri-statemanager     Running

# Verify nodeagent systemd service
systemctl status nodeagent.service

# Verify API server is listening (no /api/health endpoint exists)
HOST_IP=$(hostname -I | awk '{print $1}')
curl -X GET "http://${HOST_IP}:47099/api/notify"
```

---

## Step 7: Development Validation

Before committing or releasing any code changes, always run the full validation sequence:

```bash
export PATH="$HOME/.cargo/bin:$PATH"

# 1. Format check (fast)
bash scripts/fmt_check.sh

# 2. Lint check
bash scripts/clippy_check.sh

# 3. Full build
make build

# 4. Unit tests (in specific component)
cargo test --manifest-path=src/Cargo.toml
```

---

## Uninstall

To stop and remove all Pullpiri containers and pods:

```bash
sudo bash containers/uninstall-pullpiri.sh
```

To clean build artifacts:

```bash
make clean
```

---

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| `apt update` fails in Docker build | DNS resolution issue inside container | Check host DNS: `cat /etc/resolv.conf` |
| `Out of disk space` | Build artifacts too large | Ensure ≥ 20 GB free disk; run `make clean` |
| `cargo: command not found` | Rust toolchain not in PATH | Run `export PATH="$HOME/.cargo/bin:$PATH"` |
| Podman permission error | Not running as root | Run install scripts with `sudo` |
| Port already in use | Another service using ports 47001-47099 | Check `ss -tlnp | grep 470` |

---

## Further Reading

- [Getting Started](./getting-started.md)
- [Tutorial – Run a Scenario](./tutorial.md)
- [Development Guide](./developments.md)
- [Project Structure](./structure.md)
