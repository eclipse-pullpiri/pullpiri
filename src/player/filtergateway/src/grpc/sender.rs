use common::Result;

// Import the generated protobuf code from actioncontroller.proto

use common::actioncontroller::connect_server;

// Import the generated protobuf code from actioncontroller.proto
use common::actioncontroller::action_controller_connection_client::ActionControllerConnectionClient;

/// Sender for making gRPC requests to ActionController
#[derive(Clone)]
pub struct FilterGatewaySender {}

impl FilterGatewaySender {
    /// Create a new FilterGatewaySender
    ///
    /// # Returns
    ///
    /// A new FilterGatewaySender instance
    pub fn new() -> Self {
        Self {}
    }

    /// Trigger an action for a scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn trigger_action(&mut self, scenario_name: String) -> Result<()> {
        if scenario_name.trim().is_empty() {
            return Err("Invalid scenario name: cannot be empty".into());
        }
        use common::actioncontroller::TriggerActionRequest;
        let mut client = ActionControllerConnectionClient::connect(connect_server())
            .await
            .unwrap();

        let request = TriggerActionRequest { scenario_name };

        client.trigger_action(request).await.map_err(|e| {
            log::error!("Failed to trigger action: {:?}", e);
            anyhow::anyhow!("Failed to trigger action: {:?}", e)
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Error;
    use common::actioncontroller::{
        action_controller_connection_server::{
            ActionControllerConnection, ActionControllerConnectionServer,
        },
        ReconcileRequest, ReconcileResponse, TriggerActionRequest, TriggerActionResponse,
    };
    use std::net::SocketAddr;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use tokio::sync::oneshot;
    use tokio::time::{sleep, Duration};
    use tonic::{transport::Server, Request, Response, Status};

    #[derive(Default)]
    struct MockActionController;

    #[tonic::async_trait]
    impl ActionControllerConnection for MockActionController {
        async fn trigger_action(
            &self,
            request: Request<TriggerActionRequest>,
        ) -> std::result::Result<Response<TriggerActionResponse>, Status> {
            let scenario_name = request.into_inner().scenario_name;

            println!("Mock server received trigger_action: {:?}", scenario_name);

            if scenario_name.trim().is_empty() {
                // Return gRPC invalid argument error if scenario is empty
                println!("cannot: {:?}", scenario_name);
                return Err(Status::invalid_argument("Scenario name cannot be empty"));
            }
            Ok(Response::new(TriggerActionResponse {
                desc: "OK".to_string(),
                status: 0,
            }))
        }

        async fn reconcile(
            &self,
            _request: Request<ReconcileRequest>,
        ) -> std::result::Result<Response<ReconcileResponse>, Status> {
            Ok(Response::new(ReconcileResponse::default()))
        }
    }

    async fn spawn_mock_server(
        addr: SocketAddr,
    ) -> (oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let server_handle = tokio::spawn(async move {
            Server::builder()
                .add_service(ActionControllerConnectionServer::new(
                    MockActionController::default(),
                ))
                .serve_with_shutdown(addr, async {
                    shutdown_rx.await.ok();
                })
                .await
                .unwrap();
        });

        sleep(Duration::from_millis(200)).await;

        (shutdown_tx, server_handle)
    }

    #[tokio::test]
    async fn test_trigger_action_success() {
        let addr = "0.0.0.0:47001".parse().unwrap();

        // Try binding to the address to check if it's already in use
        let port_available = std::net::TcpListener::bind(addr).is_ok();

        let (shutdown_tx, server_handle): (
            Option<tokio::sync::oneshot::Sender<()>>,
            Option<tokio::task::JoinHandle<()>>,
        ) = if port_available {
            let (tx, handle) = spawn_mock_server(addr).await;
            (Some(tx), Some(handle))
        } else {
            (None, None)
        };
        // Insert mock Scenario YAML into etcd
        common::etcd::put(
            "Scenario/antipinch-enable",
            r#"
apiVersion: v1
kind: Scenario
metadata:
  name: antipinch-enable
spec:
  condition:
  action: update
  target: antipinch-enable
"#,
        )
        .await
        .unwrap();

        // Insert mock Package YAML into etcd
        common::etcd::put(
            "Package/antipinch-enable",
            r#"
apiVersion: v1
kind: Package
metadata:
  label: null
  name: antipinch-enable
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
"#,
        )
        .await
        .unwrap();
        let mut sender = FilterGatewaySender::new();
        let result = sender.trigger_action("antipinch-enable".to_string()).await;

        assert!(result.is_ok());
        if let Some(tx) = shutdown_tx {
            let _ = tx.send(());
        }
        if let Some(handle) = server_handle {
            let _ = handle.await;
        }
    }

    #[tokio::test]
    async fn test_trigger_action_empty_scenario_name_should_fail() {
        let addr = "0.0.0.0:47001".parse().unwrap();

        // Check port availability
        let port_available = std::net::TcpListener::bind(addr).is_ok();

        let (shutdown_tx, server_handle): (
            Option<tokio::sync::oneshot::Sender<()>>,
            Option<tokio::task::JoinHandle<()>>,
        ) = if port_available {
            let (tx, handle) = spawn_mock_server(addr).await;
            (Some(tx), Some(handle))
        } else {
            (None, None)
        };

        let mut sender = FilterGatewaySender::new();
        let result = sender.trigger_action("".to_string()).await;

        assert!(result.is_err());

        if let Some(tx) = shutdown_tx {
            let _ = tx.send(());
        }
        if let Some(handle) = server_handle {
            let _ = handle.await;
        }
    }

    /// Test case to validate failure due to connection error
    #[tokio::test]
    async fn test_trigger_action_failure_connection_error() {
        let mut sender = FilterGatewaySender::new();
        let result = sender.trigger_action("".to_string()).await;

        if let Err(e) = &result {
            println!("Expected failure occurred: {}", e);
        }

        assert!(result.is_err(), "Expected error but got success")
    }

    /// Test case to validate failure when `connect_server` returns an empty server address
    #[tokio::test]
    async fn test_trigger_action_failure_empty_server_address() {
        let mut sender = FilterGatewaySender::new();
        let scenario_name = "test_scenario".to_string();

        // Simulate empty server address
        let result = ActionControllerConnectionClient::connect("").await;

        // Assert that the connection attempt fails
        assert!(result.is_err());
    }
}
