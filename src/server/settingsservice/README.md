# PICCOLO Settings Service

The Settings Service is a core component of the PICCOLO framework that provides centralized configuration management and metrics filtering capabilities.

## Features

- **Configuration Management**: Create, read, update, and delete YAML/JSON configurations
- **Schema Validation**: Validate configurations against JSON schemas
- **Change History**: Track configuration changes with rollback capabilities
- **Metrics Filtering**: Filter and serve monitoring metrics from ETCD
- **Multiple Interfaces**: REST API and CLI interfaces
- **ETCD Integration**: Uses ETCD as the backend storage

## Architecture

The Settings Service consists of the following modules:

- `settings_core`: Service initialization and coordination
- `settings_config`: Configuration management with YAML/JSON support
- `settings_history`: Change history tracking and rollback
- `settings_monitoring`: Metrics data filtering from ETCD
- `settings_storage`: ETCD client for data persistence
- `settings_api`: REST API server
- `settings_cli`: Command-line interface
- `settings_utils`: Common utilities

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

### Metrics Management

- `GET /api/v1/metrics` - Get filtered metrics
- `GET /api/v1/metrics/{id}` - Get specific metric
- `GET /api/v1/metrics/component/{component}` - Get metrics by component
- `GET /api/v1/metrics/filters` - List metric filters
- `POST /api/v1/metrics/filters` - Create metric filter

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

### Metrics Commands

```bash
metrics list                   # List all metrics
metrics get <id>               # Get specific metric
metrics filter <component>     # Filter metrics by component
metrics filters                # List all filters
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
curl -X POST http://localhost:8080/api/v1/settings/myapp/config \
  -H "Content-Type: application/json" \
  -d '{
    "content": {
      "database": {
        "host": "localhost",
        "port": 5432
      }
    },
    "schema_type": "database-config",
    "author": "admin",
    "comment": "Initial database configuration"
  }'
```

### Get Configuration

```bash
curl http://localhost:8080/api/v1/settings/myapp/config
```

### Filter Metrics

```bash
curl "http://localhost:8080/api/v1/metrics?component=nodeagent&metric_type=gauge"
```

## Dependencies

- Rust 1.70+
- ETCD 3.5+
- Protocol Buffers compiler (protoc)

## License

Apache-2.0