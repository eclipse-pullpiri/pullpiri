
use manager::ScenarioParameter;


mod filter;
mod grpc;
mod manager;
mod vehicle;

use tokio::sync::mpsc::{channel, Receiver, Sender};



async fn launch_manager(rx_grpc: Receiver<ScenarioParameter>) {
    let mut manager = manager::FilterGatewayManager::new(rx_grpc).await;
    let _= manager.run().await;
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
