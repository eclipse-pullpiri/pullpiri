/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::actioncontroller::{
    action_controller_connection_client::ActionControllerConnectionClient, connect_server,
    OffloadModelRequest, OffloadModelResponse, ReconcileRequest, ReconcileResponse,
};
use common::policymanager::{
    connect_server as policymanager_connect_server,
    policy_manager_connection_client::PolicyManagerConnectionClient, ReportFaultRequest,
    ReportFaultResponse,
};
use std::env;
use tonic::{Request, Response, Status};

pub async fn _send(condition: ReconcileRequest) -> Result<Response<ReconcileResponse>, Status> {
    // Test mode bypass: return a fake successful response when env var is set
    if env::var("PULLPIRI_TEST_MODE").is_ok() {
        let resp = ReconcileResponse {
            status: 0,
            desc: "mock".to_string(),
        };
        return Ok(Response::new(resp));
    }
    let mut client = ActionControllerConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.reconcile(Request::new(condition)).await
}

/// Send offload model request to ActionController
///
/// This triggers the model migration: terminate on source_node, launch on target_node
pub async fn offload_model(
    request: OffloadModelRequest,
) -> Result<Response<OffloadModelResponse>, Status> {
    // Test mode bypass
    if env::var("PULLPIRI_TEST_MODE").is_ok() {
        let resp = OffloadModelResponse {
            success: true,
            message: "mock offload".to_string(),
            transition_id: "test-transition-id".to_string(),
        };
        return Ok(Response::new(resp));
    }

    let client = ActionControllerConnectionClient::connect(connect_server()).await;

    match client {
        Ok(mut client) => client.offload_model(Request::new(request)).await,
        Err(e) => {
            eprintln!(
                "[StateManager] Failed to connect to ActionController: {}",
                e
            );
            Err(Status::unavailable(format!(
                "Failed to connect to ActionController: {}",
                e
            )))
        }
    }
}

/// Send fault report to PolicyManager
///
/// This forwards fault notifications (e.g., deadline miss) from Timpani to PolicyManager
/// for policy-based decision making
pub async fn report_fault_to_policymanager(
    request: ReportFaultRequest,
) -> Result<Response<ReportFaultResponse>, Status> {
    // Test mode bypass
    if env::var("PULLPIRI_TEST_MODE").is_ok() {
        let resp = ReportFaultResponse {
            processed: true,
            message: "mock fault report".to_string(),
        };
        return Ok(Response::new(resp));
    }

    let client = PolicyManagerConnectionClient::connect(policymanager_connect_server()).await;

    match client {
        Ok(mut client) => client.report_fault(Request::new(request)).await,
        Err(e) => {
            eprintln!("[StateManager] Failed to connect to PolicyManager: {}", e);
            Err(Status::unavailable(format!(
                "Failed to connect to PolicyManager: {}",
                e
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_send_in_test_mode_returns_mock_response() {
        env::set_var("PULLPIRI_TEST_MODE", "1");

        let req = ReconcileRequest {
            scenario_name: "s1".to_string(),
            current: 0,
            desired: 0,
        };

        let res = _send(req).await;
        assert!(res.is_ok());
        let r = res.unwrap();
        assert_eq!(r.get_ref().status, 0);

        env::remove_var("PULLPIRI_TEST_MODE");
    }
}
