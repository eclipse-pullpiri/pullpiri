/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! gRPC sender for PolicyManager to communicate with StateManager

use common::statemanager::state_manager_connection_client::StateManagerConnectionClient;
use common::statemanager::{connect_server, OffloadingRequest, OffloadingResponse};
use tonic::{Request, Response, Status};

/// Trigger offloading request to StateManager for container migration
pub async fn trigger_offloading(
    request: OffloadingRequest,
) -> Result<Response<OffloadingResponse>, Status> {
    let addr = connect_server();

    let client = StateManagerConnectionClient::connect(addr).await;

    match client {
        Ok(mut client) => client.trigger_offloading(Request::new(request)).await,
        Err(e) => {
            eprintln!("[PolicyManager] Failed to connect to StateManager: {}", e);
            Err(Status::unavailable(format!(
                "Failed to connect to StateManager: {}",
                e
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_trigger_offloading_connection_failure() {
        let request = OffloadingRequest {
            scenario_name: "test-scenario".to_string(),
            package_name: "test-package".to_string(),
            model_name: "test-model".to_string(),
            source_node: "node1".to_string(),
            target_node: "node2".to_string(),
            policy_name: "test-policy".to_string(),
            reason: "Test reason".to_string(),
        };

        let result = trigger_offloading(request).await;
        // Should fail because StateManager is not running
        assert!(result.is_err());
    }
}
