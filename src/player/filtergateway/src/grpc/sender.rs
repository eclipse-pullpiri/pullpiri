/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::actioncontroller::{
    action_controller_connection_client::ActionControllerConnectionClient, connect_server,
    TriggerActionRequest, TriggerActionResponse,
};
use tonic::{Request, Response, Status};

pub async fn _send(
    condition: TriggerActionRequest,
) -> Result<Response<TriggerActionResponse>, Status> {
    let mut client = ActionControllerConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.trigger_action(Request::new(condition)).await
}
