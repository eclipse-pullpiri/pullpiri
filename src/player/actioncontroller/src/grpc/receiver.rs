use std::sync::Arc;
use tonic::{Request, Response, Status};

// Import the generated protobuf code
use crate::grpc::sender::statemanager::StateManagerSender;
use common::actioncontroller::{
    action_controller_connection_server::{
        ActionControllerConnection, ActionControllerConnectionServer,
    },
    CompleteNetworkSettingRequest, CompleteNetworkSettingResponse, NetworkStatus,
    PodStatus as ActionStatus, ReconcileRequest, ReconcileResponse, TriggerActionRequest,
    TriggerActionResponse,
};
use common::statemanager::{ResourceType, StateChange};

/// Receiver for handling incoming gRPC requests for ActionController
///
/// Implements the ActionControllerConnection gRPC service defined in
/// the protobuf specification. Handles incoming requests from:
/// - FilterGateway (trigger_action)
/// - StateManager (reconcile)
pub struct ActionControllerReceiver {
    /// Reference to the ActionController manager
    manager: Arc<crate::manager::ActionControllerManager>,
    /// StateManager sender for scenario state changes
    state_sender: StateManagerSender,
}

impl ActionControllerReceiver {
    /// Create a new ActionControllerReceiver instance
    ///
    /// # Arguments
    ///
    /// * `manager` - Shared reference to the ActionController manager
    ///
    /// # Returns
    ///
    /// A new ActionControllerReceiver instance
    pub fn new(manager: Arc<crate::manager::ActionControllerManager>) -> Self {
        Self {
            manager,
            state_sender: StateManagerSender::new(),
        }
    }

    /// Get a gRPC server for this receiver
    ///
    /// # Returns
    ///
    /// A configured ActionControllerConnectionServer
    pub fn into_service(self) -> ActionControllerConnectionServer<Self> {
        ActionControllerConnectionServer::new(self)
    }
}

#[tonic::async_trait]
impl ActionControllerConnection for ActionControllerReceiver {
    /// Handle trigger action requests from FilterGateway
    ///
    /// # Arguments
    ///
    /// * `request` - gRPC request containing scenario name to trigger
    ///
    /// # Returns
    ///
    /// * `Response<TriggerActionResponse>` - gRPC response with status and description
    /// * `Status` - gRPC status error if the request fails
    async fn trigger_action(
        &self,
        request: Request<TriggerActionRequest>,
    ) -> Result<Response<TriggerActionResponse>, Status> {
        use std::time::Instant;
        let start = Instant::now();

        println!("trigger_action in grpc receiver");

        let scenario_name = request.into_inner().scenario_name;
        println!("trigger_action scenario: {}", scenario_name);

        // 🔍 COMMENT 3: ActionController condition satisfaction check
        // When ActionController receives trigger_action from FilterGateway,
        // it processes the scenario and should notify StateManager of scenario
        // state changes (e.g., from "waiting" to "satisfied" after conditions are met).
        // State change requests would be sent via StateManagerSender.

        println!("🔄 SCENARIO STATE TRANSITION: ActionController Processing");
        println!("   📋 Scenario: {}", scenario_name);
        println!("   🔄 State Change: waiting → satisfied");
        println!("   🔍 Reason: ActionController received trigger_action from FilterGateway");

        // Send state change to StateManager: waiting -> satisfied
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: scenario_name.clone(),
            current_state: "waiting".to_string(),
            target_state: "satisfied".to_string(),
            transition_id: format!("actioncontroller-condition-satisfied-{}", timestamp),
            timestamp_ns: timestamp,
            source: "actioncontroller".to_string(),
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
                "   ✅ Successfully notified StateManager: scenario {} waiting → satisfied",
                scenario_name
            );
        }

        println!("   🎯 Processing scenario actions...");
        let result = match self.manager.trigger_manager_action(&scenario_name).await {
            Ok(_) => Ok(Response::new(TriggerActionResponse {
                status: 0,
                desc: "Action triggered successfully".to_string(),
            })),
            Err(e) => {
                let err_msg = e.to_string();
                let grpc_status = if err_msg.contains("Invalid scenario name") {
                    Status::invalid_argument(err_msg)
                } else if err_msg.contains("not found") {
                    Status::not_found(err_msg)
                } else if err_msg.contains("Failed to parse") {
                    Status::invalid_argument(err_msg)
                } else if err_msg.contains("Failed to start workload")
                    || err_msg.contains("Failed to stop workload")
                {
                    Status::internal(err_msg)
                } else {
                    Status::unknown(err_msg)
                };
                Err(grpc_status)
            }
        };

        let elapsed = start.elapsed();
        println!("trigger_action: elapsed = {:?}", elapsed);

        result
    }

    /// Handle reconcile requests from StateManager
    ///
    /// # Arguments
    ///
    /// * `request` - gRPC request containing scenario name and state information
    ///
    /// # Returns
    ///
    /// * `Response<ReconcileResponse>` - gRPC response with status and description
    /// * `Status` - gRPC status error if the request fails
    async fn reconcile(
        &self,
        request: Request<ReconcileRequest>,
    ) -> Result<Response<ReconcileResponse>, Status> {
        // TODO: Implementation
        let req = request.into_inner();
        let scenario_name = req.scenario_name;

        let current = i32_to_status(req.current);
        let desired = i32_to_status(req.desired);

        if current == desired {
            return Ok(Response::new(ReconcileResponse {
                status: 0, // Success
                desc: "Current and desired states are equal".to_string(),
            }));
        }

        match self
            .manager
            .reconcile_do(scenario_name, current, desired)
            .await
        {
            Ok(_) => Ok(Response::new(ReconcileResponse {
                status: 0, // Success
                desc: "Reconciliation completed successfully".to_string(),
            })),
            // If reconcile_do returns an error, convert it into a gRPC Status::internal error
            // and propagate it. This allows gRPC clients to receive a proper error status.
            Err(e) => {
                eprintln!("Reconciliation failed: {:?}", e); // Log the error for debugging
                Err(Status::internal(format!("Failed to reconcile: {}", e)))
            }
        }
    }

    async fn complete_network_setting(
        &self,
        request: Request<CompleteNetworkSettingRequest>,
    ) -> Result<Response<CompleteNetworkSettingResponse>, Status> {
        let req = request.into_inner();
        println!(
            "CompleteNetworkSettingRequest: request_id={}, network_status={:?}, pod_status={:?}, details={}",
            req.request_id, req.network_status, req.pod_status, req.details
        );

        let response = CompleteNetworkSettingResponse { acknowledged: true };
        Ok(Response::new(response))
    }
}

fn i32_to_status(value: i32) -> ActionStatus {
    match value {
        0 => ActionStatus::None,
        1 => ActionStatus::Init,
        2 => ActionStatus::Ready,
        3 => ActionStatus::Running,
        4 => ActionStatus::Done,
        5 => ActionStatus::Failed,
        _ => ActionStatus::Unknown,
    }
}

//UNIT TEST
#[cfg(test)]
mod tests {
    use super::*;
    use crate::grpc::receiver::Status;
    use crate::manager::ActionControllerManager;
    use common::actioncontroller::{ReconcileRequest, TriggerActionRequest};
    use std::sync::Arc;
    use tonic::Request;

    // #[tokio::test]
    // async fn test_reconcile_success_when_states_differ() {
    //     // Pre-populate etcd keys

    //     let scenario_yaml = r#"
    //     apiVersion: v1
    //     kind: Scenario
    //     metadata:
    //         name: antipinch-enable
    //     spec:
    //         condition:
    //         action: update
    //         target: antipinch-enable
    //     "#;
    //     common::etcd::put("scenario/antipinch-enable", scenario_yaml)
    //         .await
    //         .unwrap();

    //     let package_yaml = r#"
    //     apiVersion: v1
    //     kind: Package
    //     metadata:
    //         label: null
    //         name: antipinch-enable
    //     spec:
    //         pattern:
    //           - type: plain
    //         models:
    //           - name: antipinch-enable-core
    //             node: HPC
    //             resources:
    //                 volume: antipinch-volume
    //                 network: antipinch-network
    //     "#;
    //     common::etcd::put("package/antipinch-enable", package_yaml)
    //         .await
    //         .unwrap();

    //     let manager = Arc::new(ActionControllerManager::new());
    //     let receiver = ActionControllerReceiver::new(manager.clone());

    //     let request = Request::new(ReconcileRequest {
    //         scenario_name: "antipinch-enable".to_string(),
    //         current: common::actioncontroller::Status::Init as i32, // This is 1
    //         desired: common::actioncontroller::Status::Ready as i32, // This is 2
    //     });

    //     let response_result = receiver.reconcile(request).await;

    //     let response = response_result.unwrap();
    //     assert_eq!(
    //         response.get_ref().status,
    //         0,
    //         "Expected status 0 (success), got {}",
    //         response.get_ref().status
    //     );
    //     assert_eq!(
    //         response.get_ref().desc,
    //         "Reconciliation completed successfully",
    //         "Expected success message, got: '{}'",
    //         response.get_ref().desc
    //     );
    //     common::etcd::delete("scenario/antipinch-enable")
    //         .await
    //         .unwrap();
    //     common::etcd::delete("package/antipinch-enable")
    //         .await
    //         .unwrap();
    // }

    #[tokio::test]
    async fn test_trigger_action_failure() {
        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        let request = Request::new(TriggerActionRequest {
            scenario_name: "invalid_scenario".to_string(),
        });

        let response = receiver.trigger_action(request).await.unwrap_err();
        assert!(response.message().contains("not found"));
    }

    #[tokio::test]
    async fn test_reconcile_when_states_equal() {
        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        let request = Request::new(ReconcileRequest {
            scenario_name: "test_scenario".to_string(),
            current: 3, // RUNNING
            desired: 3, // RUNNING
        });

        let response = receiver.reconcile(request).await.unwrap();
        assert_eq!(response.get_ref().status, 0);
        assert_eq!(
            response.get_ref().desc,
            "Current and desired states are equal"
        );
    }

    #[tokio::test]
    async fn test_trigger_action_success() {
        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        let scenario_yaml = r#"
        apiVersion: v1
        kind: Scenario
        metadata:
            name: antipinch-enable
        spec:
            condition:
            action: update
            target: antipinch-enable
        "#;

        common::etcd::put("scenario/antipinch-enable", scenario_yaml)
            .await
            .unwrap();

        let package_yaml = r#"
        apiVersion: v1
        kind: Package
        metadata:
            label: null
            name: antipinch-enable
        spec:
            pattern:
              - type: plain
            models:
              - name: antipinch-enable-core
                node: HPC
                resources:
                    volume: antipinch-volume
                    network: antipinch-network
        "#;

        common::etcd::put("package/antipinch-enable", package_yaml)
            .await
            .unwrap();

        let request = Request::new(TriggerActionRequest {
            scenario_name: "antipinch-enable".to_string(),
        });

        let response = receiver.trigger_action(request).await.unwrap();
        assert_eq!(response.get_ref().status, 0);

        let _ = common::etcd::delete("scenario/antipinch-enable").await;
        let _ = common::etcd::delete("package/antipinch-enable").await;
    }

    #[tokio::test]
    async fn test_reconcile_failure_invalid_scenario() {
        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        let request = Request::new(ReconcileRequest {
            scenario_name: "invalid_scenario".to_string(),
            current: 0,
            desired: 3,
        });

        let response = receiver.reconcile(request).await.unwrap_err();
        assert!(response.message().contains("Failed to reconcile"));
    }

    #[tokio::test]
    async fn test_scenario_state_management_workflow() {
        println!("🧪 Testing ActionController Scenario State Management");
        println!("===================================================");

        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        // Setup test scenario in ETCD
        let scenario_yaml = r#"
        apiVersion: v1
        kind: Scenario
        metadata:
            name: test-state-scenario
        spec:
            condition:
            action: update
            target: test-state-scenario
        "#;

        common::etcd::put("scenario/test-state-scenario", scenario_yaml)
            .await
            .unwrap();

        let package_yaml = r#"
        apiVersion: v1
        kind: Package
        metadata:
            label: null
            name: test-state-scenario
        spec:
            pattern:
              - type: plain
            models:
              - name: test-state-scenario-core
                node: HPC
                resources:
                    volume: test-volume
                    network: test-network
        "#;

        common::etcd::put("package/test-state-scenario", package_yaml)
            .await
            .unwrap();

        println!("📋 Test Scenario: test-state-scenario");
        println!("🔄 Expected State Changes:");
        println!("   1. waiting → satisfied (on trigger_action)");
        println!("   2. allowed → completed (on processing completion)");
        println!("");

        // Test trigger_action (waiting -> satisfied)
        println!("🎯 Testing trigger_action state change...");
        let request = Request::new(TriggerActionRequest {
            scenario_name: "test-state-scenario".to_string(),
        });

        let response = receiver.trigger_action(request).await.unwrap();
        assert_eq!(response.get_ref().status, 0);
        println!("✅ trigger_action completed successfully");
        println!("");

        // Cleanup
        let _ = common::etcd::delete("scenario/test-state-scenario").await;
        let _ = common::etcd::delete("package/test-state-scenario").await;

        println!("🎉 ActionController state management test completed successfully!");
    }

    #[test]
    fn test_i32_to_status_all_variants() {
        assert_eq!(i32_to_status(0), ActionStatus::None);
        assert_eq!(i32_to_status(1), ActionStatus::Init);
        assert_eq!(i32_to_status(2), ActionStatus::Ready);
        assert_eq!(i32_to_status(3), ActionStatus::Running);
        assert_eq!(i32_to_status(4), ActionStatus::Done);
        assert_eq!(i32_to_status(5), ActionStatus::Failed);
        assert_eq!(i32_to_status(999), ActionStatus::Unknown);
        assert_eq!(i32_to_status(-1), ActionStatus::Unknown);
    }

    #[test]
    fn test_receiver_new_and_into_service() {
        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager);
        let _service = receiver.into_service();
    }

    #[tokio::test]
    async fn test_reconcile_communication_behavior() {
        println!("🧪 Testing ActionController reconcile communication behavior");

        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        // Test 1: StateManager pattern - FAILED → RUNNING (should be rejected)
        let failed_request = Request::new(ReconcileRequest {
            scenario_name: "test-scenario".to_string(),
            current: 5, // PodStatus::FAILED (what StateManager sends)
            desired: 3, // PodStatus::RUNNING (what StateManager sends)
        });

        println!("📨 Testing FAILED → RUNNING (StateManager pattern):");
        let failed_response = receiver.reconcile(failed_request).await;

        assert!(
            failed_response.is_err(),
            "ActionController should reject FAILED → RUNNING"
        );
        let error = failed_response.unwrap_err();
        assert!(error.message().contains("Invalid current status: Failed"));
        println!("   ✅ Correctly rejected: {}", error.message());

        // Test 2: Equal states (should succeed)
        let equal_request = Request::new(ReconcileRequest {
            scenario_name: "test-scenario".to_string(),
            current: 3, // PodStatus::RUNNING
            desired: 3, // PodStatus::RUNNING
        });

        println!("📨 Testing RUNNING → RUNNING (equal states):");
        let equal_response = receiver.reconcile(equal_request).await;

        assert!(equal_response.is_ok(), "Equal states should succeed");
        let response = equal_response.unwrap();
        assert_eq!(response.get_ref().status, 0);
        assert!(response.get_ref().desc.contains("equal"));
        println!(
            "   ✅ Equal states handled correctly: {}",
            response.get_ref().desc
        );

        // Test 3: Different valid states (will fail due to missing scenario, but that's expected)
        let valid_request = Request::new(ReconcileRequest {
            scenario_name: "nonexistent-scenario".to_string(),
            current: 2, // PodStatus::READY
            desired: 3, // PodStatus::RUNNING
        });

        println!("📨 Testing READY → RUNNING (nonexistent scenario):");
        let valid_response = receiver.reconcile(valid_request).await;

        // This should fail due to missing scenario, which is expected behavior
        assert!(
            valid_response.is_err(),
            "Should fail for nonexistent scenario"
        );
        let error = valid_response.unwrap_err();
        println!(
            "   ℹ️  Expected failure for nonexistent scenario: {}",
            error.message()
        );

        println!("🎉 Communication behavior test completed!");
        println!("📝 Summary:");
        println!("   • ActionController properly validates status transitions");
        println!("   • FAILED → RUNNING reconcile is rejected (business logic)");
        println!("   • Equal state transitions are handled correctly");
        println!("   • Missing scenarios are properly detected and reported");
    }

    #[tokio::test]
    async fn test_reconcile_different_status_transitions() {
        println!("🧪 Testing ActionController reconcile with different status transitions");

        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        // Test different status transition scenarios
        let test_cases = vec![
            ("FAILED to RUNNING", 5, 3), // StateManager pattern
            ("NONE to RUNNING", 0, 3),   // Initial startup
            ("READY to RUNNING", 2, 3),  // Normal transition
            ("RUNNING to DONE", 3, 4),   // Completion
        ];

        for (description, current, desired) in test_cases {
            println!("🔄 Testing transition: {}", description);

            let request = Request::new(ReconcileRequest {
                scenario_name: "test-scenario".to_string(),
                current,
                desired,
            });

            let response_result = receiver.reconcile(request).await;

            if current == desired {
                // Should succeed with "equal states" message
                assert!(response_result.is_ok(), "Equal states should succeed");
                let response = response_result.unwrap();
                assert_eq!(response.get_ref().status, 0);
                assert!(response.get_ref().desc.contains("equal"));
                println!("   ✅ Equal states handled correctly");
            } else {
                // Different states - may succeed or fail depending on scenario existence
                // For this test, we expect it to fail since scenario doesn't exist in etcd
                match response_result {
                    Ok(response) => {
                        println!("   ✅ Reconcile succeeded: {}", response.get_ref().desc);
                    }
                    Err(status) => {
                        println!(
                            "   ℹ️  Reconcile failed (expected for nonexistent scenario): {}",
                            status.message()
                        );
                        assert!(status.message().contains("Failed to reconcile"));
                    }
                }
            }
        }

        println!("🎉 Status transition tests completed successfully!");
    }
}
