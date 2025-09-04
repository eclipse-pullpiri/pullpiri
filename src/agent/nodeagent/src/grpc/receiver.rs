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
    pub node_id: String,
}

impl NodeAgentReceiver {
    pub fn new(tx: mpsc::Sender<HandleYamlRequest>, node_id: String) -> Self {
        Self { tx, node_id }
    }
}

#[tonic::async_trait]
impl NodeAgentConnection for NodeAgentReceiver {
    /// Handle a yaml request from API-Server
    ///
    /// Receives a yaml from API-Server and forwards it to the NodeAgent manager for processing.
    async fn handle_yaml<'life>(
        &'life self,
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

    /// Register this node with the cluster
    async fn register_node<'life>(
        &'life self,
        request: Request<NodeRegistrationRequest>,
    ) -> Result<Response<NodeRegistrationResponse>, Status> {
        println!("Received node registration request");
        let req = request.into_inner();

        // For now, return a simple success response
        // In a full implementation, this would connect to the master API server
        Ok(Response::new(NodeRegistrationResponse {
            node_id: req.node_id,
            cluster_info: None,
            status: common::nodeagent::NodeStatus::Ready as i32,
            success: true,
            error_message: None,
        }))
    }

    /// Report status to the cluster
    async fn report_status<'life>(
        &'life self,
        request: Request<StatusReport>,
    ) -> Result<Response<StatusAck>, Status> {
        println!(
            "Received status report request for node: {}",
            request.get_ref().node_id
        );

        Ok(Response::new(StatusAck {
            acknowledged: true,
            message: Some("Status report received".to_string()),
        }))
    }

    /// Handle heartbeat from master
    async fn heartbeat<'life>(
        &'life self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        println!("Received heartbeat for node: {}", req.node_id);

        Ok(Response::new(HeartbeatResponse {
            alive: true,
            server_timestamp: chrono::Utc::now().timestamp(),
        }))
    }

    /// Receive configuration from master
    async fn receive_config<'life>(
        &'life self,
        request: Request<ConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();
        println!("Received config for node: {}", req.node_id);

        Ok(Response::new(ConfigResponse {
            success: true,
            error_message: None,
        }))
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
        let receiver = NodeAgentReceiver::new(tx, "test-node".to_string());

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
        let receiver = NodeAgentReceiver::new(tx, "test-node".to_string());

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
