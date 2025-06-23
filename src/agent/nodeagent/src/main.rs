use std::error::Error;
pub mod bluechi;
pub mod grpc;
pub mod manager;

async fn initialize(skip_grpc: bool) -> Result<(), Box<dyn Error>> {
    let manager = manager::NodeAgentManager::new();
    //Production code will not effect by this change
    if !skip_grpc {
        grpc::init(manager).await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting NodeAgent...");

    // Initialize the agent
    initialize(false).await?;

    // Keep the application running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down NodeAgent...");

    Ok(())
}
