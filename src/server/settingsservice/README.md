# PICCOLO Settings Service

The Settings Service is a core component of the PICCOLO framework that provides centralized configuration management and metrics filtering capabilities for vehicle service orchestration.

## Features

- **Configuration Management**: Create, read, update, and delete YAML/JSON configurations
- **Schema Validation**: Validate configurations against JSON schemas
- **Change History**: Track configuration changes with rollback capabilities
- **Metrics Retrieval**: Retrieve and filter monitoring metrics from ETCD (NodeInfo, ContainerInfo, SocInfo, BoardInfo)
- **Multiple Interfaces**: REST API and CLI interfaces
- **ETCD Integration**: Direct integration with monitoring ETCD storage for real-time vehicle orchestration data

## Architecture

The Settings Service consists of the following modules:

- `settings_core`: Service initialization and coordination
- `settings_config`: Configuration management with YAML/JSON support
- `settings_history`: Change history tracking and rollback
- `settings_monitoring`: High-level metrics data retrieval and filtering with caching
- `monitoring_etcd`: Direct ETCD operations for monitoring data (`/piccolo/metrics/`, `/piccolo/logs/`)
- `monitoring_types`: Type definitions for vehicle orchestration metrics (NodeInfo, SocInfo, BoardInfo)
- `settings_storage`: ETCD client for configuration data persistence
- `settings_api`: REST API server with comprehensive metrics endpoints
- `settings_cli`: Command-line interface with interactive shell
- `settings_utils`: Common utilities (error handling, logging, YAML processing)

## Building

# Build the settings service
cd src/server/settingsservice
cargo build

# Or build the entire project
make build

## Running

### Server Mode

```bash
# Run the server with default settings
./target/debug/settingsservice

# Run with custom configuration
./target/debug/settingsservice \
  --etcd-endpoints localhost:2379,localhost:2380 \
  --bind-address 0.0.0.0 \
  --bind-port 8080 \
  --log-level info
```

### CLI Mode

```bash
# Run the CLI
./target/debug/settingsservice --cli

# Or use the dedicated CLI binary
./target/debug/settings-cli
```

## REST API

The Settings Service provides a comprehensive REST API:

### Configuration Management

- `GET /api/v1/settings` - List all configurations
- `GET /api/v1/settings/{path}` - Get specific configuration
- `POST /api/v1/settings/{path}` - Create new configuration
- `PUT /api/v1/settings/{path}` - Update configuration
- `DELETE /api/v1/settings/{path}` - Delete configuration
- `POST /api/v1/settings/validate` - Validate configuration

### Metrics Management (Vehicle Orchestration)

**Enhanced endpoints with direct ETCD access:**

- `GET /api/v1/metrics` - Get all metrics from ETCD with optional filtering
- `GET /api/v1/metrics/nodes` - Get all node metrics (NodeInfo) - **FIXED**
- `GET /api/v1/metrics/containers` - Get all container metrics (ContainerInfo) - **FIXED**
- `GET /api/v1/metrics/socs` - Get all SoC metrics (SocInfo) - **FIXED**
- `GET /api/v1/metrics/boards` - Get all board metrics (BoardInfo) - **FIXED**
- `GET /api/v1/metrics/nodes/{node_name}` - Get specific node metric - **FIXED**
- `GET /api/v1/metrics/containers/{container_id}` - Get specific container metric - **FIXED**
- `GET /api/v1/metrics/filters` - List metric filters
- `POST /api/v1/metrics/filters` - Create metric filter
- `DELETE /api/v1/metrics/{component}/{id}` - Delete specific metric

**Query parameters for filtering:**
- `?component=node|container|soc|board` - Filter by component type
- `?max_items=N` - Limit number of results
- `?metric_type=NodeInfo|ContainerInfo|SocInfo|BoardInfo` - Filter by metric type

### History Management

- `GET /api/v1/history/{path}` - Get configuration history
- `GET /api/v1/history/{path}/version/{version}` - Get specific version
- `POST /api/v1/history/{path}/rollback/{version}` - Rollback to version

### System Information

- `GET /api/v1/system/status` - Get system status
- `GET /api/v1/system/health` - Health check

## CLI Commands

The CLI provides an interactive shell with the following commands:

### Configuration Commands

```bash
config list [prefix]           # List configurations
config get <path>              # Get configuration
config set <path> <value>      # Set configuration
config delete <path>           # Delete configuration
config validate <path>         # Validate configuration
```

### Metrics Commands (Vehicle Orchestration)

```bash
metrics nodes                  # List all node metrics
metrics containers             # List all container metrics  
metrics socs                   # List all SoC metrics
metrics boards                 # List all board metrics
metrics node <name>            # Get specific node metric
metrics container <id>         # Get specific container metric
metrics stats                  # Show metrics statistics
```

### History Commands

```bash
history <path>                 # Show configuration history
history rollback <path> <ver>  # Rollback to version
```

## Configuration

The service can be configured using command-line arguments or environment variables:

- `--config`: Configuration file path (default: `/etc/piccolo/settings.yaml`)
- `--etcd-endpoints`: ETCD endpoints (default: `localhost:2379`)
- `--bind-address`: HTTP server bind address (default: `0.0.0.0`)
- `--bind-port`: HTTP server bind port (default: `8080`)
- `--log-level`: Log level (default: `info`)
- `--cli`: Run in CLI mode instead of server mode

## Testing

```bash
# Run tests
cargo test

# Run with output
cargo test -- --nocapture
```

## Example Usage

### Create a Configuration

```bash
curl -X POST http://localhost:8080/api/v1/settings/vehicle/orchestrator \
  -H "Content-Type: application/json" \
  -d '{
    "content": {
      "node_selection": {
        "strategy": "resource_based",
        "cpu_threshold": 80.0,
        "memory_threshold": 90.0
      },
      "container_policy": {
        "restart_policy": "always",
        "resource_limits": {
          "cpu": "2.0",
          "memory": "4Gi"
        }
      }
    },
    "schema_type": "orchestrator-config",
    "author": "vehicle-admin",
    "comment": "Vehicle service orchestration configuration"
  }'
```

### Get Configuration

```bash
curl http://localhost:8080/api/v1/settings/vehicle/orchestrator
```

### Get All Node Metrics - **WORKING**

```bash
curl http://localhost:8080/api/v1/metrics/nodes
```

### Get Specific Node Metrics

```bash
curl http://localhost:8080/api/v1/metrics/nodes/vehicle-ecu-01
```

### Get All Container Metrics - **WORKING**

```bash
curl http://localhost:8080/api/v1/metrics/containers
```

### Get Container Metrics for Specific Container

```bash
curl http://localhost:8080/api/v1/metrics/containers/db368045fa4d40ffa3ba8cae61eeb9df36e120a2350e36d71e547b7ce3f1a9d5
```

### Filter Metrics with Query Parameters

```bash
# Get only node metrics, limit to 10 items
curl "http://localhost:8080/api/v1/metrics?component=node&max_items=10"

# Get all container metrics
curl "http://localhost:8080/api/v1/metrics?component=container"

# Get all SoC metrics
curl "http://localhost:8080/api/v1/metrics/socs"

# Get all board metrics
curl "http://localhost:8080/api/v1/metrics/boards"
```

### Enable Debug Logging for Troubleshooting

```bash
RUST_LOG=debug ./target/debug/settingsservice
```

## Vehicle Service Orchestration Integration

The Settings Service integrates directly with the Pullpiri vehicle orchestration framework:

- **MonitoringServer**: Stores vehicle node, container, SoC, and board metrics in ETCD at `/piccolo/metrics/`
- **NodeAgent**: Reports node resource utilization and container status to MonitoringServer
- **APIServer**: Consumes configurations for orchestration policies
- **ETCD**: Central storage for both configurations (`/piccolo/settings/`) and real-time metrics (`/piccolo/metrics/`)

## Port Usage

Following Pullpiri networking conventions:
- **Settings Service**: `8080` (configurable within Pullpiri's 47001-47099 range)
- **ETCD**: `2379, 2380` (standard ETCD ports)
- **Other Pullpiri Services**: `47001-47099` (gRPC: 47001+, REST: up to 47099)

## Dependencies

- Rust 1.70+
- ETCD 3.5+
- Protocol Buffers compiler (protoc)

## License

Apache-2.0 (following Pullpiri framework licensing)