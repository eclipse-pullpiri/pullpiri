/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::filtergateway::filter_gateway_connection_server::FilterGatewayConnection;
use common::filtergateway::{RegisterScenarioRequest, RegisterScenarioResponse};

pub struct FilterGatewayGrpcServer {}

#[tonic::async_trait]
impl FilterGatewayConnection for FilterGatewayGrpcServer {
    async fn register_scenario(
        &self,
        request: tonic::Request<RegisterScenarioRequest>,
    ) -> Result<tonic::Response<RegisterScenarioResponse>, tonic::Status> {
        let req = request.into_inner();
        let command = req.scenario_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }
}
