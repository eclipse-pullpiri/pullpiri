/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Running gRPC message sending to filtergateway

use common::filtergateway::{
    connect_server, filter_gateway_connection_client::FilterGatewayConnectionClient,
    HandleScenarioRequest, HandleScenarioResponse,
};
use tonic::{Request, Response, Status};

/// Send scenario information to filtergateway via gRPC
///
/// ### Parametets
/// * `scenario: HandleScenarioRequest` - wrapped scenario information
/// ### Description
/// This is generated almost automatically by `tonic_build`, so you
/// don't need to modify it separately.
pub async fn send(
    scenario: HandleScenarioRequest,
) -> Result<Response<HandleScenarioResponse>, Status> {
    let mut client = FilterGatewayConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.handle_scenario(Request::new(scenario)).await
}
