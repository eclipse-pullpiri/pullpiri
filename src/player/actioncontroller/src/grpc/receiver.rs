use std::sync::Arc;
use tonic::{Request, Response, Status};

// Import the generated protobuf code
use common::actioncontroller::{
    action_controller_connection_server::{
        ActionControllerConnection, ActionControllerConnectionServer,
    },
    CompleteNetworkSettingRequest, CompleteNetworkSettingResponse, ReconcileRequest,
    ReconcileResponse, Status as ActionStatus, TriggerActionRequest, TriggerActionResponse,
};

/// Receiver for handling incoming gRPC requests for ActionController
///
/// Implements the ActionControllerConnection gRPC service defined in
/// the protobuf specification. Handles incoming requests from:
/// - FilterGateway (trigger_action)
/// - StateManager (reconcile)
pub struct ActionControllerReceiver {
    /// Reference to the ActionController manager
    manager: Arc<crate::manager::ActionControllerManager>,
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
        Self { manager }
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
        // TODO: Implementation
        println!("trigger_action in grpc receiver");

        let scenario_name = request.into_inner().scenario_name;
        println!("trigger_action scenario: {}", scenario_name);

        match self.manager.trigger_manager_action(&scenario_name).await {
            Ok(_) => Ok(Response::new(TriggerActionResponse {
                status: 0,
                desc: "Action triggered successfully".to_string(),
            })),

            Err(e) => {
                let err_msg = e.to_string();

                // Decide gRPC error code based on error content
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
                    // fallback to Unknown error code
                    Status::unknown(err_msg)
                };

                // Return the gRPC error status instead of success response with error status code
                Err(grpc_status)
            }
        }
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
            "CompleteNetworkSettingRequest: request_id={}, status={:?}, details={}",
            req.request_id, req.status, req.details
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
    use crate::manager::ActionControllerManager;
    use common::actioncontroller::{ReconcileRequest, TriggerActionRequest};
    use std::sync::Arc;
    use tonic::Request;

    #[tokio::test]
    async fn test_reconcile_success_when_states_differ() {
        // Pre-populate etcd keys

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

        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        let request = Request::new(ReconcileRequest {
            scenario_name: "antipinch-enable".to_string(),
            current: common::actioncontroller::Status::Init as i32, // This is 1
            desired: common::actioncontroller::Status::Ready as i32, // This is 2
        });

        let response_result = receiver.reconcile(request).await;

        let response = response_result.unwrap();
        assert_eq!(
            response.get_ref().status,
            0,
            "Expected status 0 (success), got {}",
            response.get_ref().status
        );
        assert_eq!(
            response.get_ref().desc,
            "Reconciliation completed successfully",
            "Expected success message, got: '{}'",
            response.get_ref().desc
        );
        common::etcd::delete("scenario/antipinch-enable")
            .await
            .unwrap();
        common::etcd::delete("package/antipinch-enable")
            .await
            .unwrap();
    }

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
}
