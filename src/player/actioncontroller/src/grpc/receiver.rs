use common::Result;
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
    ) -> std::result::Result<Response<TriggerActionResponse>, Status> {
        // TODO: Implementation
        let scenario_name = request.into_inner().scenario_name;

        match self.manager.trigger_manager_action(scenario_name).await {
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
    ) -> std::result::Result<Response<ReconcileResponse>, Status> {
        // TODO: Implementation
        let req = request.into_inner();
        let scenario_name = req.scenario_name;
        let current = req.current;
        let desired = req.desired;

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
