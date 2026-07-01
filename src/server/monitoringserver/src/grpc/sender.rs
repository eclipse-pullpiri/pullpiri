/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! gRPC sender for MonitoringServer to communicate with PolicyManager

use common::monitoringserver::NodeInfo;
use common::policymanager::policy_manager_connection_client::PolicyManagerConnectionClient;
use common::policymanager::{
    connect_server, ReportNodeMetricsRequest, ReportNodeMetricsResponse, RunningContainer,
};
use tonic::{Request, Response, Status};

/// Send node metrics to PolicyManager for threshold-based policy evaluation
pub async fn report_node_metrics(
    node_info: NodeInfo,
    running_containers: Vec<RunningContainer>,
) -> Result<Response<ReportNodeMetricsResponse>, Status> {
    let addr = connect_server();

    let client = PolicyManagerConnectionClient::connect(addr).await;

    match client {
        Ok(mut client) => {
            let request = ReportNodeMetricsRequest {
                node_info: Some(node_info),
                running_containers,
            };
            client.report_node_metrics(Request::new(request)).await
        }
        Err(e) => {
            // Log but don't fail - PolicyManager might not be running
            eprintln!(
                "[MonitoringServer] Failed to connect to PolicyManager: {}",
                e
            );
            Err(Status::unavailable(format!(
                "Failed to connect to PolicyManager: {}",
                e
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_report_node_metrics_connection_failure() {
        // Test that connection failure is handled gracefully
        let node_info = NodeInfo {
            node_name: "test-node".to_string(),
            cpu_usage: 50.0,
            cpu_count: 4,
            gpu_count: 1,
            used_memory: 4096,
            total_memory: 8192,
            mem_usage: 50.0,
            rx_bytes: 1000,
            tx_bytes: 2000,
            read_bytes: 3000,
            write_bytes: 4000,
            os: "Linux".to_string(),
            arch: "x86_64".to_string(),
            ip: "192.168.1.100".to_string(),
        };

        let result = report_node_metrics(node_info, vec![]).await;
        // Should fail because PolicyManager is not running
        assert!(result.is_err());
    }
}
