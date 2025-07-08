pub mod receiver;
pub mod sender;

use crate::manager::NodeAgentParameter;
use tokio::sync::mpsc::Sender;
use tonic::transport::Server;

/// Initializes the gRPC server for NodeAgent.
///
/// # Arguments
/// * `tx` - Channel sender for NodeAgentParameter
pub async fn init(tx: Sender<NodeAgentParameter>) -> common::Result<()> {
    let grpc_server = receiver::NodeAgentReceiver::new(tx);

    let addr = common::nodeagent::open_server().parse()?;
    println!("Starting gRPC server on {}", addr);

    tokio::spawn(async move {
        if let Err(e) = Server::builder()
            .add_service(grpc_server.into_service())
            .serve(addr)
            .await
        {
            eprintln!("gRPC server error: {}", e);
        }
    });

    println!("gRPC server started and listening");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::NodeAgentManager;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_init_success() {
        // Create a dummy channel for testing
        let (tx, _rx) = mpsc::channel(1);
        // Call the `init` function and wait for it to complete
        let result = init(tx).await;
        // Assert that the result is successful (Ok)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_init_edge_case() {
        // Create a dummy channel for testing
        let (tx, _rx) = mpsc::channel(1);
        // Call the `init` function and wait for it to complete
        let result = init(tx).await;
        // Use a match statement to handle both success and error cases
        match result {
            Ok(_) => assert!(true),
            Err(_) => assert!(false, "Expected Ok(()), but got an Err"),
        }
    }
}
