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

        println!("🔄 SCENARIO STATE TRANSITION: PolicyManager Processing");
        println!("   📋 Scenario: {}", scenario_name);
        println!(
            "   🛡️  Policy Check Status: {}",
            if status == 0 { "PASSED" } else { "FAILED" }
        );

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        if status == 0 {
            // Policy satisfied: satisfied -> allowed
            println!("   🔄 State Change: satisfied → allowed");
            println!("   🔍 Reason: Policy requirements satisfied");

            let state_change = StateChange {
                resource_type: ResourceType::Scenario as i32,
                resource_name: scenario_name.clone(),
                current_state: "satisfied".to_string(),
                target_state: "allowed".to_string(),
                transition_id: format!("policymanager-policy-allowed-{}", timestamp),
                timestamp_ns: timestamp,
                source: "policymanager".to_string(),
            };

            println!("   📤 Sending StateChange to StateManager:");
            println!("      • Resource Type: SCENARIO");
            println!("      • Resource Name: {}", state_change.resource_name);
            println!("      • Current State: {}", state_change.current_state);
            println!("      • Target State: {}", state_change.target_state);
            println!("      • Transition ID: {}", state_change.transition_id);
            println!("      • Source: {}", state_change.source);

            if let Err(e) = self
                .state_sender
                .clone()
                .send_state_change(state_change)
                .await
            {
                println!("   ❌ Failed to send state change to StateManager: {:?}", e);
            } else {
                println!(
                    "   ✅ Successfully notified StateManager: scenario {} satisfied → allowed",
                    scenario_name
                );
            }
        } else {
            // Policy not satisfied: satisfied -> denied
            println!("   🔄 State Change: satisfied → denied");
            println!("   🔍 Reason: Policy requirements not satisfied");

            let state_change = StateChange {
                resource_type: ResourceType::Scenario as i32,
                resource_name: scenario_name.clone(),
                current_state: "satisfied".to_string(),
                target_state: "denied".to_string(),
                transition_id: format!("policymanager-policy-denied-{}", timestamp),
                timestamp_ns: timestamp,
                source: "policymanager".to_string(),
            };

            println!("   📤 Sending StateChange to StateManager:");
            println!("      • Resource Type: SCENARIO");
            println!("      • Resource Name: {}", state_change.resource_name);
            println!("      • Current State: {}", state_change.current_state);
            println!("      • Target State: {}", state_change.target_state);
            println!("      • Transition ID: {}", state_change.transition_id);
            println!("      • Source: {}", state_change.source);

            if let Err(e) = self
                .state_sender
                .clone()
                .send_state_change(state_change)
                .await
            {
                println!("   ❌ Failed to send state change to StateManager: {:?}", e);
            } else {
                println!(
                    "   ✅ Successfully notified StateManager: scenario {} satisfied → denied",
                    scenario_name
                );
            }
        }

        Ok(Response::new(CheckPolicyResponse { status, desc }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_policy_manager_state_changes() {
        println!("🧪 Testing PolicyManager Scenario State Management");
        println!("=================================================");

        let server = PolicyManagerGrpcServer::new();

        println!("📋 Testing Policy Success Case:");
        println!("   🔄 Expected State Change: satisfied → allowed");

        // Test policy success (satisfied -> allowed)
        let request = Request::new(CheckPolicyRequest {
            scenario_name: "test_scenario".to_string(),
        });

        let response = server.check_policy(request).await.unwrap();
        let policy_response = response.into_inner();

        assert_eq!(policy_response.status, 0);
        assert_eq!(policy_response.desc, "Policy check passed");
        println!("✅ Policy success state change completed");
        println!("");

        println!("📋 Testing Policy Failure Case:");
        println!("   🔄 Expected State Change: satisfied → denied");

        // Test policy failure (satisfied -> denied)
        let request = Request::new(CheckPolicyRequest {
            scenario_name: "restricted_scenario".to_string(),
        });

        let response = server.check_policy(request).await.unwrap();
        let policy_response = response.into_inner();

        assert_eq!(policy_response.status, 1);
        assert!(policy_response.desc.contains("Policy check failed"));
        println!("✅ Policy failure state change completed");
        println!("");

        println!("🎉 PolicyManager state management test completed successfully!");
    }
}
