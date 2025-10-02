/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Controls the flow of data between each module.
use crate::node::node_lookup::{find_guest_nodes, find_node_by_hostname, get_node_ip};
use common::apiserver::api_server_connection_server::ApiServerConnectionServer;
use common::filtergateway::{Action, HandleScenarioRequest};
use common::nodeagent::HandleYamlRequest;
use tonic::transport::Server;

/// Launch REST API listener, gRPC server, and reload scenario data in etcd
pub async fn initialize() {
    // 먼저 호스트 노드를 etcd에 등록합니다.
    if let Err(e) = register_host_node().await {
        eprintln!("Failed to register host node: {:?}", e);
    } else {
        println!("Host node registered successfully");
    }

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

/// settings.yaml에서 호스트 정보를 가져와 etcd에 등록합니다.
///
/// 이 함수는 apiserver가 시작될 때 호출되어 자신의 정보를 etcd에 등록합니다.
/// 이렇게 하면 다른 컴포넌트가 apiserver 호스트 정보를 조회할 수 있습니다.
///
/// ### Returns
///
/// * `Result<(), Box<dyn std::error::Error + Send + Sync>>` - 성공 또는 실패 결과
async fn register_host_node() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // settings.yaml에서 호스트 정보 가져오기
    let config = common::setting::get_config();
    let hostname = config.host.name.clone();
    let ip_address = config.host.ip.clone();
    let node_type = match config.host.r#type.as_str() {
        "vehicle" => 2, // NodeType::Vehicle as i32
        "cloud" => 1,   // NodeType::Cloud as i32
        _ => 0,         // NodeType::Unspecified as i32
    };
    let node_role = match config.host.role.as_str() {
        "master" => 1,    // NodeRole::Master as i32
        "nodeagent" => 2, // NodeRole::Nodeagent as i32
        "bluechi" => 3,   // NodeRole::Bluechi as i32
        _ => 0,           // NodeRole::Unspecified as i32
    };

    // NodeRegistrationRequest 생성
    let node_id = format!("{}-{}", hostname, ip_address);
    let registration_request = common::nodeagent::NodeRegistrationRequest {
        node_id: node_id.clone(),
        hostname: hostname.clone(),
        ip_address: ip_address.clone(),
        metadata: std::collections::HashMap::new(),
        resources: None,
        node_type,
        node_role,
    };

    // NodeManager를 사용하여 노드 등록
    let node_manager = crate::node::NodeManager::new()?;
    node_manager.register_node(registration_request).await?;

    // 추가적으로 nodes/{hostname} 키에도 저장 (ActionController가 이 키를 사용)
    let hostname_key = format!("nodes/{}", hostname);
    common::etcd::put(&hostname_key, &ip_address).await?;

    println!(
        "Host node information registered to etcd: {} ({})",
        hostname, ip_address
    );
    Ok(())
}

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

    // 패키지에서 노드 정보를 추출
    let mut target_node = None;

    // YAML 문서에서 Package 종류 찾아서 노드 정보 추출
    let docs: Vec<&str> = body.split("---").collect();
    for doc in docs {
        if doc.trim().is_empty() {
            continue;
        }

        match serde_yaml::from_str::<serde_yaml::Value>(doc) {
            Ok(value) => {
                if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
                    if kind == "Package" {
                        if let Ok(package) =
                            serde_yaml::from_value::<common::spec::artifact::Package>(value.clone())
                        {
                            for model in package.get_models() {
                                let node_name = model.get_node();
                                if !node_name.is_empty() {
                                    println!("Found node in package: {}", node_name);
                                    if let Some(node_info) = find_node_by_hostname(&node_name).await
                                    {
                                        println!(
                                            "Found node info for {}: IP={}",
                                            node_name, node_info.ip_address
                                        );
                                        target_node = Some(node_info.ip_address);
                                        break;
                                    }
                                }
                            }
                            if target_node.is_some() {
                                break;
                            }
                        }
                    }
                }
            }
            Err(_) => continue, // Skip invalid YAML
        }
    }

    // 노드를 찾지 못했으면 기본 노드 IP 사용
    let node_ip = if let Some(ip) = target_node {
        ip
    } else {
        println!("No target nodes found in package or nodes not registered, using default node");
        get_node_ip().await
    };

    println!("apply_artifact: Using node IP: {}", node_ip);

    // Log a warning if using 0.0.0.0 - this is likely a problem
    if node_ip == "0.0.0.0" {
        eprintln!("Warning: Using IP 0.0.0.0 which may not be accessible from NodeAgent.");
        eprintln!(
            "NodeAgent transport errors are likely. Consider using a specific IP in settings.yaml"
        );
    }

    // Try to send to the node
    match crate::grpc::sender::nodeagent::send_to_node(handle_yaml.clone(), node_ip.clone()).await {
        Ok(_) => println!("Successfully sent yaml to NodeAgent"),
        Err(e) => {
            eprintln!("Error sending yaml to NodeAgent: {:?}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("NodeAgent connection error: {:?}", e),
            )));
        }
    };

    // etcd에서 게스트 노드들의 정보를 가져와 yaml 전송
    let guest_nodes = find_guest_nodes().await;
    if guest_nodes.is_empty() {
        println!("No guest nodes found in etcd, skipping guest node deployment");
    } else {
        for guest_node in guest_nodes {
            if guest_node.ip_address != node_ip {
                // 이미 전송한 노드와 다른 경우에만 전송
                println!(
                    "Attempting to send yaml to guest node {} at {}",
                    guest_node.node_id, guest_node.ip_address
                );
                match crate::grpc::sender::nodeagent::send_to_node(
                    handle_yaml.clone(),
                    guest_node.ip_address,
                )
                .await
                {
                    Ok(_) => println!(
                        "Successfully sent yaml to guest NodeAgent {}",
                        guest_node.node_id
                    ),
                    Err(e) => println!(
                        "Error sending yaml to guest NodeAgent {}: {:?}",
                        guest_node.node_id, e
                    ),
                }
            }
        }
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
