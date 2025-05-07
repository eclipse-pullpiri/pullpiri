use std::sync::Arc;
use tonic::{Request, Response, Status};

// Import the generated protobuf code
use common::actioncontroller::{
    action_controller_connection_server::{
        ActionControllerConnection, ActionControllerConnectionServer,
    },
    ReconcileRequest, ReconcileResponse, Status as ActionStatus, TriggerActionRequest,
    TriggerActionResponse,
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
        let scenario_name = request.into_inner().scenario_name;

        match self.manager.trigger_manager_action(&scenario_name).await {
            Ok(_) => Ok(Response::new(TriggerActionResponse {
                status: 0, // Success
                desc: "Action triggered successfully".to_string(),
            })),
            Err(e) => Ok(Response::new(TriggerActionResponse {
                status: 1, // Error
                desc: format!("Failed to trigger action: {}", e),
            })),
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
            Err(e) => Ok(Response::new(ReconcileResponse {
                status: 1, // Error
                desc: format!("Failed to reconcile: {}", e),
            })),
        }
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
    use common::actioncontroller::{TriggerActionRequest, ReconcileRequest};
    use tonic::Request;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_reconcile_success_when_states_differ() {
        // Pre-populate etcd keys
        common::etcd::put("scenario/test_scenario", r#"
        targets: test_package
        actions: launch
        "#).await.unwrap();

        common::etcd::put("package/test_package", r#"
        models:
        - name: model1
            node: node1  # This node will be skipped, but that’s okay because desired != Running
        "#).await.unwrap();

        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        let request = Request::new(ReconcileRequest {
            scenario_name: "test_scenario".to_string(),
            current: common::actioncontroller::Status::Init as i32,
            desired: common::actioncontroller::Status::Ready as i32, // NOT Running → workloads skipped
        });

        let response = receiver.reconcile(request).await.unwrap();

        assert_eq!(response.get_ref().status, 0);
        assert_eq!(
            response.get_ref().desc,
            "Reconciliation completed successfully"
        );
    }

    #[tokio::test]
    async fn test_trigger_action_failure() {
        let manager = Arc::new(ActionControllerManager::new());
        let receiver = ActionControllerReceiver::new(manager.clone());

        let request = Request::new(TriggerActionRequest {
            scenario_name: "invalid_scenario".to_string(),
        });

        let response = receiver.trigger_action(request).await.unwrap();
        assert_eq!(response.get_ref().status, 1);
        assert!(
            response.get_ref().desc.contains("Failed to trigger action")
        );
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
    
        let request = Request::new(TriggerActionRequest {
            scenario_name: "test_scenario".to_string(),
        });
    
        let response = receiver.trigger_action(request).await.unwrap();
        assert_eq!(response.get_ref().status, 0);
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

        let response = receiver.reconcile(request).await.unwrap();
        assert_eq!(response.get_ref().status, 1);
        assert!(
            response.get_ref().desc.contains("Failed to reconcile")
        );
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
