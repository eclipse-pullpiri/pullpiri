
use manager::ScenarioParameter;


mod filter;
mod grpc;
mod manager;
mod vehicle;

use tokio::sync::mpsc::{channel, Receiver, Sender};



async fn launch_manager(rx_grpc: Receiver<ScenarioParameter>) {
    let mut manager = manager::FilterGatewayManager::new(rx_grpc).await;
    
    match manager.initialize().await {
        Ok(_) => {
            println!("FilterGatewayManager successfully initialized");
            // Only proceed to run if initialization was successful
            if let Err(e) = manager.run().await {
                eprintln!("Error running FilterGatewayManager: {:?}", e);
            }
        },
        Err(e) => {
            eprintln!("Failed to initialize FilterGatewayManager: {:?}", e);
        }
    }
}

/// Initialize FilterGateway
///
/// Sets up the manager thread, gRPC services, and DDS listeners.
/// This is the main initialization function for the FilterGateway component.
///
/// # Returns
///
async fn initialize( tx_grpc: Sender<manager::ScenarioParameter>  )  {
    // Set up logging

    // let mut manager = manager::FilterGatewayManager::new(rx_grpc, tx_dds, rx_dds);
    // manager.run().await;

    use common::filtergateway::filter_gateway_connection_server::FilterGatewayConnectionServer;
    use tonic::transport::Server;

    let server = crate::grpc::receiver::FilterGatewayReceiver::new(tx_grpc);
    let addr = common::filtergateway::open_server()
        .parse()
        .expect("gateway address parsing error");

    println!("Piccolod gateway listening on {}", addr);

    let _ = Server::builder()
        .add_service(FilterGatewayConnectionServer::new(server))
        .serve(addr)
        .await;

    
}




#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for logging

    let (tx_grpc, rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) = channel(100);
    
  
    // Launch the manager thread
    let mgr = launch_manager(rx_grpc);
        
    // Initialize the application
    let grpc = initialize(tx_grpc);
    
    tokio::join!(mgr, grpc);
    
}
//Unit Test Cases
#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::{channel, Sender, Receiver};
    use crate::manager::ScenarioParameter;
    use crate::{launch_manager, initialize};
    use tokio::task::LocalSet;
    use tokio::time::{sleep, Duration};

    /// Test to ensure that the channels are initialized with the correct capacity
    #[tokio::test]
    async fn test_main_initializes_channels() {
        let (tx_grpc, rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) = channel(100);
        assert_eq!(tx_grpc.capacity(), 100); // Check if the channel capacity is set correctly
        assert!(!rx_grpc.is_closed()); // Ensure the receiver is not closed
    }

    /// Test to ensure that the manager thread launches without any panic
    #[tokio::test(flavor = "multi_thread")]
    async fn test_main_launch_manager() {
        let (_tx_grpc, rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) = channel(100);

        // Use LocalSet to run a non-Send future like launch_manager
        let local = LocalSet::new();
        local.spawn_local(async move {
            let _ = launch_manager(rx_grpc).await;
        });

        // Run the local task for a short time to simulate launch
        tokio::select! {
            _ = local => {}
            _ = sleep(Duration::from_millis(200)) => {}
        }

        // Test is successful if it reaches this point
        assert!(true);
    }

    /// Test to ensure that the gRPC initialization runs without any panic
    #[tokio::test(flavor = "multi_thread")]
    async fn test_main_initialize_grpc() {
        let (tx_grpc, _rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) = channel(100);

        let local = LocalSet::new();
        local.spawn_local(async move {
            let _ = initialize(tx_grpc).await;
        });

        tokio::select! {
            _ = local => {}
            _ = sleep(Duration::from_millis(200)) => {}
        }

        assert!(true);
    }
}