/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::nodeagent::node_agent_connection_server::NodeAgentConnection;
use common::nodeagent::{HandleWorkloadRequest, HandleWorkloadResponse};

pub struct NodeAgentGrpcServer {}

#[tonic::async_trait]
impl NodeAgentConnection for NodeAgentGrpcServer {
    async fn handle_workload(
        &self,
        request: tonic::Request<HandleWorkloadRequest>,
    ) -> Result<tonic::Response<HandleWorkloadResponse>, tonic::Status> {
        let req = request.into_inner();
        let command = req.workload_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }
}
