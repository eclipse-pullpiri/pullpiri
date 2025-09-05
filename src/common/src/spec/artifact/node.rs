use super::Artifact;
use super::Node;

impl Artifact for Node {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Node {
    pub fn get_spec(&self) -> &Option<NodeSpec> {
        &self.spec
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NodeSpec {
    // Basic node information
    pub role: Option<String>,          // "master" or "sub"
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    
    // Resource information
    pub resources: Option<NodeResources>,
    
    // Clustering information
    pub cluster_id: Option<String>,
    pub status: Option<String>,
    pub last_heartbeat: Option<i64>,
    
    // Configuration
    pub config: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NodeResources {
    pub cpu_cores: Option<i32>,
    pub memory_mb: Option<i64>,
    pub disk_gb: Option<i64>,
    pub architecture: Option<String>,
    pub os_version: Option<String>,
}

impl NodeSpec {
    pub fn get_role(&self) -> &Option<String> {
        &self.role
    }

    pub fn get_ip_address(&self) -> &Option<String> {
        &self.ip_address
    }

    pub fn get_hostname(&self) -> &Option<String> {
        &self.hostname
    }

    pub fn get_resources(&self) -> &Option<NodeResources> {
        &self.resources
    }

    pub fn get_cluster_id(&self) -> &Option<String> {
        &self.cluster_id
    }

    pub fn get_status(&self) -> &Option<String> {
        &self.status
    }

    pub fn get_last_heartbeat(&self) -> &Option<i64> {
        &self.last_heartbeat
    }

    pub fn get_config(&self) -> &Option<std::collections::HashMap<String, String>> {
        &self.config
    }
}
