/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Running gRPC message sending to monitoringserver

use common::monitoringserver::{
    monitoring_server_connection_client::MonitoringServerConnectionClient, NodeLiveliness,
    NodeStatusNotification, NodeStatusNotificationResponse,
};
use tonic::{Request, Response, Status};

/// Notify the MonitoringServer that a node has disconnected.
///
/// This triggers the MonitoringServer to clear all metrics associated
/// with the given node from etcd.
///
/// ### Parameters
/// * `node_name: &str` - the name of the disconnected node
pub async fn notify_node_disconnected(
    node_name: &str,
) -> Result<Response<NodeStatusNotificationResponse>, Status> {
    let addr = common::monitoringserver::connect_server();
    let mut client = MonitoringServerConnectionClient::connect(addr)
        .await
        .map_err(|e| {
            Status::unavailable(format!("Failed to connect to MonitoringServer: {}", e))
        })?;

    let notification = NodeStatusNotification {
        node_name: node_name.to_string(),
        liveliness: NodeLiveliness::NodeDisconnected.into(),
    };

    client.notify_node_status(Request::new(notification)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notify_node_disconnected_returns_error_when_no_server() {
        // Without a running MonitoringServer, this should fail gracefully
        let result = notify_node_disconnected("test-node").await;
        assert!(
            result.is_err(),
            "Expected connection error when MonitoringServer is not running"
        );
    }
}
