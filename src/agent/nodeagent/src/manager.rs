use common::{
    spec::artifact::{Package, Scenario},
    Result,
};
pub struct NodeAgentManager {}

impl NodeAgentManager {
    pub fn new() -> Self {
        NodeAgentManager {}
    }

    pub async fn init(&self) -> common::Result<()> {
        // Initialize the gRPC server
        let manager = crate::manager::NodeAgentManager::new();
        crate::grpc::init(manager).await?;

        Ok(())
    }

    pub async fn handle_workload(&self, workload_name: &String) -> Result<()> {
        crate::bluechi::parse(workload_name.to_string()).await?;
        // Handle the workload request
        println!("Handling workload request: {:?}", workload_name);
        //ToDo : Implement the logic  1. extart etcd Network. 2. using extracted data to control the node
        Ok(())
    }

}

