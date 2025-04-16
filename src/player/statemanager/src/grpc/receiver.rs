/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::statemanager::state_manager_connection_server::StateManagerConnection;
use common::statemanager::{Action, Response};

pub struct StateManagerGrpcServer {}

#[tonic::async_trait]
impl StateManagerConnection for StateManagerGrpcServer {
    async fn send_action(
        &self,
        request: tonic::Request<Action>,
    ) -> Result<tonic::Response<Response>, tonic::Status> {
        let req = request.into_inner();
        let command = req.action;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }
}
