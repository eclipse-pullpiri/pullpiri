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
    dummy: Option<String>,
}

impl NodeSpec {
    pub fn get_node(&self) -> &Option<String> {
        &self.dummy
    }
}
