use common::filtergateway::{
    filter_gateway_connection_client::FilterGatewayConnectionClient, HandleScenarioRequest,
};
use filtergateway::FilterGatewayReceiver;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tonic::{transport::Server, Request};

#[tokio::test]
/// Test valid YAML scenario is accepted by the gRPC server and forwarded correctly.
async fn test_handle_scenario_with_valid_yaml() {
    let (tx, mut rx) = mpsc::channel(1);
    let receiver = FilterGatewayReceiver::new(tx);

    // Start gRPC server on a local port
    let addr: SocketAddr = "127.0.0.1:50055".parse().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(receiver.into_service())
            .serve(addr)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Create gRPC client
    let mut client = FilterGatewayConnectionClient::connect("http://127.0.0.1:50055")
        .await
        .expect("Failed to connect");

    let scenario_yaml = r#"
    apiVersion: v1
    kind: Scenario
    metadata:
      name: helloworld
    spec:
      condition:
      action: update
      target: helloworld
    "#;

    let request = Request::new(HandleScenarioRequest {
        scenario: scenario_yaml.to_string(),
        action: 0,
    });

    // Send the scenario request
    let response = client.handle_scenario(request).await.unwrap().into_inner();

    // Validate response from gRPC server
    assert!(response.status);
    assert_eq!(response.desc, "Successfully handled scenario");

    // Confirm the scenario was forwarded through the channel
    let received_param = rx.recv().await.unwrap();
    assert_eq!(received_param.action, 0);
    server.abort();
}

#[tokio::test]
/// Test that sending invalid YAML results in an internal gRPC error.
async fn test_handle_scenario_with_invalid_yaml() {
    let (tx, _rx) = mpsc::channel(1);
    let receiver = FilterGatewayReceiver::new(tx);

    let addr: SocketAddr = "127.0.0.1:50056".parse().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(receiver.into_service())
            .serve(addr)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut client = FilterGatewayConnectionClient::connect("http://127.0.0.1:50056")
        .await
        .expect("Failed to connect");

    let invalid_yaml = r#"
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
    "#; // Invalid YAML due to incomplete resources

    let request = Request::new(HandleScenarioRequest {
        scenario: invalid_yaml.to_string(),
        action: 0,
    });

    // The request should fail due to invalid YAML parsing
    let result = client.handle_scenario(request).await;
    assert!(result.is_err());

    server.abort();
}

#[tokio::test]
/// Test that empty YAML input is rejected with an error.
async fn test_handle_scenario_with_empty_yaml() {
    let (tx, _rx) = mpsc::channel(1);
    let receiver = FilterGatewayReceiver::new(tx);

    let addr: SocketAddr = "127.0.0.1:50057".parse().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(receiver.into_service())
            .serve(addr)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut client = FilterGatewayConnectionClient::connect("http://127.0.0.1:50057")
        .await
        .expect("Failed to connect");

    let empty_yaml = "";

    let request = Request::new(HandleScenarioRequest {
        scenario: empty_yaml.to_string(),
        action: 0,
    });

    let result = client.handle_scenario(request).await;
    assert!(result.is_err());

    server.abort();
}

#[tokio::test]
/// Test that YAML missing required fields is rejected.
async fn test_handle_scenario_with_missing_fields() {
    let (tx, _rx) = mpsc::channel(1);
    let receiver = FilterGatewayReceiver::new(tx);

    let addr: SocketAddr = "127.0.0.1:50058".parse().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(receiver.into_service())
            .serve(addr)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut client = FilterGatewayConnectionClient::connect("http://127.0.0.1:50058")
        .await
        .expect("Failed to connect");

    let incomplete_yaml = r#"
    apiVersion: v1
    kind: Scenario
    metadata:
      name: helloworld
    spec:
      action: update
    "#; // Missing "target" field

    let request = Request::new(HandleScenarioRequest {
        scenario: incomplete_yaml.to_string(),
        action: 0,
    });

    let result = client.handle_scenario(request).await;
    assert!(result.is_err());

    server.abort();
}

#[tokio::test]
async fn test_handle_scenario_with_closed_channel_should_fail() {
    let (tx, rx) = mpsc::channel(1);
    drop(rx); // Close the receiving end — channel is now closed

    let receiver = FilterGatewayReceiver::new(tx);

    let addr: SocketAddr = "127.0.0.1:50059".parse().unwrap();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(receiver.into_service())
            .serve(addr)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut client = FilterGatewayConnectionClient::connect("http://127.0.0.1:50059")
        .await
        .expect("Failed to connect");

    let scenario_yaml = r#"
    apiVersion: v1
    kind: Scenario
    metadata:
      name: helloworld
    spec:
      condition:
      action: update
      target: helloworld
    "#;

    let request = Request::new(HandleScenarioRequest {
        scenario: scenario_yaml.to_string(),
        action: 0,
    });

    // Expect the request to fail because the channel is closed
    let result = client.handle_scenario(request).await;

    assert!(
        result.is_err(),
        "Expected gRPC failure due to closed channel"
    );

    server.abort();
}
