/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::grpc::sender::statemanager::StateManagerSender;
use common::policymanager::policy_manager_connection_server::PolicyManagerConnection;
use common::policymanager::{CheckPolicyRequest, CheckPolicyResponse};
use common::statemanager::{ResourceType, StateChange};
use tonic::Response;

pub struct PolicyManagerGrpcServer {
    /// StateManager sender for scenario state changes
    state_sender: StateManagerSender,
}

impl PolicyManagerGrpcServer {
    /// Creates a new PolicyManagerGrpcServer instance
    pub fn new() -> Self {
        Self {
            state_sender: StateManagerSender::new(),
        }
    }
}

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

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        if status == 0 {
            // Policy satisfied: satisfied -> allowed
            let state_change = StateChange {
                resource_type: ResourceType::Scenario as i32,
                resource_name: scenario_name.clone(),
                current_state: "satisfied".to_string(),
                target_state: "allowed".to_string(),
                transition_id: format!("policymanager-policy-allowed-{}", timestamp),
                timestamp_ns: timestamp,
                source: "policymanager".to_string(),
            };

            if let Err(e) = self
                .state_sender
                .clone()
                .send_state_change(state_change)
                .await
            {
                println!("Failed to send state change to StateManager: {:?}", e);
            } else {
                println!(
                    "Successfully notified StateManager: scenario {} satisfied -> allowed",
                    scenario_name
                );
            }
        } else {
            // Policy not satisfied: satisfied -> denied
            let state_change = StateChange {
                resource_type: ResourceType::Scenario as i32,
                resource_name: scenario_name.clone(),
                current_state: "satisfied".to_string(),
                target_state: "denied".to_string(),
                transition_id: format!("policymanager-policy-denied-{}", timestamp),
                timestamp_ns: timestamp,
                source: "policymanager".to_string(),
            };

            if let Err(e) = self
                .state_sender
                .clone()
                .send_state_change(state_change)
                .await
            {
                println!("Failed to send state change to StateManager: {:?}", e);
            } else {
                println!(
                    "Successfully notified StateManager: scenario {} satisfied -> denied",
                    scenario_name
                );
            }
        }

        Ok(Response::new(CheckPolicyResponse { status, desc }))
    }
}
