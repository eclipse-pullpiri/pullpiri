/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::gateway::{connect_server, connection_client::ConnectionClient, Condition, Response};
use tonic::{Request, Status};

pub async fn send(condition: Condition) -> Result<tonic::Response<Response>, Status> {
    let mut client = ConnectionClient::connect(connect_server()).await.unwrap();
    client.send_condition(Request::new(condition)).await
}
