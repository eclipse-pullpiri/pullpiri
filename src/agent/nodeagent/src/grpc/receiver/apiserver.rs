/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::nodeagent::fromapiserver::{
    ConfigRequest, ConfigResponse, HandleYamlRequest, HandleYamlResponse, HeartbeatRequest,
    HeartbeatResponse, NodeRegistrationRequest, NodeRegistrationResponse, StatusAck, StatusReport,
};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// Handle a yaml request from API-Server
///
/// Receives a yaml from API-Server and forwards it to the NodeAgent manager for processing.
pub async fn handle_yaml(
    tx: mpsc::Sender<HandleYamlRequest>,
    request: Request<HandleYamlRequest>,
) -> Result<Response<HandleYamlResponse>, Status> {
    println!("Got a Yamlrequest from api-server");
    let req: HandleYamlRequest = request.into_inner();

    match tx.send(req).await {
        Ok(_) => Ok(tonic::Response::new(HandleYamlResponse {
            status: true,
            desc: "Successfully processed YAML".to_string(),
        })),
        Err(e) => Err(tonic::Status::new(
            tonic::Code::Unavailable,
            format!("cannot send condition: {}", e),
        )),
    }
}

/// Register this node with the API server
pub async fn register_node(
    request: Request<NodeRegistrationRequest>,
) -> Result<Response<NodeRegistrationResponse>, Status> {
    println!("Processing RegisterNode request");
    let _req = request.into_inner();

    // TODO: Implement node registration logic
    // This is typically called by the master node, not the node itself
    let config = crate::config::Config::get();
    let master_ip = config.nodeagent.master_ip.clone();

    let response = NodeRegistrationResponse {
        success: true,
        message: "Node registration processed".to_string(),
        cluster_token: "node-token".to_string(),
        cluster_config: Some(common::nodeagent::fromapiserver::ClusterConfig {
            master_endpoint: format!("http://{}:47098", master_ip),
            heartbeat_interval: 30,
            settings: std::collections::HashMap::new(),
        }),
    };

    Ok(Response::new(response))
}

/// Report status to the API server
pub async fn report_status(request: Request<StatusReport>) -> Result<Response<StatusAck>, Status> {
    println!("Processing StatusReport request");
    let req = request.into_inner();

    // TODO: Process status report and update local state
    println!("Received status from node: {}", req.node_id);

    let response = StatusAck {
        received: true,
        message: "Status report received".to_string(),
    };

    Ok(Response::new(response))
}

/// Process heartbeat from API server
pub async fn heartbeat(
    request: Request<HeartbeatRequest>,
) -> Result<Response<HeartbeatResponse>, Status> {
    println!("Processing Heartbeat request");
    let req = request.into_inner();

    // TODO: Process heartbeat and update last seen time
    println!("Heartbeat from node: {} at {}", req.node_id, req.timestamp);

    let config = crate::config::Config::get();
    let master_ip = config.nodeagent.master_ip.clone();

    let response = HeartbeatResponse {
        ack: true,
        updated_config: Some(common::nodeagent::fromapiserver::ClusterConfig {
            master_endpoint: format!("http://{}:47098", master_ip),
            heartbeat_interval: 30,
            settings: std::collections::HashMap::new(),
        }),
    };

    Ok(Response::new(response))
}

/// Receive configuration updates from API server
pub async fn receive_config(
    request: Request<ConfigRequest>,
) -> Result<Response<ConfigResponse>, Status> {
    println!("Processing ReceiveConfig request");
    let req = request.into_inner();

    // TODO: Apply configuration changes
    println!("Received config with {} settings", req.config.len());

    let response = ConfigResponse {
        applied: true,
        message: "Configuration applied successfully".to_string(),
    };

    Ok(Response::new(response))
}

#[cfg(test)]
mod tests {
    use crate::grpc::receiver::{NodeAgentConnection, NodeAgentReceiver};
    use common::nodeagent::fromapiserver::{
        ConfigRequest, HandleYamlRequest, HeartbeatRequest, NodeRegistrationRequest, StatusReport,
    };
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::sync::Mutex;
    use tonic::Request;

    const VALID_ARTIFACT_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: hellow
spec:
  condition:
  action: update
  target: hellow
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: hellow
spec:
  pattern:
    - type: plain
  models:
    - name: hellow-core
      node: HPC
      resources:
        volume:
        network:
---
apiVersion: v1
kind: Model
metadata:
  name: hellow-core
  annotations:
    io.piccolo.annotations.package-type: hellow-core
    io.piccolo.annotations.package-name: hellow
    io.piccolo.annotations.package-network: default
  labels:
    app: hellow-core
spec:
  hostNetwork: true
  containers:
    - name: hellow
      image: hellow
  terminationGracePeriodSeconds: 0
"#;

    #[tokio::test]
    async fn test_handle_yaml_with_valid_artifact_yaml() {
        let (tx, mut rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
            Arc::new(Mutex::new(std::collections::HashMap::new())),
        );

        let request = HandleYamlRequest {
            yaml: VALID_ARTIFACT_YAML.to_string(),
            ..Default::default()
        };
        let tonic_request = Request::new(request.clone());

        let response = receiver.handle_yaml(tonic_request).await.unwrap();
        let response_inner = response.into_inner();

        assert!(response_inner.status);
        assert_eq!(response_inner.desc, "Successfully processed YAML");

        let received = rx.recv().await.unwrap();
        assert_eq!(received.yaml, request.yaml);
    }

    #[tokio::test]
    async fn test_handle_yaml_send_error() {
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
            Arc::new(Mutex::new(std::collections::HashMap::new())),
        );

        let request = HandleYamlRequest {
            yaml: VALID_ARTIFACT_YAML.to_string(),
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let result = receiver.handle_yaml(tonic_request).await;

        assert!(result.is_err());
        let status = result.err().unwrap();
        assert_eq!(status.code(), tonic::Code::Unavailable);
        assert!(status.message().starts_with("cannot send condition:"));
    }

    #[tokio::test]
    async fn test_register_node_success() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
            Arc::new(Mutex::new(std::collections::HashMap::new())),
        );

        let request = NodeRegistrationRequest {
            node_id: "test-node".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.100".to_string(),
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let response = receiver
            .register_node(tonic_request)
            .await
            .unwrap()
            .into_inner();
        assert!(response.success);
        assert_eq!(response.message, "Node registration processed");
        assert_eq!(response.cluster_token, "node-token");
        assert!(response.cluster_config.is_some());
        let config = response.cluster_config.unwrap();
        assert!(config.master_endpoint.contains("http://"));
        assert_eq!(config.heartbeat_interval, 30);
    }

    #[tokio::test]
    async fn test_report_status_success() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
            Arc::new(Mutex::new(std::collections::HashMap::new())),
        );

        let request = StatusReport {
            node_id: "test-node".to_string(),
            status: 1,
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let response = receiver
            .report_status(tonic_request)
            .await
            .unwrap()
            .into_inner();
        assert!(response.received);
        assert_eq!(response.message, "Status report received");
    }

    #[tokio::test]
    async fn test_heartbeat_success() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
            Arc::new(Mutex::new(std::collections::HashMap::new())),
        );

        let request = HeartbeatRequest {
            node_id: "test-node".to_string(),
            timestamp: 123456789,
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let response = receiver
            .heartbeat(tonic_request)
            .await
            .unwrap()
            .into_inner();
        assert!(response.ack);
        assert!(response.updated_config.is_some());
        let config = response.updated_config.unwrap();
        assert!(config.master_endpoint.contains("http://"));
        assert_eq!(config.heartbeat_interval, 30);
    }

    #[tokio::test]
    async fn test_receive_config_success() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = NodeAgentReceiver::new(
            tx,
            "test-node".to_string(),
            "test-host".to_string(),
            "192.168.1.100".to_string(),
            Arc::new(Mutex::new(std::collections::HashMap::new())),
        );

        let mut config_map = std::collections::HashMap::new();
        config_map.insert("key".to_string(), "value".to_string());

        let request = ConfigRequest {
            config: config_map,
            ..Default::default()
        };
        let tonic_request = Request::new(request);

        let response = receiver
            .receive_config(tonic_request)
            .await
            .unwrap()
            .into_inner();
        assert!(response.applied);
        assert_eq!(response.message, "Configuration applied successfully");
    }
}
