/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::policymanager::policy_manager_connection_server::PolicyManagerConnection;
use common::policymanager::{ CheckPolicyRequest, CheckPolicyResponse };

pub struct PolicyManagerGrpcServer {}

#[tonic::async_trait]
impl PolicyManagerConnection for PolicyManagerGrpcServer {
    async fn check_policy(
        &self,
        request: tonic::Request<CheckPolicyRequest>
    ) -> Result<tonic::Response<CheckPolicyResponse>, tonic::Status> {
        let req = request.into_inner();
        let scenario_name = req.scenario_name; // Renamed for clarity

        // --- Simulate Policy Check Logic ---
        // In a real application, we'd perform our policy check here.
        // For demonstration, let's say 'test_scenario' passes, others fail.
        if scenario_name == "test_scenario" {
            // Policy check passed
            Ok(
                tonic::Response::new(CheckPolicyResponse {
                    status: 0, // Success status
                    desc: "Policy check passed for test_scenario".to_string(),
                })
            )
        } else if scenario_name.trim().is_empty() {
            // Example: Specific gRPC status for invalid argument
            Err(tonic::Status::invalid_argument("Scenario name cannot be empty".to_string()))
        } else {
            // Policy check failed for other scenarios
            // Return a gRPC protocol-level error to the client
            Err(
                tonic::Status::permission_denied(
                    format!("Policy check failed for scenario: {}", scenario_name)
                )
            )
        }
    }
}
