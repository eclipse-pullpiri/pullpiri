/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::actioncontroller::{
    action_controller_connection_client::ActionControllerConnectionClient, connect_server,
    ReconcileRequest, ReconcileResponse,
};
use tonic::{Request, Response, Status};

pub async fn _send(condition: ReconcileRequest) -> Result<Response<ReconcileResponse>, Status> {
    let mut client = ActionControllerConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.reconcile(Request::new(condition)).await
}
