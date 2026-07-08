<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# SettingsCLI

A command-line interface tool for interacting with the Pullpiri SettingsService via REST APIs.

## Overview

SettingsCLI provides developers with a convenient way to access all REST APIs provided by the SettingsService. Since SettingsService runs as a container, this separated CLI app can be used anywhere on the system where developers need to interact with the service.

## Features

- **Metrics Operations**: Get system metrics and board information
- **Board Management**: List and inspect board configurations
- **Node Management**: Monitor and manage individual nodes
- **SoC Management**: Handle System-on-Chip resource management
- **Container Management**: List and inspect container configurations and stats
- **YAML Artifact Management**: Apply and withdraw YAML artifacts to/from the system
- **Health Checks**: Verify SettingsService connectivity
- **Flexible Output**: Support for both formatted and raw JSON output
- **Comprehensive Help**: Built-in help system for all commands

## Installation

Build the CLI tool as part of the Pullpiri project:

```bash
cd src/tools/settingscli
cargo build --release
```

Or build all tools together:

```bash
cd src/tools
cargo build --release
```

## Usage

### Basic Syntax

```bash
settingscli [OPTIONS] <COMMAND>
```

### Global Options

- `-u, --url <URL>`: SettingsService URL (default: http://localhost:8080)
- `-t, --timeout <SECONDS>`: Request timeout in seconds (default: 30)
- `-v, --verbose`: Enable verbose output
- `-h, --help`: Print help information
- `-V, --version`: Print version information

### Commands

#### Health Check

Test connection to SettingsService:

```bash
settingscli health
```

#### Metrics

Display live system metrics:

```bash
# Display formatted metrics
settingscli top metrics

# Get raw JSON metrics
settingscli raw metrics
```

#### Board Operations

```bash
# Get all boards
settingscli get board

# Describe specific board information
settingscli describe board <BOARD_ID>

# Get raw board data
settingscli raw board [BOARD_ID]
```

#### Node Operations

```bash
# Get all nodes
settingscli get node

# Describe specific node information
settingscli describe node <NODE_ID>

# Get raw node data
settingscli raw node [NODE_ID]
```

#### SoC Operations

```bash
# Get all SoCs
settingscli get soc

# Describe specific SoC information
settingscli describe soc <SOC_ID>

# Get raw SoC data
settingscli raw soc [SOC_ID]
```

#### Container Operations

```bash
# Get all containers
settingscli get containers

# Describe specific container information
settingscli describe container <CONTAINER_ID>

# Get raw container data
settingscli raw container
```

#### YAML Artifact Management

```bash
# Apply YAML artifact from file
settingscli yaml apply <FILE_PATH>

# Apply YAML artifact from stdin
settingscli yaml apply -

# Withdraw YAML artifact from file
settingscli yaml withdraw <FILE_PATH>

# Withdraw YAML artifact from stdin
settingscli yaml withdraw -
```

### Examples

```bash
# Check if SettingsService is running
settingscli health

# Display system metrics with custom URL and timeout
settingscli -u http://192.168.1.100:8080 -t 60 top metrics

# Get all boards with verbose output
settingscli -v get board

# Describe specific node details
settingscli describe node lg-OptiPlex-3070

# Get raw JSON output for a specific SoC
settingscli raw soc 10.221.40.190

# Get all containers
settingscli get container

# Describe specific container details by ID
settingscli describe container 2a465a2ea2d8ce9d35ab5eaae729067267ec09377edf89d02daa6c78d3787d2e

# Get raw container data
settingscli raw container

# Apply YAML artifact
settingscli yaml apply /path/to/artifact.yaml

# Apply YAML from stdin
cat artifact.yaml | settingscli yaml apply -

# Withdraw YAML artifact
settingscli yaml withdraw /path/to/artifact.yaml
```

### YAML Artifact Format

When using YAML operations, the artifact should be a multi-document YAML containing:

```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: example-scenario
spec:
  condition: null
  action: update
  target: example-target
---
apiVersion: v1
kind: Package
metadata:
  name: example-package
spec:
  pattern:
    - type: plain
  models:
    - name: example-model
      node: target-node
---
apiVersion: v1
kind: Model
metadata:
  name: example-model
  annotations:
    io.pullpiri.annotations.package-type: example
  labels:
    app: example
spec:
  hostNetwork: true
  containers:
    - name: example-container
      image: alpine:latest
  restartPolicy: Always
```

## Testing

### Running Tests

```bash
# Run unit tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_client_creation
```

### Integration Testing

With all services running, test the CLI:

```bash
# Test basic connectivity
./target/release/settingscli health

# Test metrics endpoint
./target/release/settingscli top metrics

# Test all resource endpoints
./target/release/settingscli get boards
./target/release/settingscli get nodes
./target/release/settingscli get socs
./target/release/settingscli get containers

# Test YAML operations (requires valid YAML file)
./target/release/settingscli yaml apply examples/helloworld.yaml
./target/release/settingscli yaml withdraw examples/helloworld.yaml
```

## Error Handling

The CLI provides clear error messages for common issues:

- **Connection errors**: When SettingsService is unreachable
- **Timeout errors**: When requests take too long
- **JSON parsing errors**: When response format is unexpected
- **HTTP errors**: When API endpoints return error status codes
- **File errors**: When YAML files cannot be read or parsed
- **YAML validation errors**: When YAML artifacts are missing required kinds

### Common Error Scenarios

```bash
# Service unreachable
$ settingscli health
✗ Failed to connect to SettingsService: Connection refused

# Invalid container ID
$ settingscli describe container invalid-id
✗ Failed to fetch container invalid-id: Request failed with status: 404 Not Found

# Missing YAML file
$ settingscli yaml apply nonexistent.yaml
✗ Failed to apply YAML artifact: File not found: nonexistent.yaml

# Server error during container retrieval
$ settingscli describe container <container-id>
✗ Failed to fetch container <container-id>: Request failed with status: 500 Internal Server Error
```

## Development

### Project Structure

```
src/tools/settingscli/
├── Cargo.toml          # Dependencies and metadata
├── src/
│   ├── main.rs         # CLI entry point
│   ├── lib.rs          # Library exports
│   ├── client.rs       # HTTP client implementation
│   ├── error.rs        # Error handling
│   └── commands/       # Command implementations
│       ├── mod.rs      # Command utilities
│       ├── top.rs      # Live system monitoring
│       ├── metrics.rs  # Metrics operations
│       ├── board.rs    # Board operations
│       ├── node.rs     # Node operations
│       ├── soc.rs      # SoC operations
│       ├── container.rs # Container operations
│       └── yaml.rs     # YAML artifact management
└── tests/              # Integration tests
    ├── integration_test.rs
    └── cli_test.rs
```

### Adding New Commands

1. Create a new file in `src/commands/`
2. Implement the command logic with appropriate error handling
3. Add the command to the main CLI enum in `main.rs`
4. Update the command dispatcher
5. Add tests for the new functionality

### Code Validation

Before committing changes, always run:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
scripts/fmt_check.sh
scripts/clippy_check.sh
make build
```

**NEVER CANCEL** these validation steps.

## API Endpoints

The CLI interacts with the following SettingsService REST API endpoints:

### Resource Management APIs

| Command | HTTP Method | Endpoint | Description |
|---------|-------------|----------|-------------|
| `top metrics` | GET | `/api/v1/metrics` | Display live system metrics |
| `raw metrics` | GET | `/api/v1/metrics` | Get raw metrics data |
| `get boards` | GET | `/api/v1/boards` | Get all boards |
| `describe board <id>` | GET | `/api/v1/boards/{id}` | Describe specific board |
| `raw board` | GET | `/api/v1/boards` | Get raw board data |
| `get nodes` | GET | `/api/v1/nodes` | Get all nodes |
| `describe node <id>` | GET | `/api/v1/nodes/{id}` | Describe specific node |
| `raw node` | GET | `/api/v1/nodes` | Get raw node data |
| `get socs` | GET | `/api/v1/socs` | Get all SoCs |
| `describe soc <id>` | GET | `/api/v1/socs/{id}` | Describe specific SoC |
| `raw soc` | GET | `/api/v1/socs` | Get raw SoC data |
| `get containers` | GET | `/api/v1/containers` | Get all containers |
| `describe container <id>` | GET | `/api/v1/containers/{id}` | Describe specific container |
| `raw container` | GET | `/api/v1/containers` | Get raw container data |

### YAML Artifact APIs

| Command | HTTP Method | Endpoint | Description |
|---------|-------------|----------|-------------|
| `yaml apply <file>` | POST | `/api/v1/yaml` | Apply YAML artifact |
| `yaml withdraw <file>` | DELETE | `/api/v1/yaml` | Withdraw YAML artifact |

### System APIs

| Command | HTTP Method | Endpoint | Description |
|---------|-------------|----------|-------------|
| `health` | GET | `/api/v1/system/health` | Health check |

### Response Formats

**Metrics Response:**
```json
[
  {
    "component": "node|container|soc|board",
    "id": "resource-id",
    "metric_type": "NodeInfo|ContainerInfo|SocInfo|BoardInfo",
    "timestamp": "2025-10-23T08:47:40.207254214Z",
    "value": {
      "type": "NodeInfo",
      "value": { "node_name": "...", "cpu_usage": 7.34, ... }
    }
  }
]
```

**Container Response:**
```json
[
  {
    "id": "container-id",
    "names": ["container-name"],
    "image": "docker.io/library/alpine:latest",
    "state": {
      "Status": "running",
      "Running": "true",
      "Pid": "1234"
    },
    "config": {
      "Hostname": "hostname",
      "User": "root"
    },
    "stats": {
      "CpuTotalUsage": "68328000",
      "MemoryUsage": "1548288"
    }
  }
]
```

**YAML Apply/Withdraw Response:**
```json
{
  "message": "YAML artifact applied successfully",
  "applied": [
    {"kind": "Scenario", "name": "example"},
    {"kind": "Package", "name": "example"},
    {"kind": "Model", "name": "example"}
  ]
}
```

## Troubleshooting

### Common Issues and Solutions

#### 1. Connection Issues
```bash
# Problem: Cannot connect to SettingsService
$ settingscli health
✗ Failed to connect to SettingsService: Connection refused

# Solution: Check if SettingsService is running and accessible
# Verify the correct URL and port (default: localhost:8080)
settingscli -u http://localhost:8080 health
```

#### 2. Container API 500 Errors
```bash
# Problem: Container DESCRIBE returns 500 Internal Server Error
$ settingscli describe container <container-id>
✗ Failed to fetch container: Request failed with status: 500 Internal Server Error

# Solution: This is a known server-side issue. Use get containers or raw container instead:
settingscli get containers
settingscli raw container
```

#### 3. Empty Results
```bash
# Problem: Commands return empty results
$ settingscli get boards
No boards found.

# Solution: Ensure all required services are running:
# 1. podman.socket service
sudo systemctl start podman.socket

# 2. Pullpiri services (in order)
cargo run --manifest-path=src/player/statemanager/Cargo.toml &
cargo run --manifest-path=src/server/monitoringserver/Cargo.toml &
cargo run --manifest-path=src/server/settingsservice/Cargo.toml &
cargo run --manifest-path=src/agent/nodeagent/Cargo.toml &
```

#### 4. YAML Validation Warnings
```bash
# Problem: YAML apply shows missing kinds warning
⚠ Warning: Missing recommended kinds: Package, Model
The API Server expects Scenario, Package, and Model kinds for proper operation.

# Solution: Ensure your YAML contains all required document types:
# - Scenario (defines the operation)
# - Package (defines the package structure)
# - Model (defines the container/pod specification)
```

#### 5. Timeout Issues
```bash
# Problem: Requests timing out
$ settingscli get metrics
✗ Request timed out

# Solution: Increase timeout for slow responses
settingscli -t 60 get metrics
```

### Service Dependencies

The CLI requires these services to be running for full functionality:

1. **podman.socket** - Container management
2. **statemanager** - Application state management
3. **monitoringserver** - System metrics collection
4. **settingsservice** - REST API server
5. **nodeagent** - Node resource monitoring

### Debug Mode

Enable verbose output for troubleshooting:

```bash
# Enable verbose output
settingscli -v <command>

# Example with connection details
settingscli -v -u http://192.168.1.100:8080 health
```

## Complete Command Reference

### Global Commands

```bash
# Show help for all commands
settingscli --help

# Show version information
settingscli --version

# Test connectivity to SettingsService
settingscli health
settingscli -u http://custom-host:8080 health
```

### Metrics Commands

```bash
# Get formatted system metrics
settingscli get metrics

# Get raw JSON metrics data
settingscli raw metrics

# With custom settings
settingscli -u http://remote-host:8080 -t 30 get metrics
```

### Board Commands

```bash
# Get all boards (formatted)
settingscli get boards

# Describe specific board details
settingscli describe board 10.221.40.100

# Get raw board data (all boards)
settingscli raw board

# Get raw data for specific board
settingscli raw board 10.221.40.100
```

### Node Commands

```bash
# Get all nodes (formatted)
settingscli get nodes

# Describe specific node details
settingscli describe node lg-OptiPlex-3070

# Get raw node data (all nodes)
settingscli raw node

# Get raw data for specific node
settingscli raw node lg-OptiPlex-3070
```

### SoC Commands

```bash
# Get all SoCs (formatted)
settingscli get soc

# Describe specific SoC details
settingscli describe soc 10.221.40.190

# Get raw SoC data (all SoCs)
settingscli raw soc

# Get raw data for specific SoC
settingscli raw soc 10.221.40.190
```

### Container Commands

```bash
# Get all containers (formatted)
settingscli get containers

# Describe specific container details (may return 500 error - known issue)
settingscli describe container 2a465a2ea2d8ce9d35ab5eaae729067267ec09377edf89d02daa6c78d3787d2e

# Get raw container data (recommended for detailed info)
settingscli raw container
```

### YAML Artifact Commands

```bash
# Apply YAML artifact from file
settingscli yaml apply /path/to/artifact.yaml
settingscli yaml apply ./examples/helloworld.yaml

# Apply YAML from stdin
cat artifact.yaml | settingscli yaml apply -
echo "apiVersion: v1..." | settingscli yaml apply -

# Withdraw YAML artifact from file
settingscli yaml withdraw /path/to/artifact.yaml

# Withdraw YAML from stdin
cat artifact.yaml | settingscli yaml withdraw -
```

### Advanced Usage Examples

```bash
# Chain commands with jq for JSON processing
settingscli raw metrics | jq '.[] | select(.component == "node")'

# Save raw output to file
settingscli raw container > containers.json

# Check specific container by name pattern
settingscli raw container | jq '.[] | select(.names[0] | contains("alpine"))'

# Apply YAML with error handling
settingscli yaml apply deployment.yaml && echo "Success" || echo "Failed"

# Use custom timeout for slow networks
settingscli -t 120 -u http://remote-vehicle:8080 get board

# Verbose mode with custom settings
settingscli -v -u http://192.168.1.100:8080 -t 60 get metrics
```

## Related Components

- **SettingsService**: The REST API server this CLI communicates with (port 8080)
- **Pullpiri Core**: The main vehicle service orchestrator framework
- **Monitoring Server**: Provides system metrics and monitoring data (port 47001+)
- **State Manager**: Manages application state and configuration
- **Node Agent**: Reports node resource utilization and container status
- **API Server**: Processes YAML artifacts for vehicle service orchestration (port 47099)

### Service Architecture

```
┌─────────────────┐    HTTP/REST    ┌─────────────────┐
│   SettingsCLI   │◄──────────────► │ SettingsService │
│                 │                 │   (port 8080)   │
└─────────────────┘                 └─────────────────┘
                                            │
                                            │
                                            ▼
                                    ┌─────────────────┐
                                    │     RocksDB     │
                                    │  (port 47007)   │
                                    └─────────────────┘
                                            ▲
                    ┌───────────────────────┼───────────────────────┐
                    │                       │                       │
            ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
            │ MonitoringServer│    │   StateManager  │    │    NodeAgent    │
            │   (gRPC/REST)   │    │   (gRPC/REST)   │    │   (gRPC/REST)   │
            └─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Port Allocation

Following Pullpiri networking conventions:
- **SettingsService**: `8080` (configurable within 47001-47099 range)
- **SettingsCLI**: Connects to SettingsService (no listening port)
- **API Server**: `47099` (receives YAML artifacts from SettingsService)
- **Other Pullpiri Services**: `47001-47099` (gRPC: 47001+, REST: up to 47099)
- **RocksDB**: `47007`

## License

This project is licensed under the Apache-2.0 license.