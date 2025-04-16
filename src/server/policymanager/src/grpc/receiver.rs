/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::policymanager::policy_manager_connection_server::PolicyManagerConnection;
use common::policymanager::{CheckPolicyRequest, CheckPolicyResponse};

pub struct PolicyManagerGrpcServer {}

#[tonic::async_trait]
impl PolicyManagerConnection for PolicyManagerGrpcServer {
    async fn check_policy(
        &self,
        request: tonic::Request<CheckPolicyRequest>,
    ) -> Result<tonic::Response<CheckPolicyResponse>, tonic::Status> {
        let req = request.into_inner();
        let command = req.scenario_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }
}
