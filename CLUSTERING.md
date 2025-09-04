# PICCOLO Clustering System Implementation

This implementation provides a lightweight clustering system for the PICCOLO framework, optimized for embedded environments.

## Overview

The clustering system enables communication and management between master nodes and sub nodes through a gRPC-based architecture.

### Key Components

1. **API Server (Master Node)**
   - Node registration and management
   - Cluster topology management  
   - Health monitoring and status tracking
   - gRPC services for cluster operations

2. **NodeAgent (Sub Node)**
   - Connection to master node
   - System resource reporting
   - Heartbeat mechanism
   - Status updates

## Architecture

```
┌─────────────────────┐     ┌─────────────────────┐
│   Master Node       │     │    Sub Node         │
│                     │     │                     │
│  ┌─────────────┐    │     │  ┌─────────────┐    │
│  │ API Server  │    │     │  │ NodeAgent   │    │
│  │             │    │     │  │             │    │
│  │ - Registry  │◄───┼─────┤  │ - Client    │    │
│  │ - Manager   │    │     │  │ - Monitor   │    │
│  │ - Status    │    │     │  │ - Report    │    │
│  └─────────────┘    │     │  └─────────────┘    │
└─────────────────────┘     └─────────────────────┘
```

## Protocol Messages

### Node Registration
```protobuf
service NodeAgentConnection {
  rpc RegisterNode(NodeRegistrationRequest) returns (NodeRegistrationResponse);
  rpc ReportStatus(StatusReport) returns (StatusAck);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
}

service ApiServerService {
  rpc GetNodes(GetNodesRequest) returns (GetNodesResponse);
  rpc RegisterNode(NodeRegistrationRequest) returns (NodeRegistrationResponse);
  rpc GetTopology(GetTopologyRequest) returns (GetTopologyResponse);
}
```

## Usage Example

### Master Node (API Server)
```rust
use apiserver::node::NodeRegistry;
use std::sync::Arc;

// Create node registry
let registry = Arc::new(NodeRegistry::new());

// Set up gRPC server
let receiver = ApiServerReceiver::new(registry.clone());
let service = ApiServerServiceServer::new(receiver);
```

### Sub Node (NodeAgent)
```rust
use nodeagent::cluster::ClusterClient;

// Create cluster client
let mut client = ClusterClient::new(
    "master-node:47098".to_string(),
    "worker-1".to_string(), 
    "192.168.1.10".to_string()
);

// Connect and register
client.connect().await?;
let response = client.register_node().await?;

// Start background monitoring
client.start_background_tasks().await;
```

## Features Implemented

✅ **Node Management**
- Node registration with authentication
- System resource collection
- Node status tracking

✅ **Communication**
- gRPC-based messaging
- Heartbeat mechanism  
- Status reporting

✅ **Cluster Operations**
- Topology management
- Health monitoring
- Node lifecycle management

✅ **Resource Monitoring**
- CPU and memory usage
- System information collection
- Metric reporting

## Configuration

The system uses the following default ports:
- API Server gRPC: 47098
- API Server REST: 47099
- NodeAgent: 47004

## Testing

Run the clustering tests with:
```bash
# API Server tests
cargo test --manifest-path=src/server/apiserver/Cargo.toml

# NodeAgent tests  
cargo test cluster --manifest-path=src/agent/nodeagent/Cargo.toml
```

## Next Steps

The core clustering functionality is complete. Future enhancements could include:
- Integration with main application workflows
- Advanced authentication mechanisms
- Multi-master support
- Web-based management interface