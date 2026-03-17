/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::nodeagent::fromactioncontroller::{HandleWorkloadRequest, WorkloadCommand};
use common::Result;
/// Runtime implementation for NodeAgent API interactions
///
/// Handles workload operations for nodes managed by NodeAgent,
/// making gRPC calls to the NodeAgent service to perform
/// operations like creating, starting, stopping, and deleting workloads.

pub async fn create_workload(pod: &str, node_name: &str) -> Result<()> {
    let cmd = WorkloadCommand::Create;
    handle_workload(cmd, pod, node_name).await?;
    Ok(())
}

pub async fn handle_workload(cmd: WorkloadCommand, pod: &str, node_name: &str) -> Result<()> {
    if let Some(addr) = get_node_name_from_hostname(node_name).await {
        println!("node_name: {}, addr: {}", node_name, addr);

        let request = HandleWorkloadRequest {
            workload_command: cmd.into(),
            pod: pod.to_string(),
        };
        crate::grpc::sender::nodeagent::send_workload_handle_request(&addr, request).await?;
    } else {
        println!("Node {} not found in DB", node_name);
        return Err(format!("Node {} not found in DB", node_name).into());
    }

    Ok(())
}

pub async fn start_workload(pod: &str, node_name: &str) -> Result<()> {
    let cmd = WorkloadCommand::Start;
    handle_workload(cmd, pod, node_name).await?;
    Ok(())
}

pub async fn stop_workload(pod: &str, node_name: &str) -> Result<()> {
    let cmd = WorkloadCommand::Stop;
    handle_workload(cmd, pod, node_name).await?;
    Ok(())
}

pub async fn restart_workload(pod: &str, node_name: &str) -> Result<()> {
    let cmd = WorkloadCommand::Restart;
    handle_workload(cmd, pod, node_name).await?;
    Ok(())
}

/// Find a node by IP address from simplified node keys
async fn get_node_name_from_hostname(hostname: &str) -> Option<String> {
    println!("Checking node keys in etcd...");
    match common::persistency::get(&format!("nodes/{}", hostname)).await {
        Ok(ip) => {
            println!("Found node IP: {}", ip);
            Some(ip)
        }
        Err(e) => {
            println!("Error checking nodes: {}", e);
            None
        }
    }
}

//UNIT TEST
#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    // ------------------------- create_workload() -------------------------

    #[tokio::test]
    async fn test_create_workload_returns_ok() {
        let result = create_workload("test_model", "test_node").await;
        assert!(result.is_ok(), "create_workload() should return Ok");
    }

    #[tokio::test]
    async fn test_create_workload_invalid_scenario_should_fail() {
        let result = create_workload("", "").await; // Empty scenario = invalid
        assert!(
            result.is_ok(),
            "TODO: expect Err once create_workload validates input"
        );
    }

    // ------------------------- restart_workload() -------------------------

    #[tokio::test]
    async fn test_restart_workload_returns_ok() {
        let result = restart_workload("test_model", "test_node").await;
        assert!(result.is_ok(), "restart_workload() should return Ok");
    }

    #[tokio::test]
    async fn test_restart_workload_nonexistent_should_fail() {
        let result = restart_workload("nonexistent_scenario", "test_node").await;
        assert!(
            result.is_ok(),
            "TODO: expect Err when workload does not exist"
        );
    }

    // ------------------------- start_workload() -------------------------

    #[tokio::test]
    async fn test_start_workload_returns_ok() {
        let result = start_workload("test_model", "test_node").await;
        assert!(result.is_ok(), "start_workload() should return Ok");
    }

    #[tokio::test]
    async fn test_start_workload_nonexistent_should_fail() {
        let result = start_workload("nonexistent_model", "test_node").await;
        assert!(
            result.is_ok(),
            "TODO: expect Err when workload does not exist"
        );
    }

    // ------------------------- stop_workload() -------------------------

    #[tokio::test]
    async fn test_stop_workload_returns_ok() {
        let result = stop_workload("test_model", "test_node").await;
        assert!(result.is_ok(), "stop_workload() should return Ok");
    }

    #[tokio::test]
    async fn test_stop_workload_nonexistent_should_fail() {
        let result = stop_workload("nonexistent_model", "test_node").await;
        assert!(
            result.is_ok(),
            "TODO: expect Err when workload does not exist"
        );
    }
}
