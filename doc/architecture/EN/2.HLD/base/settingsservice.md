# Settings Service Detailed Design Document

**Document Number**: PICCOLO-SETTINGS-2025-001  
**Version**: 1.0  
**Date**: 2025-08-05  
**Author**: PICCOLO Team  
**Category**: HLD (High-Level Design)

## 1. Overview

The Settings Service is a core component for system configuration management in the PICCOLO framework. It provides various interfaces for administrators and developers to easily manage system configuration, including web-based UI and CLI for YAML-based configuration file creation, editing, verification, and deployment. It also supports configuration change history management and rollback features.

### 1.1 Purpose

The main purposes of the Settings Service are as follows:

1. Provide centralized configuration management for various components in the PICCOLO framework
2. Enable easy and safe application of configuration changes through multiple interfaces
3. Ensure system stability through change history management and rollback
4. Restrict configuration access based on user permissions
5. Support unified configuration creation using templates

### 1.2 Main Features

The Settings Service provides the following main features:

1. **Configuration Management**
   - Create, edit, and delete YAML-based configuration files
   - Validate configuration and analyze impact
   - Apply changes and monitor results

2. **Multiple Interfaces**
   - Web-based UI
   - Powerful CLI interface
   - REST API and gRPC interface

3. **Change Management**
   - Save and view change history
   - Support rollback to previous versions
   - Change verification and collision prevention

4. **Monitoring Information**
   - View PICCOLO resource status information stored in ETCD
   - Visualize resource-specific monitoring data
   - Export monitoring data

## 2. Architecture

The Settings Service is designed with a modular structure that clearly separates each functional area. This improves maintainability and scalability, allowing for independent development and testing of individual components.

### 2.1 Main Components

The main components of the Settings Service are as follows:

1. **Core Manager**
   - Service initialization and component coordination
   - Process configuration change requests and control flow
   - Manage interactions between other modules

2. **Configuration Manager**
   - Load and save configuration files
   - Configuration validation
   - Change impact analysis

3. **History Manager**
   - Configuration change history management
   - Version comparison
   - Rollback functionality

4. **Monitoring Viewer**
   - Retrieve monitoring data from ETCD
   - Process resource-specific monitoring data
   - Visualize monitoring information

5. **Web Interface**
   - Web server and API
   - User interface components
   - Real-time configuration editor

6. **CLI Interface**
   - Command-line command processing
   - Interactive shell
   - Batch processing

7. **External Integration**
   - API Server integration
   - ETCD storage integration
   - External system integration

### 2.2 Data Flow

The main data flows in the Settings Service are as follows:

1. **Configuration Change Flow**
   - User requests configuration change via Web UI or CLI
   - Configuration validation and impact analysis
   - Save changes to ETCD and record history
   - Forward changes to API Server
   - Monitor change application results and provide feedback

2. **Rollback Flow**
   - User requests rollback to previous version
   - Load previous version configuration
   - Create and validate rollback plan
   - Execute rollback and verify results
   - Record rollback history

3. **Monitoring Data Retrieval Flow**
   - User requests monitoring information
   - Retrieve resource status information from ETCD
   - Process and aggregate monitoring data
   - Provide data visualization or export in requested format

### 2.3 System Integration

The Settings Service integrates with other PICCOLO framework components as follows:

1. **API Server**
   - Forward changed configuration
   - Check configuration application status
   - Synchronize system-wide configuration

2. **State Manager**
   - Check system state after configuration changes
   - Monitor component status
   - Make rollback decisions based on state

3. **ETCD**
   - Store configuration data
   - Manage change history
   - Configuration version management
   - Retrieve monitoring data

4. **Monitoring Server**
   - Retrieve monitoring data
   - Collect system status information

## 3. Interfaces

The Settings Service provides various user and system interfaces.

### 3.1 User Interfaces

#### 3.1.1 Web UI

The web-based user interface provides the following features:

1. **Dashboard**
   - System configuration overview
   - Recent changes
   - Status monitoring
   - Monitoring metrics visualization

2. **Configuration Editor**
   - Hierarchical resource browsing
   - YAML/JSON editor
   - Schema-based validation
   - Property-based form editing

3. **History Browser**
   - View configuration change history
   - Compare versions
   - Rollback functionality

4. **Monitoring Viewer**
   - View PICCOLO resource status information collected from ETCD
   - Visualize resource-specific monitoring data
   - Filter and export monitoring data

#### 3.1.2 CLI

The command-line interface provides the following features:

1. **Basic Commands**
   - Get/set/delete configuration
   - Apply configuration
   - Validate configuration

2. **History Management**
   - View change history
   - Compare versions
   - Rollback to previous versions

3. **Interactive Shell**
   - Auto-completion
   - Command history
   - Inline help

4. **Batch Processing**
   - Script execution
   - Pipeline integration
   - Automation support

5. **Monitoring Queries**
   - Retrieve resource status information
   - Filter monitoring data
   - View status summaries
   - Automation support

5. **Monitoring Queries**
   - Retrieve resource status information
   - Filter monitoring data
   - View status summaries

### 3.2 System Interfaces

#### 3.2.1 REST API

The REST API provides the following endpoints:

1. **Configuration Management**
   - `GET /api/config/{key}` - Get configuration
   - `PUT /api/config/{key}` - Change configuration
   - `DELETE /api/config/{key}` - Delete configuration
   - `POST /api/config/apply` - Apply configuration
   - `POST /api/config/validate` - Validate configuration

2. **History Management**
   - `GET /api/history` - View history
   - `GET /api/history/{id}` - View specific history entry
   - `POST /api/rollback` - Execute rollback

3. **Monitoring Information**
   - `GET /api/monitoring/resources` - List of monitorable resources
   - `GET /api/monitoring/data/{resource}` - Retrieve resource-specific monitoring data
   - `GET /api/monitoring/status` - Retrieve system-wide status

#### 3.2.2 gRPC API

The gRPC API provides the following services:

1. **ConfigService**
   - `GetConfig` - Get configuration
   - `SetConfig` - Change configuration
   - `ApplyConfig` - Apply configuration
   - `ValidateConfig` - Validate configuration

2. **HistoryService**
   - `ListHistory` - View history
   - `GetHistoryEntry` - View specific history entry
   - `RollbackConfig` - Execute rollback

3. **MonitoringService**
   - `ListMonitoringResources` - List of monitorable resources
   - `GetResourceStatus` - Retrieve resource status
   - `GetMonitoringData` - Retrieve monitoring data

## 4. Data Models

The main data models used in the Settings Service are as follows.

### 4.1 Configuration Data

### 3.2 System Interfaces

#### 3.2.1 REST API

The REST API provides the following endpoints:

1. **Configuration Management**
   - `GET /api/config/{key}` - Get configuration
   - `PUT /api/config/{key}` - Change configuration
   - `DELETE /api/config/{key}` - Delete configuration
   - `POST /api/config/apply` - Apply configuration
   - `POST /api/config/validate` - Validate configuration

2. **History Management**
   - `GET /api/history` - View history
   - `GET /api/history/{id}` - View specific history entry
   - `POST /api/rollback` - Execute rollback

3. **Monitoring Information**
   - `GET /api/monitoring/resources` - List of monitorable resources
   - `GET /api/monitoring/data/{resource}` - Retrieve resource-specific monitoring data
   - `GET /api/monitoring/status` - Retrieve system-wide status

#### 3.2.2 gRPC API

The gRPC API provides the following services:

1. **ConfigService**
   - `GetConfig` - Get configuration
   - `SetConfig` - Change configuration
   - `ApplyConfig` - Apply configuration
   - `ValidateConfig` - Validate configuration

2. **HistoryService**
   - `ListHistory` - View history
   - `GetHistoryEntry` - View specific history entry
   - `RollbackConfig` - Execute rollback

3. **MonitoringService**
   - `ListMonitoringResources` - List of monitorable resources
   - `GetResourceStatus` - Retrieve resource status
   - `GetMonitoringData` - Retrieve monitoring data

## 4. Data Models

The main data models used in the Settings Service are as follows.

### 4.1 Configuration Data

```yaml
# Configuration data example
apiVersion: piccolo.io/v1
kind: Config
metadata:
  name: example-config
  namespace: system
spec:
  network:
    mtu: 1500
    ipv4:
      enabled: true
      dhcp: false
      address: "192.168.1.100/24"
      gateway: "192.168.1.1"
    ipv6:
      enabled: false
  storage:
    quota: "10GB"
    path: "/data"
  resources:
    cpu:
      limit: "2"
      request: "1"
    memory:
      limit: "4Gi"
      request: "2Gi"
```

### 4.3 Monitoring Information

```yaml
# Resource Status Information
resource_type: "container"
resource_id: "container-123456"
status: "running"
metrics:
  cpu_usage: 45.2
  memory_usage: 256.5
  disk_io: 10.3
  network_io: 5.2
last_updated: "2023-11-01T15:30:00Z"
events:
  - timestamp: "2023-11-01T15:25:00Z"
    type: "status_change"
    description: "Container started"
  - timestamp: "2023-11-01T15:20:00Z"
    type: "resource_update"
    description: "Resource configuration updated"
```

### 4.4 System Status Information

```yaml
# System status information example
id: system-status-123456
timestamp: "2025-08-05T14:00:00Z"
components:
  - name: "API Server"
    status: "Running"
    healthCheck: "Passed"
    uptime: "5d 12h 45m"
    version: "v1.2.3"
  - name: "State Manager"
    status: "Running"
    healthCheck: "Passed"
    uptime: "5d 12h 40m"
    version: "v1.2.1"
  - name: "ETCD"
    status: "Running"
    healthCheck: "Passed"
    uptime: "10d 8h 15m"
    version: "v3.5.2"
resourceSummary:
  nodes:
    total: 5
    healthy: 5
    unhealthy: 0
  models:
    total: 42
    running: 38
    failed: 1
    pending: 3
  networks:
    total: 8
    healthy: 8
    degraded: 0
alerts:
  critical: 0
  warning: 2
  info: 5
```

## 5. Detailed Functionality

### 5.1 Configuration Management

#### 5.1.1 Configuration Validation

Configuration validation is performed in the following steps:

1. **Syntax Validation**: Check for YAML/JSON syntax errors
2. **Schema Validation**: Validate against defined schema
3. **Semantic Validation**: Validate semantic validity of configuration values
4. **Dependency Validation**: Check dependencies with other configurations
5. **Impact Analysis**: Analyze impact of changes on the system

#### 5.1.2 Configuration Application

Configuration application is performed in the following steps:

1. **Pre-validation**: Validate before application
2. **Change Planning**: Plan changes to be applied
3. **Backup**: Backup current configuration
4. **Application**: Apply changes through API Server
5. **Monitoring**: Monitor application results
6. **Confirm/Rollback**: Confirm success or rollback if issues occur

### 5.2 History Management

#### 5.2.1 Change History Tracking

All configuration changes are recorded with the following information:

1. **Time Information**: Change date and time
2. **User Information**: User who performed the change
3. **Change Content**: Specific changes made
4. **Version Information**: Before and after versions
5. **Change Reason**: User-provided explanation

#### 5.2.2 Rollback Functionality

The rollback functionality is performed in the following process:
  - history.rollback
lastLogin: "2025-08-05T10:15:00Z"
status: ACTIVE
```

## 5. Detailed Functionality

### 5.1 Configuration Management

#### 5.1.1 Configuration Validation

Configuration validation is performed in the following steps:

1. **Syntax Validation**: Check for YAML/JSON syntax errors
2. **Schema Validation**: Validate against defined schema
3. **Semantic Validation**: Validate semantic validity of configuration values
4. **Dependency Validation**: Check dependencies with other configurations
5. **Impact Analysis**: Analyze impact of changes on the system

#### 5.1.2 Configuration Application

Configuration application is performed in the following steps:

1. **Pre-validation**: Validate before application
2. **Change Planning**: Plan changes to be applied
3. **Backup**: Backup current configuration
4. **Application**: Apply changes through API Server
5. **Monitoring**: Monitor application results
6. **Confirm/Rollback**: Confirm success or rollback if issues occur

### 5.2 History Management

#### 5.2.1 Change History Tracking

All configuration changes are recorded with the following information:

1. **Time Information**: Change date and time
2. **User Information**: User who performed the change
3. **Change Content**: Specific changes made
4. **Version Information**: Before and after versions
5. **Change Reason**: User-provided explanation

#### 5.2.2 Rollback Functionality

The rollback functionality is performed in the following process:

1. **Target Version Selection**: Select previous version to roll back to
2. **Rollback Plan Creation**: Create change plan from current state to target state
3. **Impact Analysis**: Analyze impact of rollback on the system
4. **Rollback Execution**: Perform rollback according to plan
5. **Result Verification**: Verify rollback success
6. **History Recording**: Record rollback operation in history

### 5.3 Monitoring Information Retrieval

#### 5.3.1 ETCD-based Monitoring Data Retrieval

The process of retrieving monitoring data from ETCD is as follows:

1. **Resource Information Retrieval**: Retrieve resource list and status information stored in ETCD
2. **Data Processing**: Process raw data into user-friendly format
3. **Status Aggregation**: Aggregate and summarize status information by resource type
4. **Event Correlation**: Display with related event information
5. **Visualization Preparation**: Prepare data for charts, graphs, and other visualizations

#### 5.3.2 Monitoring Dashboard

The monitoring dashboard provides the following information:

1. **System Overview**: Overall system status and key metrics summary
2. **Resource-specific Status**: Status and usage by resource type
3. **Real-time Metrics**: Real-time display of key metrics such as CPU, memory, disk, network
4. **Event Timeline**: Timeline of key events and status changes
5. **Resource Relationship Diagram**: Visualization of resource relationships and dependencies


5. **User Status Management**: Manage active/inactive status

## 6. Security Considerations

The Settings Service is designed with the following security aspects in mind:

1. **Authentication and Authorization**
   - Strong authentication system
   - Granular permission model
   - Principle of least privilege

2. **Data Protection**
   - Encryption of sensitive configuration information
   - Protection of data in transit (TLS)
   - Protection of stored data

3. **Audit and Tracking**
   - Record all changes
   - Log user activities
   - Protect audit logs

4. **Input Validation**
   - Validate all user inputs
   - Prevent malicious inputs
   - Safe parsing and processing

## 6. Security Considerations

The Settings Service implements the following security measures:

1. **Data Protection**
   - Encryption of sensitive data
   - Secure configuration storage
   - Protection against data leakage

2. **Access Control**
   - Clear separation of administrative and operational functions
   - Principle of least privilege
   - Role-based access control

3. **Secure Communication**
   - TLS/SSL encryption for all communications
   - Certificate validation
   - Secure API endpoints

4. **Audit and Compliance**
   - Comprehensive audit trails
   - Activity logging
   - Compliance with security standards

5. **API Security**
   - Restrict API access
   - Rate limiting
   - Token-based authentication

## 7. Performance Considerations

The Settings Service is designed with the following performance aspects in mind:

1. **Responsiveness**
   - Optimize user interface response time
   - Improve responsiveness through asynchronous processing
   - Provide user feedback

2. **Scalability**
   - Support distributed architecture
   - Handle large configuration datasets
   - Support simultaneous access by multiple users

3. **Resource Usage**
   - Efficient memory management
   - Optimize CPU usage
   - Storage space efficiency
   - Caching

## 8. Implementation Considerations

The following considerations should be taken into account when implementing the Settings Service:

1. **Technology Stack**
   - Rust language
   - Actix Web framework
   - gRPC communication
   - ETCD storage

2. **Modularity**
   - Separate modules by functionality
   - Clear interface definitions
   - Independent testing of modules

3. **Testability**
   - Structure conducive to unit testing
   - Support for integration testing
   - Use of mock objects

4. **Documentation**
   - In-code documentation
   - API documentation
   - User guide

5. **Internationalization**
   - Multi-language support
   - Localization settings
   - Cultural expression considerations

## 9. Deployment and Operations

Deployment and operational considerations for the Settings Service are as follows:

1. **Containerization**
   - Provide Docker image
   - Support Kubernetes deployment
   - Configurable environment variables

2. **Monitoring**
   - Status monitoring
   - Performance metrics collection
   - Log aggregation

3. **Backup and Recovery**
   - Regular configuration data backup
   - Failure recovery procedures
   - Data consistency maintenance

4. **Upgrades**
   - Support for zero-downtime upgrades
   - Compatibility with previous versions
   - Rollback plans

5. **Operational Documentation**
   - Installation guide
   - Operations manual
   - Troubleshooting guide

## 10. Conclusion

The Settings Service is a core component of the PICCOLO framework, providing various interfaces and functions for efficient and safe management of system configurations. It enhances system stability and user productivity by offering intuitive user experiences through Web UI and CLI, along with powerful validation, history management, and monitoring management capabilities.

This design document provides an overview of the basic architecture and main features of the Settings Service, with details that may be added or adjusted during the actual implementation process. Ultimately, the Settings Service supports PICCOLO framework components to be configured, managed, and monitored in a consistent and reliable manner.

### 4.2 History Entry

```yaml
# History entry example
id: history-123456
timestamp: "2025-08-05T13:30:00Z"
changeType: "configuration_update"
resource:
  type: "config"
  name: "system-config"
changes:
  - path: "networking.mtu"
    oldValue: 1500
    newValue: 9000
  - path: "storage.quota"
    oldValue: 10GB
    newValue: 20GB
description: "Updated network MTU and storage quota"
  labels:
    category: network
    environment: production
spec:
  # Configuration details
  parameter1: value1
  parameter2: value2
  nestedConfig:
    subParam1: subValue1
    subParam2: subValue2
  arrayConfig:
    - item1
    - item2
    - item3
```

### 4.2 History Entry

```yaml
# History entry example
id: history-123456
timestamp: "2025-08-05T14:30:00Z"
user: admin
action: UPDATE
configKey: example-config
version: "v1.2.3"
changes:
  - path: spec.parameter1
    oldValue: oldValue1
    newValue: value1
  - path: spec.nestedConfig.subParam2
    oldValue: oldSubValue2
    newValue: subValue2
comment: "Updated network parameters"
status: APPLIED
```

### 4.3 Monitoring Data

```yaml
# Monitoring data example
resourceType: "node"
resourceId: "node-01"
timestamp: "2025-08-05T13:45:00Z"
metrics:
  cpu_usage: 65.3
  memory_usage: 78.2
  disk_usage: 45.8
  network:
    rx_bytes: 1250000
    tx_bytes: 850000
status:
  condition: "Ready"
  lastHeartbeat: "2025-08-05T13:44:55Z"
  message: "Node operating normally"
events:
  - timestamp: "2025-08-05T13:30:12Z"
    type: "Warning"
    reason: "HighCPU"
    message: "CPU usage exceeded warning threshold"
  - timestamp: "2025-08-05T12:45:30Z"
    type: "Normal"
    reason: "SystemStartup"
    message: "Node started successfully"
```
```
