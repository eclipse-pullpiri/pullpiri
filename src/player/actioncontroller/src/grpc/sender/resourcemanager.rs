/*
 * SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Sender for the ResourceManager service.
//!
//! Used by the ActionController to validate resource availability, record the
//! desired resource state, and reconcile the actual runtime state during the
//! Dynamic Resource Scaling workflow (#514 / #526).

use common::resourcemanager::resource_manager_connection_client::ResourceManagerConnectionClient;
use common::resourcemanager::{
    ReportActualStateRequest, ReportActualStateResponse, UpdateDesiredStateRequest,
    UpdateDesiredStateResponse, ValidateResourceUpdateRequest, ValidateResourceUpdateResponse,
};
use tonic::{Request, Status};

async fn connect() -> Result<ResourceManagerConnectionClient<tonic::transport::Channel>, Status> {
    ResourceManagerConnectionClient::connect(common::resourcemanager::connect_server())
        .await
        .map_err(|e| Status::unavailable(format!("ResourceManager connect failed: {}", e)))
}

pub async fn validate_resource_update(
    request: ValidateResourceUpdateRequest,
) -> Result<ValidateResourceUpdateResponse, Status> {
    let mut client = connect().await?;
    Ok(client
        .validate_resource_update(Request::new(request))
        .await?
        .into_inner())
}

pub async fn update_desired_state(
    request: UpdateDesiredStateRequest,
) -> Result<UpdateDesiredStateResponse, Status> {
    let mut client = connect().await?;
    Ok(client
        .update_desired_state(Request::new(request))
        .await?
        .into_inner())
}

pub async fn report_actual_state(
    request: ReportActualStateRequest,
) -> Result<ReportActualStateResponse, Status> {
    let mut client = connect().await?;
    Ok(client
        .report_actual_state(Request::new(request))
        .await?
        .into_inner())
}
