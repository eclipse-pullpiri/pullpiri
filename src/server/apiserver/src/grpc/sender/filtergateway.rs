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
    use common::filtergateway::{Action, HandleScenarioRequest}; // Import Action type
    use tonic::Status;
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

    const INVALID_SCENARIO_YAML_EMPTY: &str = r#"
"#;

    const INVALID_SCENARIO_YAML_MISSING_FIELD: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name:
spec:
  condition:
  action: update
  target: helloworld
"#;

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
    /// Test the `send()` function with a valid scenario and action APPLY
    #[tokio::test]
    async fn test_send_with_valid_scenario_apply() {
        // Create a valid HandleScenarioRequest with action APPLY
        let scenario = HandleScenarioRequest {
            action: Action::Apply.into(),
            scenario: VALID_SCENARIO_YAML.to_string(), // Scenario name as a string
        };

        // Call the send() function directly
        let result = send(scenario).await;

        // Check that the result is an error (because no server is running)
        assert!(
            result.is_err(),
            "Expected an error when sending without a server"
        );

        // If it's an error, check if it's a Status with a connection issue
        if let Err(e) = result {
            let error_message = format!("{}", e);
            println!("Received error: {}", error_message);
            assert!(
                error_message.contains("connection"),
                "Expected a connection error, but got: {}",
                error_message
            );
        }
    }

    /// Test the `send()` function with a valid scenario and action WITHDRAW
    #[tokio::test]
    async fn test_send_with_valid_scenario_withdraw() {
        // Create a valid HandleScenarioRequest with action WITHDRAW
        let scenario = HandleScenarioRequest {
            action: Action::Withdraw.into(),
            scenario: VALID_SCENARIO_YAML.to_string(), // Scenario name as a string
        };

        // Call the send() function directly
        let result = send(scenario).await;

        // Check that the result is an error (because no server is running)
        assert!(
            result.is_err(),
            "Expected an error when sending without a server"
        );

        // If it's an error, check if it's a Status with a connection issue
        if let Err(e) = result {
            let error_message = format!("{}", e);
            println!("Received error: {}", error_message);
            assert!(
                error_message.contains("connection"),
                "Expected a connection error, but got: {}",
                error_message
            );
        }
    }

    /// Test the `send()` function with an empty scenario name
    #[tokio::test]
    async fn test_send_with_empty_scenario_name_apply() {
        // Create a HandleScenarioRequest with an empty scenario name
        let scenario = HandleScenarioRequest {
            action: Action::Apply.into(),
            scenario: INVALID_SCENARIO_YAML_EMPTY.to_string(), // Empty scenario list
        };

        // Call the send() function directly
        let result = send(scenario).await;

        // Assert that the result is an error due to invalid scenario name
        assert!(result.is_err(), "Expected an error for empty scenario name");

        if let Err(e) = result {
            let error_message = format!("{}", e);
            println!("Received error: {}", error_message);
            assert!(
                error_message.contains("invalid argument"),
                "Expected 'invalid argument' error, but got: {}",
                error_message
            );
        }
    }

    /// Test the `send()` function with missing required fields (empty request)
    #[tokio::test]
    async fn test_send_with_missing_field_apply() {
        // Create a HandleScenarioRequest with empty values
        let scenario = HandleScenarioRequest {
            action: Action::Apply.into(),
            scenario: INVALID_SCENARIO_YAML_MISSING_FIELD.to_string(), // MISSING NAME field
        };

        // Call the send() function directly
        let result = send(scenario).await;

        // Assert that the result is an error due to missing or empty fields
        assert!(result.is_err(), "Expected an error due to empty fields");

        if let Err(e) = result {
            let error_message = format!("{}", e);
            println!("Received error: {}", error_message);
            assert!(
                error_message.contains("invalid argument"),
                "Expected 'invalid argument' error, but got: {}",
                error_message
            );
        }
    }

    /// Test the `send()` function with a non-existent scenario (this assumes your system handles this case)
    #[tokio::test]
    async fn test_send_with_nonexistent_scenario_apply() {
        // Create a HandleScenarioRequest with a non-existent scenario
        let scenario = HandleScenarioRequest {
            action: Action::Apply.into(),
            scenario: INVALID_NO_SCENARIO_YAML.to_string(), // Non-existent scenario
        };

        // Call the send() function directly
        let result = send(scenario).await;

        // Assert that the result is an error due to non-existent scenario
        assert!(
            result.is_err(),
            "Expected an error for non-existent scenario"
        );

        if let Err(e) = result {
            let error_message = format!("{}", e);
            println!("Received error: {}", error_message);
            assert!(
                error_message.contains("not found"),
                "Expected 'not found' error, but got: {}",
                error_message
            );
        }
    }

    /// Test the `send()` function with an empty scenario name
    #[tokio::test]
    async fn test_send_with_empty_scenario_name_withdraw() {
        // Create a HandleScenarioRequest with an empty scenario name
        let scenario = HandleScenarioRequest {
            action: Action::Withdraw.into(),
            scenario: INVALID_SCENARIO_YAML_EMPTY.to_string(), // Empty scenario list
        };

        // Call the send() function directly
        let result = send(scenario).await;

        // Assert that the result is an error due to invalid scenario name
        assert!(result.is_err(), "Expected an error for empty scenario name");

        if let Err(e) = result {
            let error_message = format!("{}", e);
            println!("Received error: {}", error_message);
            assert!(
                error_message.contains("invalid argument"),
                "Expected 'invalid argument' error, but got: {}",
                error_message
            );
        }
    }

    /// Test the `send()` function with missing required fields (empty request)
    #[tokio::test]
    async fn test_send_with_missing_field_withdraw() {
        // Create a HandleScenarioRequest with empty values
        let scenario = HandleScenarioRequest {
            action: Action::Withdraw.into(),
            scenario: INVALID_SCENARIO_YAML_MISSING_FIELD.to_string(), // MISSING NAME field
        };

        // Call the send() function directly
        let result = send(scenario).await;

        // Assert that the result is an error due to missing or empty fields
        assert!(result.is_err(), "Expected an error due to empty fields");

        if let Err(e) = result {
            let error_message = format!("{}", e);
            println!("Received error: {}", error_message);
            assert!(
                error_message.contains("invalid argument"),
                "Expected 'invalid argument' error, but got: {}",
                error_message
            );
        }
    }

    /// Test the `send()` function with a non-existent scenario (this assumes your system handles this case)
    #[tokio::test]
    async fn test_send_with_nonexistent_scenario_withdraw() {
        // Create a HandleScenarioRequest with a non-existent scenario
        let scenario = HandleScenarioRequest {
            action: Action::Withdraw.into(),
            scenario: INVALID_NO_SCENARIO_YAML.to_string(), // Non-existent scenario
        };

        // Call the send() function directly
        let result = send(scenario).await;

        // Assert that the result is an error due to non-existent scenario
        assert!(
            result.is_err(),
            "Expected an error for non-existent scenario"
        );

        if let Err(e) = result {
            let error_message = format!("{}", e);
            println!("Received error: {}", error_message);
            assert!(
                error_message.contains("not found"),
                "Expected 'not found' error, but got: {}",
                error_message
            );
        }
    }
}
