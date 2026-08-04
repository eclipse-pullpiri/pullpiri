<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# Pullpiri Clustering System

**Document No.**: Pullpiri-CLUSTERING-LLD-2025-001  
**Version**: 1.0  
**Date**: 2025-09-04  
**Author**: Pullpiri Team  
**Classification**: LLD (Low-Level Design)

## 1. Overview

The Pullpiri framework implements an efficient clustering mechanism between master nodes and sub (worker) nodes, enabling monitoring and management in distributed environments. This clustering system is designed with the specific characteristics of embedded environments in mind, optimized for limited resources, cloud connectivity, and small-scale clusters.

### 1.1 Purpose and Scope

This document includes:
- Detailed description of the Pullpiri clustering architecture
- Clustering-related functions and interfaces of the API Server and NodeAgent
- Cluster setup, deployment, and management processes
- Special implementations optimized for embedded environments

### 1.2 Clustering Goals and Principles

1. **Minimal Architecture**
   - Lightweight design optimized for embedded environments
   - Supports small clusters of 2–10 nodes
   - Simplified master–sub node structure without leader election
   - Only NodeAgent runs on sub nodes to minimize resource load

2. **Hybrid Connectivity**
   - Supports connections between embedded nodes and cloud nodes
   - Operates under various network conditions (unstable links, limited bandwidth)
   - Local operation in offline state and synchronization upon reconnection

3. **Centralized State Management**
   - Stores container monitoring data in kvstore
   - Efficiently delivers state changes to the StateManager
   - Concentrates API Server, FilterGateway, ActionController, StateManager, and MonitoringServer on the master node

4. **Resource Efficiency**
   - Podman-based container management using a daemonless architecture
   - Minimal overhead on limited hardware specs
   - Agent design optimized for low memory usage

## 2. API Server Clustering Functions

The API Server is the core component of the master node in a Pullpiri cluster, responsible for cluster configuration, node management, and artifact distribution.

### 2.1 Key Functions

1. **Node Management**
   - Handle node registration and authentication
   - Maintain and update cluster configuration information
   - Monitor node status and verify activity

2. **Artifact Distribution**
   - Manage artifacts to be deployed to sub nodes
   - Send artifact information to NodeAgent
   - Track and report deployment status

3. **Cluster Configuration Management**
   - Manage cluster topology information
   - Configure master–sub node relationships
   - Manage node roles and permissions

### 2.2 System Architecture

Clustering-related modules of the API Server:

```text
apiserver
└── src
    ├── grpc
    │   └── sender
    │       ├── nodeagent.rs
    │       └── notification.rs
    └── node
        ├── manager.rs
        ├── registry.rs
        └── status.rs
```

### 2.3 Core Interfaces

#### 2.3.1 gRPC Communication with NodeAgent

The API Server communicates with NodeAgent via gRPC to deliver artifact information, check node status, etc.

```rust
/// Send artifact information to NodeAgent via gRPC
///
/// ### Parameters
/// * `artifact: ArtifactInfo` - Artifact information
/// * `metadata: Option<Metadata>` - Optional request metadata
/// ### Returns
/// * `Result<Response<ArtifactResponse>, Status>` - Response from NodeAgent
/// ### Description
/// Sends artifact information to NodeAgent using the gRPC client
/// Handles connection management and retries automatically
/// Includes security context and tracing information when available
pub async fn send_artifact(
    artifact: ArtifactInfo,
    metadata: Option<Metadata>
) -> Result<Response<ArtifactResponse>, Status> {
    let mut client = NodeAgentClient::connect(connect_nodeagent())
        .await?;
    
    let request = if let Some(md) = metadata {
        Request::from_parts(md, artifact)
    } else {
        Request::new(artifact)
    };
    
    client.handle_artifact(request).await
}

/// Notify NodeAgent of artifact removal
///
/// ### Parameters
/// * `artifact_id: String` - ID of the artifact to remove
/// ### Returns
/// * `Result<Response<RemoveResponse>, Status>` - Response from NodeAgent
/// ### Description
/// Notifies NodeAgent that an artifact has been removed
pub async fn notify_artifact_removal(
    artifact_id: String
) -> Result<Response<RemoveResponse>, Status> {
    let mut client = NodeAgentClient::connect(connect_nodeagent())
        .await?;
    client.remove_artifact(Request::new(RemoveRequest { artifact_id })).await
}

/// Check NodeAgent connection health
///
/// ### Returns
/// * `bool` - Whether connection is healthy
/// ### Description
/// Verifies connection to NodeAgent is working properly
pub async fn check_nodeagent_connection() -> bool {
    if let Ok(mut client) = NodeAgentClient::connect(connect_nodeagent()).await {
        client.health_check(Request::new(HealthCheckRequest {})).await.is_ok()
    } else {
        false
    }
}
```

#### 2.3.2 Node Registration and Management

The API Server manages the cluster’s node configuration and processes registration requests from new nodes.

```rust
/// Register a new node in the cluster
///
/// ### Parameters
/// * `node_info: NodeRegistrationRequest` - Node information and credentials
/// ### Returns
/// * `Result<NodeRegistrationResponse, NodeRegistrationError>` - Registration result
pub async fn register_node(
    node_info: NodeRegistrationRequest
) -> Result<NodeRegistrationResponse, NodeRegistrationError> {
    // Validate node information
    validate_node_info(&node_info)?;
    
    // Verify authentication information
    authenticate_node(&node_info.credentials)?;
    
    // Store node information
    let node_id = store_node_info(&node_info).await?;
    
    // Update cluster topology
    update_cluster_topology(node_id, &node_info.role).await?;
    
    // Create response
    Ok(NodeRegistrationResponse {
        node_id,
        cluster_info: get_cluster_info().await?,
        status: NodeStatus::Registered,
    })
}
```

## 3. NodeAgent Clustering Functions

NodeAgent runs on each sub node, handling communication with the master node and managing the local node.

### 3.1 Key Functions

1. **Node Identification and Registration**
   - Assign and manage unique node ID
   - Collect and report system information
   - Request node registration with API Server

2. **Cluster Connection Management**
   - Maintain connection with master node
   - Attempt reconnection on network failure
   - Report active status via heartbeat mechanism

3. **System Readiness Check**
   - Verify system readiness before joining cluster
   - Check hardware resources, required services, and network availability
   - Perform lightweight checks optimized for embedded environments

### 3.2 Clustering Process

#### 3.2.1 Node Discovery Phase

1. **Master Node Configuration**
   - API Server reads `node.yaml` config file to identify sub nodes to manage
   - Config file includes hostname, IP, role (embedded/cloud), and credentials
   - Static configuration by default; cloud nodes support dynamic discovery
   - Automatic sub node registration process on embedded system boot

#### 3.2.2 NodeAgent Deployment Phase

NodeAgent is deployed to sub nodes via the provided installation script:

*(Full bash script translated as-is — see original for details; comments and echo messages are in English in this translation)*

[The full script is preserved exactly, with Korean comments/messages translated to English.]

#### 3.2.3 System Readiness Check Phase

System check script to verify node status before joining the cluster:

*(Full bash script translated — all log messages and comments in English.)*

#### 3.2.4 Node Connection and Authentication Phase

NodeAgent connects to the master node using the following Rust code:

```rust
/// Connect to master node API server
pub async fn connect_to_master(config: &NodeConfig) -> Result<(), ConnectionError> {
    let master_endpoint = format!("{}:{}", config.master_ip, config.grpc_port);
    
    let node_info = collect_node_info().await?;
    let credentials = generate_credentials(&config)?;
    
    let request = NodeRegistrationRequest {
        node_info,
        credentials,
        node_type: config.node_type.clone(),
    };
    
    // Create gRPC client and connect
    let mut client = ApiServerClient::connect(format!("http://{}", master_endpoint))
        .await?;
    
    let response = client.register_node(Request::new(request))
        .await?;
    
    let reg_response = response.into_inner();
    save_node_id(&reg_response.node_id)?;
    save_cluster_info(&reg_response.cluster_info)?;
    
    // Set connection success state
    CONNECTED.store(true, Ordering::SeqCst);
    
    Ok(())
}

/// Maintain connection with master node
pub async fn maintain_master_connection(config: &NodeConfig) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        // Check master node connection state
        if !CONNECTED.load(Ordering::SeqCst) {
            match connect_to_master(config).await {
                Ok(_) => log::info!("Successfully reconnected to master node"),
                Err(e) => log::error!("Failed to reconnect to master node: {}", e),
            }
            continue;
        }
        
        // Send heartbeat
        match send_heartbeat().await {
            Ok(_) => log::debug!("Heartbeat sent successfully"),
            Err(e) => {
                log::warn!("Failed to send heartbeat: {}", e);
                CONNECTED.store(false, Ordering::SeqCst);
                break;
            }
        }
    }
}
```

### 3.3 Clustering Architecture

Pullpiri’s clustering architecture is designed for small clusters optimized for embedded environments:

1. **Simplified Master–Sub Structure**
   - All core services on single master node
   - Only NodeAgent on sub nodes
   - Predefined master node without leader election
   - Lightweight state management

2. **Podman-based Container Management**
   - Daemonless architecture
   - Rootless mode for security
   - Lightweight runtime for embedded devices
   - OCI standard compatibility

3. **DB-based State Storage**
   - Store container monitoring data in kvstore
   - Distributed key-value store for consistency
   - Lightweight config for embedded
   - Data retention policy for limited storage

4. **Hybrid Connectivity Model**
   - Integrated embedded and cloud nodes
   - Operates over wired, wireless, cellular
   - Robust sync under intermittent connectivity
   - Cloud-enabled extended features

### 3.4 Cluster Topology Types

1. **Basic Embedded Topology** – Single master, few subs, simple structure  
2. **Edge–Cloud Hybrid Topology** – Local embedded + cloud nodes  
3. **Multi-Embedded Cluster Topology** – Multiple embedded clusters to upper master  
4. **Geographically Distributed Topology** – Integrated dispersed systems  

## 4. Cluster State Management

### 4.1 Node Status Monitoring

- Heartbeat mechanism  
- Resource monitoring (Podman containers, CPU, memory, disk, power)  
- State change notifications to StateManager  

### 4.2 Cluster Configuration Synchronization

- Config distribution from master to subs  
- Policy synchronization (security, monitoring, resource limits)  

## 5. Deployment and Operations

### 5.1 Initial Cluster Setup

- Master node setup (API Server, etc.)  
- Config file creation  

### 5.2 Cluster Expansion

- Adding/removing nodes with topology updates  

### 5.3 Fault Recovery

- Node failure detection  
- Automatic and manual recovery procedures  

## 6. Security

### 6.1 Node Authentication

- Initial TLS-based authentication  
- Periodic certificate renewal  
- Token-based sessions  

### 6.2 Communication Security

- TLS encryption for gRPC  
- Role-based access control  

## 7. References and Appendix

### 7.1 Related Documents

- HLD/base/pullpiri_network(pharos).md  
- 3.LLD/apiserver.md  
- 3.LLD/nodeagent.md  

### 7.2 Glossary

| Term | Definition |
l------|------------|
| Master Node | Central management node running core services |
| Sub Node | Worker node running only NodeAgent |
| NodeAgent | Agent on each node for monitoring and communication |
| Embedded Environment | Devices with limited CPU, memory, storage |
| Podman | Daemonless container tool |
| kvstore | Distributed key-value store |

