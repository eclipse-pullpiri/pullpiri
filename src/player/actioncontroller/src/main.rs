/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::logd;
use common::logd::logger;
use std::error::Error;

mod grpc;
mod manager;
mod runtime;

/// Initialize the ActionController component
///
/// Reads node information from `settings.yaml` file, distinguishes between
/// Bluechi nodes and NodeAgent nodes, and sets up the initial configuration
/// for the component to start processing workload orchestration requests.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration files cannot be read
/// - Node information is invalid
/// - gRPC server setup fails
async fn initialize(skip_grpc: bool) -> Result<(), Box<dyn Error>> {
    // 기본 설정 정보에서 노드 역할 확인
    let config = common::setting::get_config();
    let mut manager = manager::ActionControllerManager::new();

    // 설정 파일의 호스트 정보 확인 (노드 역할 사전 설정)
    let hostname = &config.host.name;
    let _node_type = &config.host.r#type;

    /*if node_type == "bluechi" {
        println!("Adding {} to bluechi_nodes from settings.yaml", hostname);
        manager.bluechi_nodes.push(hostname.clone());
    } else*/
    {
        logd!(
            2,
            "Adding {} to nodeagent_nodes from settings.yaml",
            hostname
        );
        manager.nodeagent_nodes.push(hostname.clone());
    }

    // gRPC 서버 초기화 (테스트 모드가 아닌 경우)
    if !skip_grpc {
        grpc::init(manager).await?;
    }

    Ok(())
}

/// Main function for the ActionController component
///
/// Sets up and runs the ActionController service which:
/// 1. Receives events from FilterGateway and StateManager
/// 2. Manages workloads via Bluechi Controller API or NodeAgent API
/// 3. Orchestrates node operations based on scenario requirements
///
/// # Errors
///
/// Returns an error if the service fails to start or encounters a
/// critical error during operation.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = logger::init_async_logger("actioncontroller").await;
    println!("initiailize action controller");

    // Initialize the controller
    initialize(false).await?;

    // TODO: Set up gRPC server

    // Keep the application running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down ActionController...");

    Ok(())
}

//UNIT TEST
#[cfg(test)]
mod tests {
    use super::*;

    // Positive test: initialize should succeed when skip_grpc is true
    #[tokio::test]
    async fn test_initialize_success() {
        let result = initialize(true).await;
        assert!(
            result.is_ok(),
            "Expected initialize() to return Ok(), got Err: {:?}",
            result.err()
        );
    }

    // Negative test (edge case): double initialization (should not panic or fail)
    #[tokio::test]
    async fn test_double_initialize() {
        let first = initialize(true).await;
        let second = initialize(true).await;

        assert!(first.is_ok(), "First initialize() should succeed");
        assert!(second.is_ok(), "Second initialize() should succeed");
    }
}
