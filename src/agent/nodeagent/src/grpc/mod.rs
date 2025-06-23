pub mod receiver;
pub mod sender;

use std::sync::Arc;

use tonic::transport::Server;

pub async fn init(manager: crate::manager::NodeAgentManager) -> common::Result<()> {
    let arc_manager = Arc::new(manager);
    let grpc_server = receiver::NodeAgentReceiver::new(arc_manager.clone());

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
