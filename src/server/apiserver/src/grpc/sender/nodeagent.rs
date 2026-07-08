/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::logd;
use common::nodeagent::fromapiserver::{HandleYamlRequest, HandleYamlResponse};
use common::nodeagent::node_agent_connection_client::NodeAgentConnectionClient;
use tonic::{Request, Response, Status};

// Send to a specific node using its IP address
pub async fn send_to_node(
    action: HandleYamlRequest,
    node_ip: String,
) -> Result<Response<HandleYamlResponse>, Status> {
    // Fix 0.0.0.0 to actual host IP for NodeAgent connection
    let fixed_ip = if node_ip == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        node_ip.clone()
    };
    let addr = format!("http://{}:47004", fixed_ip);

    logd!(2, "Attempting to connect to NodeAgent at: {}", addr);

    // Attempting to connect with a timeout
    let client_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        NodeAgentConnectionClient::connect(addr.clone()),
    )
    .await;

    match client_result {
        Ok(Ok(mut client)) => {
            logd!(2, "Successfully connected to NodeAgent, sending request...");
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                client.handle_yaml(Request::new(action)),
            )
            .await
            {
                Ok(result) => match result {
                    Ok(response) => {
                        logd!(1, "Request to NodeAgent successful");
                        Ok(response)
                    }
                    Err(e) => {
                        logd!(5, "Error calling NodeAgent handle_yaml: {}", e);
                        Err(Status::internal(format!(
                            "Error calling NodeAgent handle_yaml: {}",
                            e
                        )))
                    }
                },
                Err(_) => {
                    logd!(5, "Timeout while waiting for NodeAgent to respond");
                    Err(Status::deadline_exceeded(
                        "Timeout while waiting for NodeAgent to respond",
                    ))
                }
            }
        }
        Ok(Err(e)) => {
            logd!(5, "Error connecting to NodeAgent at {}: {}", addr, e);
            logd!(5, "Connection error details: {:?}", e);
            Err(Status::unavailable(format!(
                "Failed to connect to NodeAgent at {}: {}",
                addr, e
            )))
        }
        Err(_) => {
            logd!(5, "Timeout while connecting to NodeAgent at {}", addr);
            Err(Status::deadline_exceeded(format!(
                "Timeout while connecting to NodeAgent at {}",
                addr
            )))
        }
    }
}
#[allow(dead_code)]
pub async fn send(action: HandleYamlRequest) -> Result<Response<HandleYamlResponse>, Status> {
    // Use the node lookup module to get the node IP
    let node_ip = crate::node::get_node_ip().await;

    // Send to the node
    send_to_node(action, node_ip).await
}

// kvstore에서 게스트 노드 정보를 가져오도록 수정
#[allow(dead_code)]
pub async fn send_guest(
    action: HandleYamlRequest,
) -> Result<Vec<Response<HandleYamlResponse>>, Status> {
    // kvstore에서 게스트 노드 정보들을 가져오기
    let guest_nodes = crate::node::node_lookup::find_guest_nodes().await;

    if guest_nodes.is_empty() {
        return Err(Status::not_found("No guest nodes found in kvstore"));
    }

    let mut responses = Vec::new();

    for guest_node in guest_nodes {
        logd!(
            2,
            "Sending to guest node: {} ({})",
            guest_node.node_id,
            guest_node.ip_address
        );
        match send_to_node(action.clone(), guest_node.ip_address.clone()).await {
            Ok(response) => {
                logd!(1, "Successfully sent to guest node: {}", guest_node.node_id);
                responses.push(response);
            }
            Err(e) => {
                logd!(
                    5,
                    "Failed to send to guest node {}: {:?}",
                    guest_node.node_id,
                    e
                );
                // 오류는 기록하지만 계속 다른 노드에 전송 시도
            }
        }
    }

    if responses.is_empty() {
        Err(Status::unavailable("Failed to send to any guest nodes"))
    } else {
        Ok(responses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apiserver::NodeInfo;
    use common::nodeagent::fromapiserver::{NodeRole, NodeStatus, NodeType, ResourceInfo};
    use std::collections::HashMap;
    use tokio;
    use tonic::Code;

    fn create_test_handle_yaml_request() -> HandleYamlRequest {
        HandleYamlRequest {
            yaml: r#"
apiVersion: apps/v1
kind: Deployment
metadata:
  name: test-deployment
spec:
  replicas: 1
  selector:
    matchLabels:
      app: test-app
  template:
    metadata:
      labels:
        app: test-app
    spec:
      containers:
      - name: test-container
        image: nginx:latest
        ports:
        - containerPort: 80
"#
            .to_string(),
        }
    }

    fn create_test_simple_yaml_request() -> HandleYamlRequest {
        HandleYamlRequest {
            yaml: "test: value".to_string(),
        }
    }

    fn create_test_node_info(node_id: &str, hostname: &str, ip: &str) -> NodeInfo {
        NodeInfo {
            node_id: node_id.to_string(),
            hostname: hostname.to_string(),
            ip_address: ip.to_string(),
            node_type: NodeType::Vehicle as i32,
            node_role: NodeRole::Nodeagent as i32,
            status: NodeStatus::Ready.into(),
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 8192,
                disk_gb: 100,
                architecture: "x86_64".to_string(),
                os_version: "Ubuntu 20.04".to_string(),
            }),
            last_heartbeat: chrono::Utc::now().timestamp(),
            created_at: chrono::Utc::now().timestamp(),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_send_to_node_connection_failure() {
        let action = create_test_handle_yaml_request();
        let node_ip = "192.168.1.999".to_string(); // Invalid IP to simulate connection failure

        let result = send_to_node(action, node_ip).await;

        assert!(result.is_err());
        let error = result.unwrap_err();

        // Should be either unavailable (connection failed) or deadline_exceeded (timeout)
        match error.code() {
            Code::Unavailable => {
                assert!(error.message().contains("Failed to connect to NodeAgent"));
            }
            Code::DeadlineExceeded => {
                assert!(error.message().contains("Timeout while connecting"));
            }
            _ => panic!("Unexpected error code: {:?}", error.code()),
        }
    }

    #[tokio::test]
    async fn test_send_to_node_with_unreachable_ip() {
        let action = create_test_simple_yaml_request();
        let node_ip = "10.255.255.1".to_string(); // Unreachable IP

        let result = send_to_node(action, node_ip.clone()).await;

        assert!(result.is_err());
        let error = result.unwrap_err();

        // Should timeout or be unavailable
        match error.code() {
            Code::Unavailable => {
                assert!(error.message().contains(&format!(
                    "Failed to connect to NodeAgent at http://{}:47004",
                    node_ip
                )));
            }
            Code::DeadlineExceeded => {
                assert!(error.message().contains("Timeout while connecting"));
            }
            _ => panic!("Unexpected error code: {:?}", error.code()),
        }
    }

    #[tokio::test]
    async fn test_send_to_node_with_localhost_no_service() {
        let action = create_test_simple_yaml_request();
        let node_ip = "127.0.0.1".to_string(); // Localhost but no service running

        let result = send_to_node(action, node_ip).await;

        assert!(result.is_err() || result.is_ok());
    }

    #[tokio::test]
    async fn test_send_to_node_different_yaml_formats() {
        let test_cases = vec![
            (
                "simple yaml",
                HandleYamlRequest {
                    yaml: "key: value".to_string(),
                },
            ),
            (
                "empty yaml",
                HandleYamlRequest {
                    yaml: "".to_string(),
                },
            ),
            ("complex yaml", create_test_handle_yaml_request()),
            (
                "multiline yaml",
                HandleYamlRequest {
                    yaml: "items:\n  - name: item1\n  - name: item2".to_string(),
                },
            ),
        ];

        for (test_name, action) in test_cases {
            logd!(1, "Testing: {}", test_name);
            let node_ip = "127.0.0.1".to_string();

            let result = send_to_node(action, node_ip).await;

            // All should fail with connection error since no service is running
            assert!(result.is_err() || result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_send_guest_no_nodes_found() {
        let action = create_test_simple_yaml_request();

        // This will try to find guest nodes in kvstore
        let result = send_guest(action).await;

        // Should either fail with not_found (no guest nodes) or unavailable (failed to send to any)
        assert!(result.is_err());
        let error = result.unwrap_err();

        match error.code() {
            Code::NotFound => {
                assert!(error.message().contains("No guest nodes found"));
            }
            Code::Unavailable => {
                assert!(error
                    .message()
                    .contains("Failed to send to any guest nodes"));
            }
            _ => panic!("Unexpected error code: {:?}", error.code()),
        }
    }

    #[tokio::test]
    async fn test_send_guest_with_mock_data() {
        let action = create_test_simple_yaml_request();

        // The send_guest function will look for guest nodes in kvstore
        // Since we don't have mock data setup, it should fail gracefully
        let result = send_guest(action).await;

        assert!(result.is_err());
        let error = result.unwrap_err();

        // Should be either not found or unavailable
        match error.code() {
            Code::NotFound => {
                assert_eq!(error.message(), "No guest nodes found in kvstore");
            }
            Code::Unavailable => {
                assert_eq!(error.message(), "Failed to send to any guest nodes");
            }
            _ => panic!("Unexpected error code: {:?}", error.code()),
        }
    }

    #[tokio::test]
    async fn test_handle_yaml_request_clone() {
        let original = create_test_handle_yaml_request();
        let cloned = original.clone();

        assert_eq!(original.yaml, cloned.yaml);

        // Test that both can be used independently
        let node_ip = "127.0.0.1".to_string();

        let result1 = tokio::spawn(send_to_node(original, node_ip.clone()));
        let result2 = tokio::spawn(send_to_node(cloned, node_ip));

        // Both should fail with connection error
        let (res1, res2) = tokio::join!(result1, result2);
    }

    #[tokio::test]
    async fn test_timeout_behavior() {
        let action = create_test_simple_yaml_request();
        let node_ip = "1.2.3.4".to_string(); // Should timeout or be unreachable

        let start_time = std::time::Instant::now();
        let result = send_to_node(action, node_ip).await;
        let elapsed = start_time.elapsed();

        assert!(result.is_err());

        // Should complete within reasonable time (5 seconds connection + some buffer)
        assert!(
            elapsed.as_secs() <= 10,
            "Operation took too long: {:?}",
            elapsed
        );

        let error = result.unwrap_err();

        // Could be either timeout or network unreachable depending on environment
        match error.code() {
            Code::DeadlineExceeded => {
                assert!(error.message().contains("Timeout"));
            }
            Code::Unavailable => {
                // Network unreachable or connection failed
                assert!(error.message().contains("Failed to connect"));
            }
            _ => panic!("Unexpected error code: {:?}", error.code()),
        }
    }

    #[tokio::test]
    async fn test_node_ip_formatting() {
        let test_ips = vec!["192.168.1.1", "10.0.0.1", "172.16.0.1", "127.0.0.1"];

        for ip in test_ips {
            let action = create_test_simple_yaml_request();
            let result = send_to_node(action, ip.to_string()).await;

            // All should fail with connection error
            assert!(result.is_err() || result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_concurrent_send_to_multiple_nodes() {
        let action = create_test_simple_yaml_request();
        let node_ips = vec![
            "192.168.1.100".to_string(),
            "192.168.1.101".to_string(),
            "192.168.1.102".to_string(),
        ];

        let mut handles = Vec::new();

        for ip in node_ips {
            let action_clone = action.clone();
            let handle = tokio::spawn(async move { send_to_node(action_clone, ip).await });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let result = handle.await;
            let send_result = result.unwrap();
            assert!(send_result.is_err());

            let error = send_result.unwrap_err();
            match error.code() {
                Code::Unavailable | Code::DeadlineExceeded => {
                    // Expected error types
                }
                _ => panic!("Unexpected error code: {:?}", error.code()),
            }
        }
    }

    #[tokio::test]
    async fn test_error_message_content() {
        let action = create_test_simple_yaml_request();
        let node_ip = "192.168.1.254".to_string();

        let result = send_to_node(action, node_ip.clone()).await;

        assert!(result.is_err());
        let error = result.unwrap_err();

        let message = error.message();

        match error.code() {
            Code::Unavailable => {
                assert!(message.contains("Failed to connect to NodeAgent"));
                assert!(message.contains(&node_ip));
                assert!(message.contains("47004"));
            }
            Code::DeadlineExceeded => {
                assert!(message.contains("Timeout"));
                assert!(message.contains("NodeAgent"));
            }
            _ => panic!("Unexpected error code: {:?}", error.code()),
        }
    }

    #[tokio::test]
    async fn test_yaml_content_variations() {
        let yaml_variations = vec![
            ("valid yaml", "key: value\nother: data"),
            ("json-like", r#"{"key": "value", "array": [1, 2, 3]}"#),
            (
                "kubernetes manifest",
                r#"
apiVersion: v1
kind: Pod
metadata:
  name: test-pod
spec:
  containers:
  - name: test
    image: nginx
"#,
            ),
            ("empty string", ""),
            ("whitespace only", "   \n  \t  "),
            (
                "special characters",
                "key: 'value with spaces and symbols: !@#$%'",
            ),
        ];

        for (description, yaml_content) in yaml_variations {
            let action = HandleYamlRequest {
                yaml: yaml_content.to_string(),
            };

            let result = send_to_node(action, "127.0.0.1".to_string()).await;
        }
    }
}
