pub mod filter;
pub mod grpc;
pub mod manager;
pub mod vehicle;

// Re-export what you need in tests:
pub use common::spec::artifact::Scenario;
pub use common::Result;
pub use filter::Filter;
pub use grpc::receiver::FilterGatewayReceiver;
pub use grpc::sender::FilterGatewaySender;
pub use manager::ScenarioParameter;
use tokio::sync::mpsc::{channel, Receiver, Sender};
pub use vehicle::dds::listener;
pub use vehicle::dds::DdsData;
pub use vehicle::dds::DdsTopicListener;
pub async fn launch_manager(rx_grpc: Receiver<ScenarioParameter>) {
    let mut manager = manager::FilterGatewayManager::new(rx_grpc).await;

    match manager.initialize().await {
        Ok(_) => {
            println!("FilterGatewayManager successfully initialized");
            // Only proceed to run if initialization was successful
            if let Err(e) = manager.run().await {
                eprintln!("Error running FilterGatewayManager: {:?}", e);
            }
        }
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
pub async fn initialize(tx_grpc: Sender<manager::ScenarioParameter>) {
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
