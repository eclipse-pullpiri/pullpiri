# Settings Server

The Settings Server provides REST API endpoints for managing monitoring configuration in the PULLPIRI system. It is designed to support Web GUI Tools that need to configure and manage monitoring settings.

## Overview

The Settings Server focuses on monitoring functionality and provides:

- **Monitoring Settings Management**: Create, read, update, and delete monitoring configurations
- **Monitoring Status**: Real-time status information about the monitoring system
- **REST APIs**: JSON-based APIs compatible with web applications
- **Validation**: Input validation for configuration parameters

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Web GUI Tool  │────│  Settings Server │────│  Monitoring     │
│                 │    │  (Port 47007)    │    │  Infrastructure │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

The Settings Server runs on port 47007 and provides RESTful APIs for the Web GUI Tool to manage monitoring configurations.

## API Endpoints

### Monitoring Settings

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/monitoring/settings` | Get all monitoring settings |
| GET | `/api/monitoring/settings/{id}` | Get specific monitoring settings |
| POST | `/api/monitoring/settings` | Create new monitoring settings |
| PUT | `/api/monitoring/settings/{id}` | Update monitoring settings |
| DELETE | `/api/monitoring/settings/{id}` | Delete monitoring settings |

### Monitoring Status

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/monitoring/status` | Get current monitoring system status |

## Data Structures

### MonitoringSettings

```json
{
  "id": "string",                       // Unique identifier
  "monitoring_interval": 30,            // Monitoring interval in seconds
  "container_monitoring_enabled": true, // Enable/disable container monitoring
  "resource_alert_threshold": 80,       // Alert threshold (0-100)
  "data_retention_days": 30,            // Data retention period in days
  "detailed_logging": false             // Enable detailed logging
}
```

### MonitoringStatus

```json
{
  "is_running": true,                   // Service running status
  "last_update": "2025-08-11T10:48:57.736028341+00:00", // Last update timestamp
  "monitored_containers": 5,            // Number of monitored containers
  "health_status": "healthy"            // System health status
}
```

## Usage Examples

### Get All Settings

```bash
curl -X GET http://localhost:47007/api/monitoring/settings
```

### Create New Settings

```bash
curl -X POST http://localhost:47007/api/monitoring/settings \
  -H "Content-Type: application/json" \
  -d '{
    "id": "web_gui",
    "monitoring_interval": 60,
    "container_monitoring_enabled": true,
    "resource_alert_threshold": 90,
    "data_retention_days": 7,
    "detailed_logging": true
  }'
```

### Update Settings

```bash
curl -X PUT http://localhost:47007/api/monitoring/settings/web_gui \
  -H "Content-Type: application/json" \
  -d '{
    "id": "web_gui",
    "monitoring_interval": 120,
    "container_monitoring_enabled": true,
    "resource_alert_threshold": 85,
    "data_retention_days": 14,
    "detailed_logging": false
  }'
```

### Get Monitoring Status

```bash
curl -X GET http://localhost:47007/api/monitoring/status
```

## Configuration

The Settings Server uses the same configuration system as other PULLPIRI components, reading from the common settings file. It automatically:

- Initializes with default monitoring settings
- Validates all input parameters
- Provides CORS support for web applications
- Handles errors gracefully with proper HTTP status codes

## Development

### Building

```bash
cd src/server
cargo build -p settingsserver
```

### Testing

```bash
cd src/server
cargo test -p settingsserver
```

### Running

```bash
cd src/server
cargo run -p settingsserver
```

The server will start on port 47007 and initialize with default monitoring settings.

## Integration

The Settings Server is designed to integrate with:

- **Web GUI Tools**: Primary consumers of the REST APIs
- **Monitoring Server**: Settings influence monitoring behavior
- **PULLPIRI Infrastructure**: Part of the overall system architecture

For production deployments, the Settings Server should be deployed alongside other PULLPIRI components and configured according to the system requirements.