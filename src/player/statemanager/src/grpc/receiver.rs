/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::statemanager::state_manager_connection_server::StateManagerConnection;
use common::monitoringserver::{ContainerList, SendContainerListResponse};
use common::statemanager::Action;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// StateManager gRPC service handler
#[derive(Clone)]
pub struct StateManagerReceiver {
    pub tx: mpsc::Sender<ContainerList>,
}

#[tonic::async_trait]
impl StateManagerConnection for StateManagerReceiver {
    async fn send_action(
        &self,
        request: tonic::Request<Action>,
    ) -> Result<tonic::Response<common::statemanager::Response>, tonic::Status> {
        let req = request.into_inner();
        let command = req.action;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }

    /// Handle a ContainerList message from nodeagent
    ///
    /// Receives a ContainerList from nodeagent and forwards it to the StateManager manager for processing.
    async fn send_changed_container_list<'life>(
        &'life self,
        request: Request<ContainerList>,
    ) -> Result<Response<SendContainerListResponse>, Status> {
        let req: ContainerList = request.into_inner();

        match self.tx.send(req).await {
            Ok(_) => Ok(tonic::Response::new(SendContainerListResponse {
                resp: "Successfully processed ContainerList".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send changed container list: {}", e),
            )),
        }
    }
}