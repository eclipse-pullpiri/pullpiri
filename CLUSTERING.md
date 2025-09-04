# PICCOLO Clustering System - Quick Start Guide

The PICCOLO Clustering System provides lightweight cluster management for resource-constrained environments.

## Key Features

✅ **Master-Sub Node Architecture** - Simple, efficient design  
✅ **REST API** - Complete cluster management via HTTP endpoints  
✅ **Automatic Registration** - Nodes self-register with resource detection  
✅ **Heartbeat Monitoring** - 30-second intervals with failure detection  
✅ **etcd State Storage** - Distributed, reliable state management  
✅ **Resource Tracking** - CPU, memory, and disk usage monitoring  
✅ **Network Resilience** - Automatic reconnection on failures  

## Quick Demo

### Prerequisites
- Rust toolchain installed 
- etcd running on localhost:2379

### Start etcd
```bash
etcd --data-dir /tmp/etcd-demo --listen-client-urls http://127.0.0.1:2379 --advertise-client-urls http://127.0.0.1:2379
```

### Start API Server
```bash
cd pullpiri
cargo run --manifest-path=src/server/apiserver/Cargo.toml
```

### Test Cluster API

**Check cluster health:**
```bash
curl http://localhost:47099/api/v1/cluster/health | jq
```

**Register a node:**
```bash
curl -X POST http://localhost:47099/api/v1/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "node_name": "worker-1",
    "ip_address": "192.168.1.20", 
    "role": "sub",
    "resources": {
      "cpu_cores": 4,
      "memory_mb": 8192,
      "disk_gb": 100
    }
  }' | jq
```

**List all nodes:**
```bash
curl http://localhost:47099/api/v1/nodes | jq
```

**Get cluster topology:**
```bash
curl http://localhost:47099/api/v1/topology | jq
```

## NodeAgent Integration

The NodeAgent now includes automatic clustering functionality:

- **Auto-registration** on startup with system resource detection
- **Background heartbeat** every 30 seconds  
- **Environment configuration** via `PICCOLO_MASTER_IP`, `PICCOLO_NODE_ROLE`
- **Resilient networking** with automatic reconnection

## Architecture

```
┌─────────────────┐    ┌─────────────────┐
│   Master Node   │    │   Sub Node      │
│                 │    │                 │  
│ ┌─────────────┐ │    │ ┌─────────────┐ │
│ │ API Server  │ │    │ │ NodeAgent   │ │
│ │ FilterGW    │ │◄──►│ │             │ │
│ │ ActionCtrl  │ │    │ │ Heartbeat   │ │
│ │ StateManager│ │    │ │ Registration│ │
│ └─────────────┘ │    │ └─────────────┘ │
│                 │    │                 │
│ ┌─────────────┐ │    └─────────────────┘
│ │    etcd     │ │           │
│ └─────────────┘ │           │
└─────────────────┘           │
         │                    │
         │   gRPC/REST API    │
         └────────────────────┘
```

## API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/cluster/health` | Cluster health status |
| GET | `/api/v1/nodes` | List all nodes |
| POST | `/api/v1/nodes` | Register new node |
| GET | `/api/v1/nodes/{id}` | Get specific node |
| POST | `/api/v1/nodes/{id}/status` | Update node status |
| DELETE | `/api/v1/nodes/{id}` | Remove node |
| GET | `/api/v1/topology` | Get cluster topology |

## Implementation Status

- [x] Proto definitions with clustering interfaces
- [x] Node registry with etcd backend
- [x] REST API for cluster management  
- [x] NodeAgent clustering integration
- [x] Heartbeat mechanism and failure detection
- [x] Cluster topology management
- [x] Resource monitoring and metrics
- [x] Network resilience and reconnection
- [x] Build and formatting validation

The clustering system is fully functional and ready for production use in embedded environments.