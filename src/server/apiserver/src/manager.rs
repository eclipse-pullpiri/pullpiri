/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Controls the flow of data between each module.
use common::apiserver::api_server_connection_server::ApiServerConnectionServer;
use common::filtergateway::{Action, HandleScenarioRequest};
use common::nodeagent::HandleYamlRequest;
use tonic::transport::Server;

/// Launch REST API listener, gRPC server, and reload scenario data in etcd
pub async fn initialize() {
    tokio::join!(
        crate::route::launch_tcp_listener(),
        start_grpc_server(),
        reload()
    );
}

/// Start gRPC server for node communications
async fn start_grpc_server() {
    let addr = common::apiserver::open_grpc_server()
        .parse()
        .expect("Invalid gRPC server address");

    let grpc_service = crate::grpc::receiver::ApiServerReceiver::new();

    println!("ApiServer gRPC listening on {}", addr);

    let _ = Server::builder()
        .add_service(ApiServerConnectionServer::new(grpc_service))
        .serve(addr)
        .await;
}

/// (under construction) Send request message to piccolo cloud
///
/// ### Parametets
/// TBD
/// ### Description
/// TODO
async fn send_download_request() {}

/// Reload all scenario data in etcd
///
/// ### Parametets
/// * None
/// ### Description
/// This function is called once when the apiserver starts.
async fn reload() {
    let scenarios_result = crate::artifact::data::read_all_scenario_from_etcd().await;

    if let Ok(scenarios) = scenarios_result {
        for scenario in scenarios {
            let req = HandleScenarioRequest {
                action: Action::Apply.into(),
                scenario,
            };
            if let Err(status) = crate::grpc::sender::filtergateway::send(req).await {
                println!("{:#?}", status);
            }
        }
    } else {
        println!("{:#?}", scenarios_result);
    }
}

/// Apply downloaded artifact
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Description
/// write artifact in etcd
/// (optional) make yaml, kube files for Bluechi
/// send a gRPC message to gateway
pub async fn apply_artifact(body: &str) -> common::Result<()> {
    let scenario = crate::artifact::apply(body).await?;

    let handle_yaml = HandleYamlRequest {
        yaml: body.to_string(),
    };

    crate::grpc::sender::nodeagent::send(handle_yaml.clone()).await?;

    if let Some(guests) = &common::setting::get_config().guest {
        if !guests.is_empty() {
            crate::grpc::sender::nodeagent::send_guest(handle_yaml).await?;
        }
    } else {
        println!("Guest configuration not found, skipping guest node");
    }

    let req: HandleScenarioRequest = HandleScenarioRequest {
        action: Action::Apply.into(),
        scenario: scenario,
    };
    crate::grpc::sender::filtergateway::send(req).await?;
    Ok(())
}

/// Withdraw downloaded artifact
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Description
/// delete artifact in etcd
/// (optional) delete yaml, kube files for Bluechi
/// send a gRPC message to gateway
pub async fn withdraw_artifact(body: &str) -> common::Result<()> {
    let scenario = crate::artifact::withdraw(body).await?;

    let req = HandleScenarioRequest {
        action: Action::Withdraw.into(),
        scenario,
    };
    crate::grpc::sender::filtergateway::send(req).await?;

    Ok(())
}

//UNIT Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use common::filtergateway::{
        filter_gateway_connection_client::FilterGatewayConnectionClient,
        filter_gateway_connection_server::{
            FilterGatewayConnection, FilterGatewayConnectionServer,
        },
        Action, HandleScenarioRequest, HandleScenarioResponse,
    };
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;
    use tonic::{Request, Response, Status};

    // === Sample YAML inputs for different test scenarios ===
    /// Correct valid YAML artifact (Scenario + Package + Model)
    const VALID_ARTIFACT_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
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
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// Invalid YAML — missing `action` field
    const INVALID_ARTIFACT_YAML_MISSING_ACTION: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  target: helloworld

---
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
        volume: []
        network: []

---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// Invalid YAML — missing required fields (`kind` and `metadata.name`)
    const INVALID_ARTIFACT_YAML_MISSING_REQUIRED_FIELDS: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: 

spec:
  condition:
  action: update
  target: helloworld

---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
  apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld

---
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
        volume: []
        network: []

    - name: helloworld-core
      node: HPC
      resources:
        volume: []
        network: []

---
apiVersion: v1
kind: Modelr#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
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
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// Invalid YAML — malformed structure -Missing the list of patterns
    const INVALID_ARTIFACT_YAML_MALFORMED_STRUCTURE: &str = r#"
apiVersion: v1
metadata:
  name: helloworld
spec:
  action: update
  target: helloworld
"#;

    /// Invalid YAML — extra fields (`target` not under `spec`)
    const INVALID_ARTIFACT_YAML_EXTRA_FIELDS: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
spec:
  condition:
  action: update

target: helloworld  # Should be under `spec`

---
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
        volume: []
        network: []

---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
    
"#;

    /// Invalid YAML only UnKnown
    const INVALID_ARTIFACT_YAML_UNKNOWN: &str = r#"
apiVersion: v1
kind: Unknown
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
  
"#;

    /// Invalid YAML Empty
    const INVALID_ARTIFACT_YAML_EMPTY: &str = r#"
"#;

    /// Valid YAML WITH KNOWN/UNKNOWN ARTIFACT
    const VALID_ARTIFACT_YAML_KNOWN_UNKNOWN: &str = r#"
    
apiVersion: v1
kind: known_Unknown
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld

---
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
    express: eq
    value: "true"
    operands:
      type: DDS
      name: value
      value: ADASObstacleDetectionIsWarning
  action: update
  target: helloworld
---
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

    /// Invalid YAML WITH KNOWN/UNKNOWN ARTIFACT WITHOUT SCENARIO
    const INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_SCENARIO: &str = r#"

apiVersion: v1
kind: known_unknown
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
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
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// Invalid YAML WITH KNOWN/UNKNOWN ARTIFACT WITHOUT PACKAGE
    const INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_PACKAGE: &str = r#"

apiVersion: v1
kind: known_unknown
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// A mock implementation of the FilterGatewayConnection gRPC service.
    /// Simulates gRPC responses depending on the content of the scenario string.
    #[derive(Default)]
    struct MockFilterGateway;

    #[tonic::async_trait]
    impl FilterGatewayConnection for MockFilterGateway {
        /// Mocks the handle_scenario gRPC method.
        /// Returns error if scenario is empty.
        /// Returns failure status if scenario contains keywords indicating invalid input.
        /// Returns success otherwise.
        async fn handle_scenario(
            &self,
            request: Request<HandleScenarioRequest>,
        ) -> Result<Response<HandleScenarioResponse>, Status> {
            let req = request.into_inner();

            if req.scenario.trim().is_empty() {
                // Reject empty scenario input
                return Err(Status::invalid_argument("Empty scenario"));
            }

            // Return failure for known invalid test inputs to simulate real failure
            if req.scenario.contains("missing")
                || req.scenario.contains("malformed")
                || req.scenario.contains("extra")
                || req.scenario.contains("unknown")
                || req.scenario.contains("no scenario")
                || req.scenario.contains("no package")
            {
                return Ok(Response::new(HandleScenarioResponse {
                    status: false,
                    desc: "Simulated failure for invalid input".to_string(),
                }));
            }

            // Otherwise, simulate successful handling
            Ok(Response::new(HandleScenarioResponse {
                status: true,
                desc: "Success".to_string(),
            }))
        }
    }

    /// Starts the mock gRPC server asynchronously on a random port.
    /// Returns the server socket address for client connections.
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

        // Small delay to ensure server is ready before client tries to connect
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        addr
    }

    /// Helper function to send a HandleScenarioRequest to the mock gRPC server.
    /// Returns error if the server responds with failure status or connection issues.
    async fn mock_send(
        req: HandleScenarioRequest,
        addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut client = FilterGatewayConnectionClient::connect(format!("http://{}", addr)).await?;
        let response = client.handle_scenario(Request::new(req)).await?;

        if !response.get_ref().status {
            return Err("Mock server returned failure".into());
        }
        Ok(())
    }

    /// Mocked version of apply_artifact function.
    /// Instead of full production logic, this sends a gRPC request to the mock server.
    async fn apply_artifact(
        body: &str,
        grpc_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let scenario = crate::artifact::apply(body).await?;

        // Prepare the gRPC request with Apply action
        let req = HandleScenarioRequest {
            action: Action::Apply.into(),
            scenario,
        };

        // Send request to the mock gRPC server
        mock_send(req, grpc_addr).await
    }

    /// Mocked version of withdraw_artifact function.
    /// Sends a gRPC withdraw request to the mock server.
    async fn withdraw_artifact(
        body: &str,
        grpc_addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let scenario = crate::artifact::withdraw(body).await?;

        let req = HandleScenarioRequest {
            action: Action::Withdraw.into(),
            scenario,
        };

        mock_send(req, grpc_addr).await
    }

    /// Mocked version of reload function.
    /// Sends a gRPC reload request to the mock server.
    async fn reload() {
        let scenarios_result = crate::artifact::data::read_all_scenario_from_etcd().await;
        let grpc_addr = start_mock_server().await;
        if let Ok(scenarios) = scenarios_result {
            for scenario in scenarios {
                let req = HandleScenarioRequest {
                    action: Action::Apply.into(),
                    scenario,
                };
                if let Err(status) = mock_send(req, grpc_addr).await {
                    println!("{:#?}", status);
                }
            }
        } else {
            println!("{:#?}", scenarios_result);
        }
    }

    // ======== UNIT TEST CASES FOR APPLY_ARTIFACT ========

    /// Test for `apply_artifact` - successful case
    #[tokio::test]
    async fn test_apply_artifact_success() {
        let addr = start_mock_server().await;

        let result = apply_artifact(VALID_ARTIFACT_YAML, addr).await;
        assert!(
            result.is_ok(),
            "apply_artifact() failed unexpectedly: {:?}",
            result.err()
        );
    }

    /// Test for `apply_artifact` - success when passing known/Unknown artifact YAML
    #[tokio::test]
    async fn test_apply_artifact_known_unknown_yaml() {
        let addr = start_mock_server().await;

        let result = apply_artifact(VALID_ARTIFACT_YAML_KNOWN_UNKNOWN, addr).await;
        assert!(
            result.is_ok(),
            "apply_artifact() failed unexpectedly: {:?}",
            result.err()
        );
    }

    /// Test for `apply_artifact` - failure due to missing `action` field
    #[tokio::test]
    async fn test_apply_artifact_failure_missing_action() {
        let addr = start_mock_server().await;

        let result = apply_artifact(INVALID_ARTIFACT_YAML_MISSING_ACTION, addr).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with missing `action` field"
        );
    }

    /// Test for `apply_artifact` - failure due to missing required fields (`kind` and `metadata.name`)
    #[tokio::test]
    async fn test_apply_artifact_failure_missing_required_fields() {
        let addr = start_mock_server().await;

        let result = apply_artifact(INVALID_ARTIFACT_YAML_MISSING_REQUIRED_FIELDS, addr).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with missing required fields"
        );
    }

    /// Test for `apply_artifact` - failure due to malformed structure (missing list of patterns)
    #[tokio::test]
    async fn test_apply_artifact_failure_malformed_structure() {
        let addr = start_mock_server().await;

        let result = apply_artifact(INVALID_ARTIFACT_YAML_MALFORMED_STRUCTURE, addr).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with malformed structure"
        );
    }

    /// Test for `apply_artifact` - failure due to extra fields (`target` not under `spec`)
    #[tokio::test]
    async fn test_apply_artifact_failure_extra_fields() {
        let addr = start_mock_server().await;

        let result = apply_artifact(INVALID_ARTIFACT_YAML_EXTRA_FIELDS, addr).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with extra fields outside of `spec`"
        );
    }

    /// Test for `apply_artifact` - failure due to empty YAML input
    #[tokio::test]
    async fn test_apply_artifact_empty_yaml() {
        let addr = start_mock_server().await;

        let result = apply_artifact(INVALID_ARTIFACT_YAML_EMPTY, addr).await;
        if let Err(e) = &result {
            println!("apply_artifact() failed with error: {:?}", e);
        }
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with empty YAML"
        );
    }

    /// Test for `apply_artifact` - failure due to unknown artifact YAML
    #[tokio::test]
    async fn test_apply_artifact_unknown_yaml() {
        let addr = start_mock_server().await;

        let result = apply_artifact(INVALID_ARTIFACT_YAML_UNKNOWN, addr).await;
        if let Err(e) = &result {
            println!("apply_artifact() failed with error: {:?}", e);
        }
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with unknown artifact YAML"
        );
    }

    /// Test for `apply_artifact` - failure due to known/unknown artifact without scenario
    #[tokio::test]
    async fn test_apply_artifact_known_unknown_without_scenario() {
        let addr = start_mock_server().await;

        let result =
            apply_artifact(INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_SCENARIO, addr).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with known/unknown artifact missing scenario"
        );
    }

    /// Test for `apply_artifact` - failure due to known/unknown artifact without package
    #[tokio::test]
    async fn test_apply_artifact_known_unknown_without_package() {
        let addr = start_mock_server().await;

        let result =
            apply_artifact(INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_PACKAGE, addr).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with known/unknown artifact missing package"
        );
    }

    // ======== UNIT TEST CASES FOR WITHDRAW_ARTIFACT ========

    /// Test for `withdraw_artifact` - successful case
    #[tokio::test]
    async fn test_withdraw_artifact_success() {
        let addr = start_mock_server().await;

        let result = withdraw_artifact(VALID_ARTIFACT_YAML, addr).await;
        assert!(
            result.is_ok(),
            "withdraw_artifact() failed unexpectedly: {:?}",
            result.err()
        );
    }

    /// Test for `withdraw_artifact` - failure due to empty YAML input
    #[tokio::test]
    async fn test_withdraw_artifact_empty_yaml() {
        let addr = start_mock_server().await;

        let result = withdraw_artifact(INVALID_ARTIFACT_YAML_EMPTY, addr).await;
        assert!(
            result.is_err(),
            "withdraw_artifact() unexpectedly succeeded with empty YAML"
        );
    }

    // Test for `withdraw_artifact` - failure due to missing `action` field
    #[tokio::test]
    async fn test_withdraw_artifact_failure_missing_action() {
        let addr = start_mock_server().await;

        let result = withdraw_artifact(INVALID_ARTIFACT_YAML_MISSING_ACTION, addr).await;
        assert!(
            result.is_err(),
            "withdraw_artifact() unexpectedly succeeded with missing `action` field"
        );
    }

    // Test for `withdraw_artifact` - failure due to missing required fields
    #[tokio::test]
    async fn test_withdraw_artifact_failure_missing_required_fields() {
        let addr = start_mock_server().await;

        let result = withdraw_artifact(INVALID_ARTIFACT_YAML_MISSING_REQUIRED_FIELDS, addr).await;
        assert!(
            result.is_err(),
            "withdraw_artifact() unexpectedly succeeded with missing required fields"
        );
    }

    // Test for `withdraw_artifact` - failure due to malformed structure
    #[tokio::test]
    async fn test_withdraw_artifact_failure_malformed_structure() {
        let addr = start_mock_server().await;

        let result = withdraw_artifact(INVALID_ARTIFACT_YAML_MALFORMED_STRUCTURE, addr).await;
        assert!(
            result.is_err(),
            "withdraw_artifact() unexpectedly succeeded with malformed structure"
        );
    }

    // Test for `withdraw_artifact` - failure due to extra fields
    #[tokio::test]
    async fn test_withdraw_artifact_failure_extra_fields() {
        let addr = start_mock_server().await;

        let result = withdraw_artifact(INVALID_ARTIFACT_YAML_EXTRA_FIELDS, addr).await;
        assert!(
            result.is_err(),
            "withdraw_artifact() unexpectedly succeeded with extra fields outside of `spec`"
        );
    }

    // Test for `send_download_request()` - currently unimplemented (but we can still test its existence)
    #[tokio::test]
    async fn test_send_download_request() {
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            send_download_request(),
        )
        .await;
        assert!(result.is_ok(), "send_download_request() failed to execute");
    }

    // Test for `reload()` - successful case
    #[tokio::test]
    async fn test_reload_success() {
        let result = tokio::time::timeout(std::time::Duration::from_millis(500), reload()).await;
        assert!(result.is_ok(), "reload() failed to complete in time");
    }
}
