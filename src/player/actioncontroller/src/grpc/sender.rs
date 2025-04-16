use tonic::{Request, transport::Channel};
use common::Result;

// Import the generated protobuf code for external service clients
use common::policymanager::{
    policy_manager_connection_client::PolicyManagerConnectionClient,
    CheckPolicyRequest, CheckPolicyResponse
};

use common::nodeagent::{
    node_agent_connection_client::NodeAgentConnectionClient,
    HandleWorkloadRequest, HandleWorkloadResponse
};

/// Sender for making outgoing gRPC requests from ActionController
///
/// Responsible for communication with:
/// - PolicyManager: For policy checks before taking actions
/// - NodeAgent: For handling workloads on nodes that don't use Bluechi
pub struct ActionControllerSender {
    /// Client for communicating with PolicyManager
    policy_client: Option<PolicyManagerConnectionClient<Channel>>,
    /// Client for communicating with NodeAgent
    nodeagent_client: Option<NodeAgentConnectionClient<Channel>>,
}

impl ActionControllerSender {
    /// Create a new ActionControllerSender instance
    ///
    /// Initializes a sender with no active client connections.
    /// Connections must be established using the `init` method.
    ///
    /// # Returns
    ///
    /// A new ActionControllerSender instance
    pub fn new() -> Self {
        Self {
            policy_client: None,
            nodeagent_client: None,
        }
    }

    /// Initialize gRPC client connections
    ///
    /// Establishes connections to:
    /// - PolicyManager service
    /// - NodeAgent service
    ///
    /// # Returns
    ///
    /// * `Ok(())` if initialization was successful
    /// * `Err(...)` if connection establishment failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Connection to PolicyManager fails
    /// - Connection to NodeAgent fails
    pub async fn init(&mut self) -> Result<()> {
        // Initialize the policy client
        let policy_addr = common::policymanager::connect_server();
        self.policy_client = Some(PolicyManagerConnectionClient::connect(policy_addr).await?);
        
        // Initialize the nodeagent client
        let nodeagent_addr = common::nodeagent::connect_server();
        self.nodeagent_client = Some(NodeAgentConnectionClient::connect(nodeagent_addr).await?);
        
        Ok(())
    }

    /// Check if a scenario is allowed by policy
    ///
    /// Makes a gRPC request to PolicyManager to check if the scenario
    /// meets the current policy requirements.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - The name of the scenario to check
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the policy check passes
    /// * `Err(...)` if the policy check fails or the request fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The connection to PolicyManager is not established
    /// - The gRPC request fails
    /// - The policy check fails
    pub async fn check_policy(&self, scenario_name: String) -> Result<()> {
        // TODO: Implementation
        if let Some(client) = &self.policy_client {
            let request = Request::new(CheckPolicyRequest {
                scenario_name,
            });
            
            // Make the gRPC call and handle the response
        }
        
        Ok(())
    }

    /// Send a workload handling request to NodeAgent
    ///
    /// Makes a gRPC request to NodeAgent to perform an action on a workload
    /// (create, delete, start, stop, etc.)
    ///
    /// # Arguments
    ///
    /// * `workload_name` - The name of the workload to handle
    /// * `action` - The action to perform (numeric code)
    /// * `description` - Additional information about the action
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the request was successful
    /// * `Err(...)` if the request failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The connection to NodeAgent is not established
    /// - The gRPC request fails
    /// - The workload handling operation fails
    pub async fn handle_workload(&self, workload_name: String, action: i32, description: String) -> Result<()> {
        // TODO: Implementation
        if let Some(client) = &self.nodeagent_client {
            let request = Request::new(HandleWorkloadRequest {
                workload_name,
                action,
                description,
            });
            
            // Make the gRPC call and handle the response
        }
        
        Ok(())
    }
}