/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::statemanager::{
    connect_server, state_manager_connection_client::StateManagerConnectionClient, Action, Response,
};
use tonic::{Request, Status};

pub async fn _send(condition: Action) -> Result<tonic::Response<Response>, Status> {
    let mut client = StateManagerConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.send_action(Request::new(condition)).await
}
