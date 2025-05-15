use common::Result;
use vehicle::dds::DdsData;

mod filter;
mod grpc;
mod manager;
mod vehicle;
use std::sync::Arc;
use common::spec::artifact::Scenario;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tracing;

// async fn launch_manager(rx: Receiver<Scenario>, tx_dds: Sender<DdsData>, rx_dds: Receiver<DdsData>) {
//     let mut manager = manager::FilterGatewayManager::new(rx, tx_dds, rx_dds);
//     manager.run().await;
// }
/// Initialize FilterGateway
///
/// Sets up the manager thread, gRPC services, and DDS listeners.
/// This is the main initialization function for the FilterGateway component.
///
/// # Returns
///
async fn initialize( tx_dds: Sender<DdsData>,  rx_dds: Receiver<DdsData>) -> Result<()> {
    // Set up logging
    tracing::info!("Initializing FilterGateway");
    let (tx_grpc, rx_grpc): (Sender<Scenario>, Receiver<Scenario>) = channel(100);

    let mut manager = manager::FilterGatewayManager::new(rx_grpc, tx_dds, rx_dds);
    manager.run().await;

    use common::filtergateway::filter_gateway_connection_server::FilterGatewayConnectionServer;
    use tonic::transport::Server;

    let server = crate::grpc::receiver::FilterGatewayReceiver::new(tx_grpc, manager);
    let addr = common::filtergateway::open_server()
        .parse()
        .expect("gateway address parsing error");

    println!("Piccolod gateway listening on {}", addr);

    let _ = Server::builder()
        .add_service(FilterGatewayConnectionServer::new(server))
        .serve(addr)
        .await;

    
    tracing::info!("FilterGateway initialization complete");
    Ok(())
}

/// Main function for the FilterGateway component
///
/// Starts the FilterGateway service which:
/// 1. Receives scenario information from API-Server
/// 2. Subscribes to vehicle DDS topics
/// 3. Monitors conditions and triggers actions when conditions are met
///
/// # Returns
///
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    let (tx_dds, rx_dds): (Sender<DdsData>, Receiver<DdsData>) = channel(100);

    // Initialize the application
    initialize(tx_dds.clone(), rx_dds).await?;
    
    Ok(())
}

