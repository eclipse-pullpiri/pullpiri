<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Getting Started

Pullpiri is a Rust-based vehicle service orchestrator framework that enables efficient deployment and management of cloud-native in-vehicle services. It uses a microservices architecture with server, agent, and player components to orchestrate containerized workloads on vehicle systems.

This guide walks you through three paths to get Pullpiri running on your system:

| Path | Description |
|------|-------------|
| [Quick Start](#quick-start) | Deploy pre-built container images from GitHub Container Registry |
| [Build from Source](./build.md) | Build Docker images and binaries from source code |
| [Tutorial](./tutorial.md) | Run a built-in example scenario end-to-end |

---

## System Requirements

Pullpiri has been tested on the following Linux distributions:

- Ubuntu 24.04 LTS
- CentOS Stream 9

### Minimum Hardware Requirements

| Resource | Minimum |
|----------|---------|
| CPU | 2 cores |
| RAM | 512 MB |
| Disk | 20 GB |
| Architecture | `x86_64` or `aarch64` |

### Software Prerequisites

| Software | Version | Notes |
|----------|---------|-------|
| [Podman](https://podman.io/) | ≥ 4.0.0 | Required container runtime |
| Linux kernel | ≥ 5.10 | Host networking support |

---

## Quick Start

The fastest way to get Pullpiri running is to use the pre-built container images published to the GitHub Container Registry.

### Step 1: Install Prerequisites

#### Install Podman

**Ubuntu 22.04 / 24.04:**

```bash
sudo apt update
sudo apt install -y podman
```

**CentOS Stream 9 / RHEL:**

```bash
sudo dnf install -y podman
```

Verify Podman installation:

```bash
podman --version
# podman version 4.x.x or higher
```

#### Prepare System

```bash
# Create required directories
sudo mkdir -p /etc/pullpiri
sudo mkdir -p /run/pullpirilog
```

#### Open Required Ports

Pullpiri uses the following TCP ports. Configure your firewall to allow them:

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

### Step 2: Clone the Repository

The install scripts are included in the repository:

```bash
git clone https://github.com/eclipse-pullpiri/pullpiri.git
cd pullpiri
```

### Step 3: Deploy Pullpiri

The install script pulls all container images from GitHub Container Registry automatically and deploys them as Podman pods.

```bash
# Run as root (required for Podman pod and system configuration)
sudo bash containers/install-pullpiri.sh
```

The script will:
1. Create the `/etc/pullpiri/settings.yaml` configuration file.
2. Pull `ghcr.io/eclipse-pullpiri/pullpiri:latest` (server and player components).
3. Pull `ghcr.io/mco-piccolo/pullpiri-rocksdb:v11.18.0` (RocksDB storage service).
4. Start the `pullpiri-server` Podman pod (RocksDB, APIServer, PolicyManager, MonitoringServer, LogService, SettingsService).
5. Start the `pullpiri-player` Podman pod (FilterGateway, ActionController, StateManager).
6. Download the `nodeagent` binary from GitHub Releases and register it as a **systemd service** (`nodeagent.service`).

> **Note:** The first run may take a few minutes while container images are downloaded.

### Step 4: Verify Installation

Check that all containers are running:

```bash
podman pod ps
# NAME              STATUS    ...
# pullpiri-server   Running   ...
# pullpiri-player   Running   ...

podman ps
# pullpiri-rocksdbservice   Running ...
# pullpiri-apiserver        Running ...
# pullpiri-policymanager    Running ...
# pullpiri-monitoringserver Running ...
# pullpiri-logservice       Running ...
# pullpiri-settingsservice  Running ...
# pullpiri-filtergateway    Running ...
# pullpiri-actioncontroller Running ...
# pullpiri-statemanager     Running ...
```

Verify the nodeagent systemd service is running:

```bash
systemctl status nodeagent.service
# ● nodeagent.service - Pullpiri NodeAgent Service
#    Active: active (running) ...
```

Verify the API server is listening:

```bash
HOST_IP=$(hostname -I | awk '{print $1}')
curl -X GET "http://${HOST_IP}:47099/api/notify"
```

### Service Ports

| Service | Port | Protocol |
|---------|------|----------|
| API Server (REST) | 47099 | REST (HTTP) |
| API Server (gRPC) | 47098 | gRPC |
| Action Controller | 47001 | gRPC |
| Filter Gateway | 47002 | gRPC |
| Monitoring Server | 47003 | gRPC |
| NodeAgent | 47004 | gRPC |
| Policy Manager | 47005 | gRPC |
| State Manager | 47006 | gRPC |
| RocksDB Service | 47007 | gRPC |
| Settings Service | 8080 | REST (HTTP) |

### Uninstall

To stop and remove all Pullpiri containers and pods:

```bash
sudo make uninstall
# equivalent to: sudo bash containers/uninstall-pullpiri.sh
```

---

## Build from Source

If you need to build Pullpiri from source (e.g., for custom modifications or specific architectures), see the detailed **[Build Guide](./build.md)**.

The build guide covers:
- Building the Docker/Podman container image from the project `Dockerfile`
- Compiling all Rust binaries inside the container build stage
- Installing the built images on the target system

---

## Tutorial

Once Pullpiri is installed and running, follow the **[Tutorial](./tutorial.md)** to deploy your first vehicle service scenario.

The tutorial guides you through:
- Running the built-in `helloworld` scenario from the `examples/` directory
- Verifying the deployed container workload
- Understanding the Scenario → Package → Model resource model

---

## Configuration Reference

The main configuration file is automatically created at `/etc/pullpiri/settings.yaml` during installation:

```yaml
host:
  name: <hostname>        # Node hostname (auto-detected)
  ip: <host-ip>           # Host IP address (auto-detected)
  type: vehicle           # Node type
  role: master            # Node role (master or nodeagent)
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
```

To add remote guest nodes (multi-node setup), edit `/etc/pullpiri/settings.yaml`:

```yaml
host:
  name: HPC
  ip: 192.168.0.100
  type: vehicle
  role: master
guest:
  - name: ZONE1
    ip: 192.168.0.101
    type: vehicle
    role: nodeagent
```

---

## Further Reading

- [Project Structure](./structure.md)
- [API Reference](./pullpiri-apis.md)
- [Development Guide](./developments.md)
- [Release Notes](./release.md)
