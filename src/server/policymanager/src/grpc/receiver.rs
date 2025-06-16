/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::policymanager::policy_manager_connection_server::PolicyManagerConnection;
use common::policymanager::{ CheckPolicyRequest, CheckPolicyResponse };
use tonic::{Response};

pub struct PolicyManagerGrpcServer {}

#[tonic::async_trait]
impl PolicyManagerConnection for PolicyManagerGrpcServer {
    async fn check_policy(
        &self,
        request: tonic::Request<CheckPolicyRequest>
    ) -> Result<tonic::Response<CheckPolicyResponse>, tonic::Status> {
        let req = request.into_inner();
        let scenario_name = req.scenario_name; // Renamed for clarity

        // Simulate internal logic
        let (status, desc) = if scenario_name.is_empty() {
            (1, "Scenario name cannot be empty".to_string())
        } else if scenario_name == "test_scenario" {
            (0, "Policy check passed".to_string())
        } else {
            (1, format!("Policy check failed for scenario: {}", scenario_name))
        };

        Ok(Response::new(CheckPolicyResponse {
            status,
            desc,
        }))
    }
}
