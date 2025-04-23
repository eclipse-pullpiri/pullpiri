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
    // TODO: Implementation
    let arc_manager = Arc::new(manager);
    let grpc_server = receiver::ActionControllerReceiver::new(arc_manager.clone());

    let _ = Server::builder()
        .add_service(grpc_server.into_service())
        .serve(common::statemanager::open_server().parse()?)
        .await?;

    println!("gRPC server started on");

    Ok(())
}
