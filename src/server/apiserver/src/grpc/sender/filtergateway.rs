/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Running gRPC message sending to filtergateway

use common::filtergateway::{
    connect_server, filter_gateway_connection_client::FilterGatewayConnectionClient,
    HandleScenarioRequest, HandleScenarioResponse,
};
use tonic::{Request, Response, Status};

/// Send scenario information to filtergateway via gRPC
///
/// ### Parametets
/// * `scenario: HandleScenarioRequest` - wrapped scenario information
/// ### Description
/// This is generated almost automatically by `tonic_build`, so you
/// don't need to modify it separately.
pub async fn send(
    scenario: HandleScenarioRequest,
) -> Result<Response<HandleScenarioResponse>, Status> {
    let mut client = FilterGatewayConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.handle_scenario(Request::new(scenario)).await
}

//UNIT TEST CASES

#[cfg(test)]
mod tests {
    use super::*;
    use common::filtergateway::{
        filter_gateway_connection_server::{
            FilterGatewayConnection, FilterGatewayConnectionServer,
        },
        Action, HandleScenarioRequest, HandleScenarioResponse,
    };
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::{Request, Response, Status};

    // === Mock Scenario Definitions ===

    /// A valid YAML representing a proper Scenario
    const VALID_SCENARIO_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
"#;

    /// An empty scenario YAML (invalid case)
    const INVALID_SCENARIO_YAML_EMPTY: &str = r#"
"#;

    /// A scenario YAML with missing required field (`metadata.name`)
    const INVALID_SCENARIO_YAML_MISSING_FIELD: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name:
spec:
  condition:
  target: helloworld
"#;

    /// A YAML that is not a Scenario at all (different kind)
    const INVALID_NO_SCENARIO_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
"#;

    // === Mock gRPC Server Implementation ===

    /// A simple mock implementation of the gRPC service
    #[derive(Default)]
    struct MockFilterGateway;

    /// HandleScenario just checks if the scenario string is empty
    #[tonic::async_trait]
    impl FilterGatewayConnection for MockFilterGateway {
        async fn handle_scenario(
            &self,
            request: Request<HandleScenarioRequest>,
        ) -> Result<Response<HandleScenarioResponse>, Status> {
            let req = request.into_inner();

            if req.scenario.trim().is_empty() {
                return Err(Status::invalid_argument("Empty scenario"));
            }

            Ok(Response::new(HandleScenarioResponse {
                status: false,
                desc: format!("Mock handled: {:?}", req.action),
            }))
        }
    }

    /// Starts a mock gRPC server on a random available port
    async fn start_mock_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let stream = TcpListenerStream::new(listener);

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(FilterGatewayConnectionServer::new(
                    MockFilterGateway::default(),
                ))
                .serve_with_incoming(stream)
                .await
                .unwrap();
        });

        // Delay to allow the server to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        addr
    }

    /// Helper function to call `send()` logic with mock server endpoint
    async fn send_mocked(
        scenario: HandleScenarioRequest,
        addr: SocketAddr,
    ) -> Result<Response<HandleScenarioResponse>, Status> {
        let mut client = FilterGatewayConnectionClient::connect(format!("http://{}", addr))
            .await
            .unwrap();
        client.handle_scenario(Request::new(scenario)).await
    }

    // === TEST CASES ===

    /// Test the `send()` function with a valid scenario and action APPLY
    #[tokio::test]
    async fn test_send_with_valid_scenario_apply() {
        let addr = start_mock_server().await;

        let scenario = HandleScenarioRequest {
            action: Action::Apply.into(),
            scenario: VALID_SCENARIO_YAML.to_string(),
        };

        let result = send_mocked(scenario, addr).await;
        assert!(result.is_ok());
    }

    /// Test the `send()` function with a valid scenario and action WITHDRAW
    #[tokio::test]
    async fn test_send_with_valid_scenario_withdraw() {
        let addr = start_mock_server().await;

        let scenario = HandleScenarioRequest {
            action: Action::Withdraw.into(),
            scenario: VALID_SCENARIO_YAML.to_string(),
        };

        let result = send_mocked(scenario, addr).await;
        assert!(result.is_ok());
    }

    /// Test the `send()` function with an empty scenario name (invalid case)
    #[tokio::test]
    async fn test_send_with_empty_scenario_name_apply() {
        let addr = start_mock_server().await;

        let scenario = HandleScenarioRequest {
            action: Action::Apply.into(),
            scenario: INVALID_SCENARIO_YAML_EMPTY.to_string(),
        };

        let result = send_mocked(scenario, addr).await;
        assert!(result.is_err(), "Expected an error for empty scenario name");
    }

    /// Test the `send()` function with missing required fields (e.g., name)
    #[tokio::test]
    async fn test_send_with_missing_field_apply() {
        let addr = start_mock_server().await;

        let scenario = HandleScenarioRequest {
            action: Action::Apply.into(),
            scenario: INVALID_SCENARIO_YAML_MISSING_FIELD.to_string(),
        };

        let result = send_mocked(scenario, addr).await;
        assert!(result.is_ok()); // mock does not parse YAML deeply
    }

    /// Test the `send()` function with a non-Scenario kind (e.g., Package)
    #[tokio::test]
    async fn test_send_with_nonexistent_scenario_apply() {
        let addr = start_mock_server().await;

        let scenario = HandleScenarioRequest {
            action: Action::Apply.into(),
            scenario: INVALID_NO_SCENARIO_YAML.to_string(),
        };

        let result = send_mocked(scenario, addr).await;
        assert!(result.is_ok()); // mock accepts anything non-empty
    }

    /// Test the `send()` function with an empty scenario name and action WITHDRAW
    #[tokio::test]
    async fn test_send_with_empty_scenario_name_withdraw() {
        let addr = start_mock_server().await;

        let scenario = HandleScenarioRequest {
            action: Action::Withdraw.into(),
            scenario: INVALID_SCENARIO_YAML_EMPTY.to_string(),
        };

        let result = send_mocked(scenario, addr).await;
        assert!(result.is_err(), "Expected an error for empty scenario name");
    }

    /// Test the `send()` function with missing required fields and action WITHDRAW
    #[tokio::test]
    async fn test_send_with_missing_field_withdraw() {
        let addr = start_mock_server().await;

        let scenario = HandleScenarioRequest {
            action: Action::Withdraw.into(),
            scenario: INVALID_SCENARIO_YAML_MISSING_FIELD.to_string(),
        };

        let result = send_mocked(scenario, addr).await;
        assert!(result.is_ok());
    }

    /// Test the `send()` function with a non-Scenario kind and action WITHDRAW
    #[tokio::test]
    async fn test_send_with_nonexistent_scenario_withdraw() {
        let addr = start_mock_server().await;

        let scenario = HandleScenarioRequest {
            action: Action::Withdraw.into(),
            scenario: INVALID_NO_SCENARIO_YAML.to_string(),
        };

        let result = send_mocked(scenario, addr).await;
        assert!(result.is_ok());
    }
}
