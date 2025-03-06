/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

pub mod policymanager {
    use common::policymanager::{
        connect_server, policy_manager_connection_client::PolicyManagerConnectionClient,
        CheckPolicyRequest, CheckPolicyResponse,
    };
    use tonic::{Request, Response, Status};

    pub async fn _send(
        condition: CheckPolicyRequest,
    ) -> Result<Response<CheckPolicyResponse>, Status> {
        let mut client = PolicyManagerConnectionClient::connect(connect_server())
            .await
            .unwrap();
        client.check_policy(Request::new(condition)).await
    }
}

pub mod nodeagent {
    use common::nodeagent::{
        connect_server, node_agent_connection_client::NodeAgentConnectionClient,
        HandleWorkloadRequest, HandleWorkloadResponse,
    };
    use tonic::{Request, Response, Status};

    pub async fn _send(
        condition: HandleWorkloadRequest,
    ) -> Result<Response<HandleWorkloadResponse>, Status> {
        let mut client = NodeAgentConnectionClient::connect(connect_server())
            .await
            .unwrap();
        client.handle_workload(Request::new(condition)).await
    }
}
