use common::nodeagent::node_agent_connection_server::NodeAgentConnection;
use common::nodeagent::{
    HandleYamlRequest, HandleYamlResponse, HeartbeatRequest, HeartbeatResponse,
    NodeRegistrationRequest, NodeRegistrationResponse, NodeStatusRequest, NodeStatusResponse,
};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// NodeAgent gRPC service handler
#[derive(Clone)]
pub struct NodeAgentReceiver {
    pub tx: mpsc::Sender<HandleYamlRequest>,
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

    /// Handle node registration request
    ///
    /// For NodeAgent, this method returns an error since registration should be initiated by the node itself
    async fn register_node(
        &self,
        _request: Request<NodeRegistrationRequest>,
    ) -> Result<Response<NodeRegistrationResponse>, Status> {
        Err(tonic::Status::new(
            tonic::Code::Unimplemented,
            "Node registration should be initiated by the node, not received",
        ))
    }

    /// Handle heartbeat request
    ///
    /// For NodeAgent, this method returns an error since heartbeat should be sent by the node itself
    async fn send_heartbeat(
        &self,
        _request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        Err(tonic::Status::new(
            tonic::Code::Unimplemented,
            "Heartbeat should be sent by the node, not received",
        ))
    }

    /// Handle node status request
    ///
    /// Returns the current status of this node
    async fn get_node_status(
        &self,
        request: Request<NodeStatusRequest>,
    ) -> Result<Response<NodeStatusResponse>, Status> {
        let req = request.into_inner();

        // For now, return a simple success response
        // In a full implementation, this would check the actual node status
        println!("Node status requested for: {}", req.node_id);

        Ok(tonic::Response::new(NodeStatusResponse {
            success: true,
            node: None, // Would contain actual node info in full implementation
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
