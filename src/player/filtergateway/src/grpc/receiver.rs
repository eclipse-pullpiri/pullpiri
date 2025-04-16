/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::filtergateway::filter_gateway_connection_server::FilterGatewayConnection;
use common::filtergateway::{HandleScenarioRequest, HandleScenarioResponse};

pub struct FilterGatewayGrpcServer {}

#[tonic::async_trait]
impl FilterGatewayConnection for FilterGatewayGrpcServer {
    async fn handle_scenario(
        &self,
        request: tonic::Request<HandleScenarioRequest>,
    ) -> Result<tonic::Response<HandleScenarioResponse>, tonic::Status> {
        let req = request.into_inner();
        let command = req.scenario;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }
}
