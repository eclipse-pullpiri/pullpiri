/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::policymanager::policy_manager_connection_server::PolicyManagerConnection;
use common::policymanager::{CheckPolicyRequest, CheckPolicyResponse};
use tonic::Response;

pub struct PolicyManagerGrpcServer {}

#[tonic::async_trait]
impl PolicyManagerConnection for PolicyManagerGrpcServer {
    async fn check_policy(
        &self,
        request: tonic::Request<CheckPolicyRequest>,
    ) -> Result<tonic::Response<CheckPolicyResponse>, tonic::Status> {
        let req = request.into_inner();
        let scenario_name = req.scenario_name; // Renamed for clarity

        // Simulate internal logic
        let (status, desc) = if scenario_name.is_empty() {
            (1, "Scenario name cannot be empty".to_string())
        } else if scenario_name == "test_scenario" {
            (0, "Policy check passed".to_string())
        } else {
            (
                1,
                format!("Policy check failed for scenario: {}", scenario_name),
            )
        };

        // 🔍 COMMENT 4: PolicyManager policy satisfaction
        // When PolicyManager determines that a scenario satisfies policy requirements
        // (status == 0), it should notify StateManager of the scenario state change
        // from "satisfied" to "allowed" state. This would be done via StateManagerSender.
        // TODO: Add StateManager notification for policy approval
        // if status == 0 {
        //     // Send state change: scenario_name from "satisfied" to "allowed"
        // }

        Ok(Response::new(CheckPolicyResponse { status, desc }))
    }
}
