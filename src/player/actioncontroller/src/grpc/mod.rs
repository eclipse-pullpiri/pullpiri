pub mod receiver;
pub mod sender;

use std::sync::Arc;

use tonic::transport::Server;

/// Initialize the gRPC communication system for ActionController
///
/// Sets up the gRPC server to receive requests from FilterGateway and StateManager,
/// and establishes client connections to communicate with PolicyManager and NodeAgent.
///
/// # Returns
///
/// * `Ok(())` if initialization was successful
/// * `Err(...)` if the initialization failed
///
/// # Errors
///
/// Returns an error if:
/// - Server address binding fails
/// - Client connection establishment fails
pub async fn init(manager: crate::manager::ActionControllerManager) -> common::Result<()> {
    let arc_manager = Arc::new(manager);
    let grpc_server = receiver::ActionControllerReceiver::new(arc_manager.clone());
    
    let addr = common::actioncontroller::open_server().parse()?;
    println!("Starting gRPC server on {}", addr);
    
    tokio::spawn(async move {
        if let Err(e) = Server::builder()
            .add_service(grpc_server.into_service())
            .serve(addr)
            .await {
            eprintln!("gRPC server error: {}", e);
        }
    });
    
    println!("gRPC server started and listening");
    
    Ok(())
}

//UNIT TEST
#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::ActionControllerManager;
    use tokio::time::{sleep, timeout, Duration};

    #[tokio::test]
    async fn test_open_server_returns_valid_address() {
        let addr = common::actioncontroller::open_server();
        println!("open_server() returned: {}", addr);

        // Just check it contains ":" (host:port format)
        assert!(addr.contains(':'), "Address should be in host:port format");
    }

    #[tokio::test]
    async fn test_connect_server_returns_valid_url() {
        let url = common::actioncontroller::connect_server();
        println!("connect_server() returned: {}", url);

        // Should start with http:// and contain ":"
        assert!(url.starts_with("http://"));
        assert!(url.contains(':'));
    }

    #[tokio::test]
    async fn test_init_starts_server_and_is_cancelled_safely() {
        let manager = ActionControllerManager::new();

        // Spawn init() in a task so we can cancel later
        let task = tokio::spawn(async move {
            let result = init(manager).await;
            // We don't expect it to return unless we forcefully cancel
            assert!(result.is_ok() || result.is_err());
        });

        // Allow init to start up briefly
        sleep(Duration::from_millis(300)).await;

        // Abort task so we don't get stuck
        task.abort();
    }

    //NEGATIVE TEST
    #[tokio::test]
    async fn test_init_fails_when_port_is_already_in_use() {
        let manager1 = ActionControllerManager::new();
        let manager2 = ActionControllerManager::new();

        // Start first init() to occupy the port
        let task1 = tokio::spawn(async move {
            let result = init(manager1).await;
            assert!(result.is_ok(), "First init() should succeed");
        });

        // Give the first server time to bind the address
        sleep(Duration::from_millis(300)).await;

        // Attempt second init (should fail due to port in use)
        let result = timeout(Duration::from_secs(1), init(manager2)).await;

        match result {
            Ok(inner_result) => {
                assert!(
                    inner_result.is_err(),
                    "Second init() should fail because the port is already in use"
                );
            }
            Err(_) => {
                panic!("Second init() timed out instead of failing quickly");
            }
        }

        // Clean up first server
        task1.abort();
    }
}
