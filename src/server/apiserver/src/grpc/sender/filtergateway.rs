/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::filtergateway::{
    connect_server, filter_gateway_connection_client::FilterGatewayConnectionClient,
    RegisterScenarioRequest, RegisterScenarioResponse,
};
use tonic::{Request, Response, Status};

pub async fn send(
    condition: RegisterScenarioRequest,
) -> Result<Response<RegisterScenarioResponse>, Status> {
    let mut client = FilterGatewayConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.register_scenario(Request::new(condition)).await
}
