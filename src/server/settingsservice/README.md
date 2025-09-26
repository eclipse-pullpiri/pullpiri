# PICCOLO Settings Service

The Settings Service is a core component of the PICCOLO framework that provides centralized configuration management and metrics filtering capabilities for vehicle service orchestration.

## Features

- **Configuration Management**: Create, read, update, and delete YAML/JSON configurations
- **Schema Validation**: Validate configurations against JSON schemas
- **Change History**: Track configuration changes with rollback capabilities
- **Metrics Retrieval**: Retrieve and filter monitoring metrics from ETCD (NodeInfo, ContainerInfo, SocInfo, BoardInfo)
- **Resource Management**: List vehicle orchestration resources (nodes, containers, SoCs, boards)
- **Multiple Interfaces**: REST API interface
- **ETCD Integration**: Direct integration with monitoring ETCD storage for real-time vehicle orchestration data
- **YAML Artifact Management**: Apply and withdraw YAML artifacts through API Server integration

## Architecture

The Settings Service consists of the following modules:

- `settings_core`: Service initialization and coordination
- `settings_config`: Configuration management with YAML/JSON support
- `settings_history`: Change history tracking and rollback
- `settings_monitoring`: High-level metrics data retrieval and filtering with caching (returns both Metric objects with labels and raw resource objects)
- `monitoring_etcd`: Direct ETCD operations for monitoring data (`/piccolo/metrics/`, `/piccolo/logs/`)
- `monitoring_types`: Type definitions for vehicle orchestration metrics (NodeInfo, SocInfo, BoardInfo)
- `settings_storage`: ETCD client for configuration data persistence
- `settings_api`: REST API server with comprehensive metrics endpoints
- `settings_utils`: Common utilities (error handling, logging, YAML processing)

## Building

```bash
# Build the settings service
cd src/server/settingsservice
cargo build

# Or build the entire project
make build
```

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

## REST API

The Settings Service provides a comprehensive REST API:

### Configuration Management

- `GET /api/v1/settings` - List all configurations
- `GET /api/v1/settings/{path}` - Get specific configuration
- `POST /api/v1/settings/{path}` - Create new configuration
- `PUT /api/v1/settings/{path}` - Update configuration
- `DELETE /api/v1/settings/{path}` - Delete configuration
- `POST /api/v1/settings/validate` - Validate configuration

### Vehicle Resource Management

**Node Management:**
- `GET /api/v1/nodes` - List all nodes
- `GET /api/v1/nodes/{name}` - Get specific node
- `GET /api/v1/nodes/{name}/pods/metrics` - Get pod metrics for specific node (enhanced with hostname)
- `GET /api/v1/nodes/{name}/containers` - Get all containers for specific node

**Container Management:**
- `GET /api/v1/containers` - List all containers
- `GET /api/v1/containers/{id}` - Get specific container (includes logs)

**SoC Management:**
- `GET /api/v1/socs` - List all SoCs
- `GET /api/v1/socs/{name}` - Get specific SoC

**Board Management:**
- `GET /api/v1/boards` - List all boards
- `GET /api/v1/boards/{name}` - Get specific board

### YAML Artifact Management

**New endpoints for YAML operations:**
- `POST /api/v1/yaml` - Apply YAML artifact (forwards to API Server)
- `DELETE /api/v1/yaml` - Withdraw YAML artifact (forwards to API Server)

### Metrics Management (Vehicle Orchestration)

**Enhanced endpoints with direct ETCD access:**

- `GET /api/v1/metrics` - Get all metrics from ETCD with optional filtering
- `GET /api/v1/metrics/nodes` - Get all node metrics (NodeInfo)
- `GET /api/v1/metrics/containers` - Get all container metrics (ContainerInfo)
- `GET /api/v1/metrics/socs` - Get all SoC metrics (SocInfo)
- `GET /api/v1/metrics/boards` - Get all board metrics (BoardInfo)
- `GET /api/v1/metrics/nodes/{node_name}` - Get specific node metric
- `GET /api/v1/metrics/containers/{container_id}` - Get specific container metric
- `GET /api/v1/metrics/filters` - List metric filters
- `POST /api/v1/metrics/filters` - Create metric filter
- `DELETE /api/v1/metrics/{component}/{id}` - Delete specific metric

**Query parameters for filtering:**
- `?component=node|container|soc|board` - Filter by component type
- `?max_items=N` - Limit number of results
- `?metric_type=NodeInfo|ContainerInfo|SocInfo|BoardInfo` - Filter by metric type
- `?filter=search_term` - Filter by resource name/ID

### History Management

- `GET /api/v1/history/{path}` - Get configuration history
- `GET /api/v1/history/{path}/version/{version}` - Get specific version
- `POST /api/v1/history/{path}/rollback/{version}` - Rollback to version

### System Information

- `GET /api/v1/system/status` - Get system status
- `GET /api/v1/system/health` - Health check
- `POST /api/v1/monitoring/sync` - Sync with monitoring server

## Configuration

The service can be configured using command-line arguments or environment variables:

- `--config`: Configuration file path (default: `/etc/piccolo/settings.yaml`)
- `--etcd-endpoints`: ETCD endpoints (default: `localhost:2379`)
- `--bind-address`: HTTP server bind address (default: `0.0.0.0`)
- `--bind-port`: HTTP server bind port (default: `8080`)
- `--log-level`: Log level (default: `info`)

## Testing

```bash
# Run tests
cargo test

# Run with output
cargo test -- --nocapture

# Validate code formatting
export PATH="$HOME/.cargo/bin:$PATH" && scripts/fmt_check.sh

# Run linting checks
export PATH="$HOME/.cargo/bin:$PATH" && scripts/clippy_check.sh
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

### Get All Node Metrics

```bash
curl http://localhost:8080/api/v1/metrics/nodes
```

### Get Specific Node Metrics

```bash
curl http://localhost:8080/api/v1/metrics/nodes/vehicle-ecu-01
```

### Get All Container Metrics

```bash
curl http://localhost:8080/api/v1/metrics/containers
```

### Get Container Metrics for Specific Container (with logs)

```bash
curl http://localhost:8080/api/v1/containers/vehicle-diagnostics
```

### Get Metrics with Labels (returns Metric objects)

```bash
# Get all metrics with labels and filtering support
curl http://localhost:8080/api/v1/metrics

# Get only container metrics with labels
curl "http://localhost:8080/api/v1/metrics?component=container"
```

### Filter Resources with Query Parameters

```bash
# Get only node metrics, limit to 10 items
curl "http://localhost:8080/api/v1/metrics/nodes?page_size=10"

# Filter containers by name
curl "http://localhost:8080/api/v1/containers?filter=diagnostics"

# Get all SoC metrics
curl "http://localhost:8080/api/v1/metrics/socs"

# Get all board metrics
curl "http://localhost:8080/api/v1/metrics/boards"
```

### Apply YAML Artifacts

```bash
curl -X POST http://localhost:8080/api/v1/yaml \
  -H "Content-Type: text/plain" \
  -d 'apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition: null
  action: update
  target: helloworld
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld
      node: lge-NUC11TNHi5
      resources:
        volume:
        network:
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld
  annotations:
    io.piccolo.annotations.package-type: helloworld
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: quay.io/podman/hello:latest
  terminationGracePeriodSeconds: 0
  restartPolicy: Always'
```

### Withdraw YAML Artifacts

To withdraw (delete) a YAML artifact, you must also provide a **multi-document YAML** containing all required kinds (`Scenario`, `Package`, and `Model`). The API Server expects the full artifact definition for proper deletion.

```bash
curl -X DELETE http://localhost:8080/api/v1/yaml \
  -H "Content-Type: text/plain" \
  -d 'apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition: null
  action: update
  target: helloworld'
```

**Note:**  
- Always pass the full YAML artifact (Scenario, Package, Model) for both apply and withdraw operations.
- The API Server will reject requests missing required kinds.

### Enable Debug Logging for Troubleshooting

```bash
RUST_LOG=debug ./target/debug/settingsservice
```

## Request/Response Schemas

### YAML Artifact Request

```bash
# Content-Type: text/plain
# Body: Raw YAML content
POST /api/v1/yaml
DELETE /api/v1/yaml
```

### Pod Metrics Response (Enhanced)

```json
{
  "node_name": "string",
  "hostname": "string (optional)",
  "pod_count": "number",
  "pods": [
    {
      "container_id": "string",
      "container_name": "string (optional)",
      "image": "string",
      "status": "string (optional)",
      "node_name": "string",
      "hostname": "string (optional)",
      "labels": {
        "key": "value"
      },
      "created_at": "ISO 8601 timestamp"
    }
  ]
}
```

### Metric Response (with labels)

```json
{
  "id": "string",
  "component": "node|container|soc|board",
  "metric_type": "NodeInfo|ContainerInfo|SocInfo|BoardInfo",
  "labels": {
    "container_id": "string",
    "image": "string",
    "status": "string",
    "hostname": "string"
  },
  "value": {
    "type": "ContainerInfo|NodeInfo|SocInfo|BoardInfo",
    "value": "... resource object ..."
  },
  "timestamp": "ISO 8601 timestamp"
}
```

### Query Parameters (Enhanced)

**All resource queries support:**
- `?page=N` - Page number for pagination
- `?page_size=N` - Number of items per page
- `?filter=search_term` - Filter by resource name/ID

**Metrics queries additionally support:**
- `?component=node|container|soc|board` - Filter by component type
- `?metric_type=NodeInfo|ContainerInfo|SocInfo|BoardInfo` - Filter by metric type
- `?filter_id=string` - Use existing filter by ID

## API Response Types

The Settings Service provides two types of responses for resource data:

1. **Raw Resource Objects** (e.g., `/api/v1/metrics/containers`)
   - Returns `ContainerInfo`, `NodeInfo`, `SocInfo`, `BoardInfo` directly
   - Suitable for simple resource listing and details

2. **Metric Objects with Labels** (e.g., `/api/v1/metrics`)
   - Returns `Metric` objects containing resource data plus metadata
   - Includes labels, timestamps, and filtering capabilities
   - Suitable for advanced monitoring and analytics

## Vehicle Service Orchestration Integration

The Settings Service integrates directly with the Pullpiri vehicle orchestration framework:

- **MonitoringServer**: Stores vehicle node, container, SoC, and board metrics in ETCD at `/piccolo/metrics/`
- **NodeAgent**: Reports node resource utilization and container status to MonitoringServer
- **APIServer**: Consumes configurations for orchestration policies and resource management; receives YAML artifacts forwarded by Settings Service
- **ETCD**: Central storage for both configurations (`/piccolo/settings/`) and real-time metrics (`/piccolo/metrics/`)

## Port Usage

Following Pullpiri networking conventions:
- **Settings Service**: `8080` (configurable within Pullpiri's 47001-47099 range)
- **API Server**: `47099` (for YAML artifact forwarding)
- **ETCD**: `2379, 2380` (standard ETCD ports)
- **Other Pullpiri Services**: `47001-47099` (gRPC: 47001+, REST: up to 47099)

## Error Handling

The API returns standard HTTP status codes:

- `200 OK` - Successful operation
- `400 Bad Request` - Invalid request data
- `404 Not Found` - Resource not found
- `500 Internal Server Error` - Server error

Error responses include detailed error messages:

```json
{
  "error": "Error description",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## Dependencies

- Rust 1.70+
- ETCD 3.5+
- Protocol Buffers compiler (protoc)
- API Server (for YAML artifact operations)

## License

Apache-2.0 (following Pullpiri framework licensing)