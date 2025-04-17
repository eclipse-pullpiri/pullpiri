use super::Artifact;
use super::Network;

impl Artifact for Network {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Network {
    pub fn get_spec(&self) -> &Option<NetworkSpec> {
        &self.spec
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NetworkSpec {
    dummy: Option<String>,
}

impl NetworkSpec {
    pub fn get_network(&self) -> &Option<String> {
        &self.dummy
    }
}
