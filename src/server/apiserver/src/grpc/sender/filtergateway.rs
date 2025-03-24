/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::filtergateway::{
    connect_server, filter_gateway_connection_client::FilterGatewayConnectionClient,
    HandleScenarioRequest, HandleScenarioResponse,
};
use tonic::{Request, Response, Status};

pub async fn send(
    scenario: HandleScenarioRequest,
) -> Result<Response<HandleScenarioResponse>, Status> {
    let mut client = FilterGatewayConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.handle_scenario(Request::new(scenario)).await
}
