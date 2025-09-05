use common::nodeagent::node_agent_connection_server::NodeAgentConnection;
use common::nodeagent::{
    ConfigRequest, ConfigResponse, HandleYamlRequest, HandleYamlResponse, HeartbeatRequest,
    HeartbeatResponse, NodeRegistrationRequest, NodeRegistrationResponse, StatusAck, StatusReport,
};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// NodeAgent gRPC service handler
#[derive(Clone)]
pub struct NodeAgentReceiver {
    pub tx: mpsc::Sender<HandleYamlRequest>,
    // Add node information for clustering
    pub node_id: String,
    pub hostname: String,
    pub ip_address: String,
}

impl NodeAgentReceiver {
    pub fn new(
        tx: mpsc::Sender<HandleYamlRequest>,
        node_id: String,
        hostname: String,
        ip_address: String,
    ) -> Self {
        Self {
            tx,
            node_id,
            hostname,
            ip_address,
        }
    }
}

#[tonic::async_trait]
impl NodeAgentConnection for NodeAgentReceiver {
    /// Handle a yaml request from API-Server
    ///
    /// Receives a yaml from API-Server and forwards it to the NodeAgent manager for processing.
    async fn handle_yaml(
        &self,
        request: Request<HandleYamlRequest>,
    ) -> Result<Response<HandleYamlResponse>, Status> {
        println!("Got a Yamlrequest from api-server");
        let req: HandleYamlRequest = request.into_inner();

        match self.tx.send(req).await {
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
    async fn register_node(
        &self,
        request: Request<NodeRegistrationRequest>,
    ) -> Result<Response<NodeRegistrationResponse>, Status> {
        println!("Processing RegisterNode request");
        let _req = request.into_inner();

        // TODO: Implement node registration logic
        // This is typically called by the master node, not the node itself
        let response = NodeRegistrationResponse {
            success: true,
            message: "Node registration processed".to_string(),
            cluster_token: "node-token".to_string(),
            cluster_config: Some(common::nodeagent::ClusterConfig {
                master_endpoint: "http://localhost:47098".to_string(),
                heartbeat_interval: 30,
                settings: std::collections::HashMap::new(),
            }),
        };

        Ok(Response::new(response))
    }

    /// Report status to the API server
    async fn report_status(
        &self,
        request: Request<StatusReport>,
    ) -> Result<Response<StatusAck>, Status> {
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
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        println!("Processing Heartbeat request");
        let req = request.into_inner();

        // TODO: Process heartbeat and update last seen time
        println!("Heartbeat from node: {} at {}", req.node_id, req.timestamp);

        let response = HeartbeatResponse {
            ack: true,
            updated_config: Some(common::nodeagent::ClusterConfig {
                master_endpoint: "http://localhost:47098".to_string(),
                heartbeat_interval: 30,
                settings: std::collections::HashMap::new(),
            }),
        };

        Ok(Response::new(response))
    }

    /// Receive configuration updates from API server
    async fn receive_config(
        &self,
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
}

#[cfg(test)]
mod tests {
    use crate::grpc::receiver::{NodeAgentConnection, NodeAgentReceiver};
    use common::nodeagent::{HandleYamlRequest, HandleYamlResponse};
    use tokio::sync::mpsc;
    use tonic::{Request, Status};

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
        let receiver = NodeAgentReceiver { tx };

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
        let receiver = NodeAgentReceiver { tx };

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
}
