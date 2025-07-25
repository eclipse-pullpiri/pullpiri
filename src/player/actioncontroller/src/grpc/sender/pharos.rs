/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Running gRPC message sending to pharos

use common::pharos_service::{
    connect_pharos_server,
    pharos_network_service_connection_client::PharosNetworkServiceConnectionClient,
    RequestNetworkPodRequest, RequestNetworkPodResponse,
};

use std::collections::HashMap;
use tonic::{Request, Response, Status};

/// Send request to Pharos to set up network for a pod
///
/// ### Parameters
/// * `node_yaml` - YAML representation of the node
/// * `pod_name` - Name of the pod
/// * `network_yamls` - Map of network configurations
/// ### Returns
/// * `Result<Response<RequestNetworkPodResponse>, Status>` - Response from Pharos
/// ### Description
/// Connects to Pharos service and requests network configuration for a pod
pub async fn request_network_pod(
    node_yaml: String,
    pod_name: String,
    network_yamls: String,
) -> Result<Response<RequestNetworkPodResponse>, Status> {
    println!("Connecting to Pharos server ....");
    // Create the request
    let request = RequestNetworkPodRequest {
        node_yaml,
        pod_name,
        network_yamls,
    };
    let mut client = PharosNetworkServiceConnectionClient::connect(connect_pharos_server())
        .await
        .unwrap();
    client.request_network_pod(Request::new(request)).await
}
