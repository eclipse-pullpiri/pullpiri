/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! gRPC receiver implementing `ResourceManagerConnection`.

use std::sync::Arc;
use tonic::{Request, Response, Status};

use common::resourcemanager::resource_manager_connection_server::ResourceManagerConnection;
use common::resourcemanager::{
    GetResourceStateRequest, GetResourceStateResponse, ReportActualStateRequest,
    ReportActualStateResponse, ResourceSyncState, UpdateDesiredStateRequest,
    UpdateDesiredStateResponse, ValidateResourceUpdateRequest, ValidateResourceUpdateResponse,
};

use crate::manager::ResourceManagerManager;

pub struct ResourceManagerReceiver {
    manager: Arc<ResourceManagerManager>,
}

impl ResourceManagerReceiver {
    pub fn new(manager: Arc<ResourceManagerManager>) -> Self {
        Self { manager }
    }
}

#[tonic::async_trait]
impl ResourceManagerConnection for ResourceManagerReceiver {
    async fn validate_resource_update(
        &self,
        request: Request<ValidateResourceUpdateRequest>,
    ) -> Result<Response<ValidateResourceUpdateResponse>, Status> {
        let req = request.into_inner();
        let result =
            self.manager
                .validate(&req.workload_id, req.requested_cpu, req.requested_memory);

        println!(
            "[ResourceManager] validate workload='{}' cpu={} mem={}MiB -> allowed={} ({})",
            req.workload_id, req.requested_cpu, req.requested_memory, result.allowed, result.reason
        );

        Ok(Response::new(ValidateResourceUpdateResponse {
            allowed: result.allowed,
            reason: result.reason,
            available_cpu: result.available_cpu,
            available_memory: result.available_memory,
        }))
    }

    async fn update_desired_state(
        &self,
        request: Request<UpdateDesiredStateRequest>,
    ) -> Result<Response<UpdateDesiredStateResponse>, Status> {
        let req = request.into_inner();
        let sync_state =
            self.manager
                .update_desired(&req.workload_id, req.cpu_limit, req.memory_limit);

        println!(
            "[ResourceManager] desired state set workload='{}' cpu={} mem={}MiB",
            req.workload_id, req.cpu_limit, req.memory_limit
        );

        Ok(Response::new(UpdateDesiredStateResponse {
            success: true,
            sync_state,
        }))
    }

    async fn report_actual_state(
        &self,
        request: Request<ReportActualStateRequest>,
    ) -> Result<Response<ReportActualStateResponse>, Status> {
        let req = request.into_inner();
        let (synchronized, sync_state, drift_detected) = self.manager.report_actual(
            &req.workload_id,
            req.cpu_limit,
            req.memory_limit,
            req.update_success,
        );

        println!(
            "[ResourceManager] actual state workload='{}' cpu={} mem={}MiB success={} -> synced={} drift={}",
            req.workload_id, req.cpu_limit, req.memory_limit, req.update_success, synchronized, drift_detected
        );

        Ok(Response::new(ReportActualStateResponse {
            synchronized,
            sync_state,
            drift_detected,
        }))
    }

    async fn get_resource_state(
        &self,
        request: Request<GetResourceStateRequest>,
    ) -> Result<Response<GetResourceStateResponse>, Status> {
        let req = request.into_inner();
        let resp = match self.manager.get_state(&req.workload_id) {
            Some(st) => GetResourceStateResponse {
                found: true,
                desired_cpu: st.desired_cpu,
                desired_memory: st.desired_memory,
                actual_cpu: st.actual_cpu,
                actual_memory: st.actual_memory,
                sync_state: st.sync_state,
            },
            None => GetResourceStateResponse {
                found: false,
                sync_state: ResourceSyncState::SyncUnknown as i32,
                ..Default::default()
            },
        };
        Ok(Response::new(resp))
    }
}
