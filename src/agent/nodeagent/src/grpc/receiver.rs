use common::nodeagent::node_agent_service_server::NodeAgentService;
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
}

#[tonic::async_trait]
impl NodeAgentService for NodeAgentReceiver {
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

    /// Report status to the API server
    ///
    /// Accepts status reports from this node
    async fn report_status(
        &self,
        request: Request<StatusReport>,
    ) -> Result<Response<StatusAck>, Status> {
        let _req = request.into_inner();

        // For now, just acknowledge the status report
        Ok(tonic::Response::new(StatusAck {
            acknowledged: true,
            message: "Status report received".to_string(),
        }))
    }

    /// Handle heartbeat request
    ///
    /// For NodeAgent, this method returns an error since heartbeat should be sent by the node itself
    async fn heartbeat(
        &self,
        _request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        Err(tonic::Status::new(
            tonic::Code::Unimplemented,
            "Heartbeat should be sent by the node, not received",
        ))
    }

    /// Receive configuration from the API server
    ///
    /// Accepts configuration updates from the master node
    async fn receive_config(
        &self,
        request: Request<ConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        let req = request.into_inner();

        // For now, just acknowledge the configuration
        println!("Configuration received for node: {}", req.node_id);

        Ok(tonic::Response::new(ConfigResponse {
            success: true,
            message: "Configuration received successfully".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::grpc::receiver::{NodeAgentReceiver, NodeAgentService};
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
        };
        let tonic_request = Request::new(request);

        let result = receiver.handle_yaml(tonic_request).await;

        assert!(result.is_err());
        let status = result.err().unwrap();
        assert_eq!(status.code(), tonic::Code::Unavailable);
        assert!(status.message().starts_with("cannot send condition:"));
    }
}
