use super::Artifact;
use super::Package;

impl Artifact for Package {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Package {
    pub fn get_models(&self) -> &Vec<ModelInfo> {
        &self.spec.models
    }
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct PackageSpec {
    pattern: Vec<Pattern>,
    models: Vec<ModelInfo>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Pattern {
    r#type: String,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct ModelInfo {
    name: String,
    node: String,
    resources: Resource,
}

impl ModelInfo {
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_node(&self) -> String {
        self.node.clone()
    }

    pub fn get_resources(&self) -> Resource {
        self.resources.clone()
    }
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct Resource {
    volume: Option<String>,
    network: Option<String>,
    realtime: Option<bool>,
}

impl Resource {
    pub fn get_volume(&self) -> Option<String> {
        self.volume.clone()
    }
    pub fn get_network(&self) -> Option<String> {
        self.network.clone()
    }
    pub fn get_realtime(&self) -> Option<bool> {
        self.realtime
    }
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct PackageStatus {
    status: Vec<ModelStatus>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct ModelStatus {
    name: String,
    state: ModelStatusState,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
enum ModelStatusState {
    None,
    Running,
    Error,
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::MetaData;

    fn create_test_package() -> Package {
        Package {
            apiVersion: "v1".to_string(),
            kind: "Package".to_string(),
            metadata: MetaData {
                name: "test-package".to_string(),
                labels: None,
                annotations: None,
            },
            spec: PackageSpec {
                pattern: vec![
                    Pattern {
                        r#type: "type1".to_string(),
                    },
                    Pattern {
                        r#type: "type2".to_string(),
                    },
                ],
                models: vec![
                    ModelInfo {
                        name: "model1".to_string(),
                        node: "node1".to_string(),
                        resources: Resource {
                            volume: Some("vol1".to_string()),
                            network: Some("net1".to_string()),
                            realtime: None,
                        },
                    },
                    ModelInfo {
                        name: "model2".to_string(),
                        node: "node2".to_string(),
                        resources: Resource {
                            volume: Some("vol2".to_string()),
                            network: None,
                            realtime: None,
                        },
                    },
                ],
            },
            status: Some(PackageStatus {
                status: vec![
                    ModelStatus {
                        name: "model1".to_string(),
                        state: ModelStatusState::Running,
                    },
                    ModelStatus {
                        name: "model2".to_string(),
                        state: ModelStatusState::None,
                    },
                ],
            }),
        }
    }

    #[test]
    fn test_artifact_trait_implementation() {
        let package = create_test_package();
        assert_eq!(package.get_name(), "test-package");
    }

    #[test]
    fn test_get_models() {
        let package = create_test_package();
        let models = package.get_models();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "model1");
        assert_eq!(models[1].name, "model2");
    }

    #[test]
    fn test_model_info_methods() {
        let model = ModelInfo {
            name: "test-model".to_string(),
            node: "test-node".to_string(),
            resources: Resource {
                volume: Some("test-vol".to_string()),
                network: Some("test-net".to_string()),
                realtime: None,
            },
        };

        assert_eq!(model.get_name(), "test-model");
        assert_eq!(model.get_node(), "test-node");

        let resources = model.get_resources();
        assert_eq!(resources.get_volume(), Some("test-vol".to_string()));
        assert_eq!(resources.get_network(), Some("test-net".to_string()));
    }

    #[test]
    fn test_resource_methods() {
        let resource_with_both = Resource {
            volume: Some("vol1".to_string()),
            network: Some("net1".to_string()),
            realtime: None,
        };

        let resource_with_volume_only = Resource {
            volume: Some("vol2".to_string()),
            network: None,
            realtime: None,
        };

        let resource_with_nothing = Resource {
            volume: None,
            network: None,
            realtime: None,
        };

        assert_eq!(resource_with_both.get_volume(), Some("vol1".to_string()));
        assert_eq!(resource_with_both.get_network(), Some("net1".to_string()));
        assert_eq!(resource_with_both.get_realtime(), None);

        assert_eq!(
            resource_with_volume_only.get_volume(),
            Some("vol2".to_string())
        );
        assert_eq!(resource_with_volume_only.get_network(), None);

        assert_eq!(resource_with_nothing.get_volume(), None);
        assert_eq!(resource_with_nothing.get_network(), None);
    }

    #[test]
    fn test_package_without_status() {
        let package = Package {
            apiVersion: "v1".to_string(),
            kind: "Package".to_string(),
            metadata: MetaData {
                name: "no-status-package".to_string(),
                labels: None,
                annotations: None,
            },
            spec: PackageSpec {
                pattern: vec![],
                models: vec![],
            },
            status: None,
        };

        assert_eq!(package.get_name(), "no-status-package");
        assert!(package.get_models().is_empty());
    }

    #[test]
    fn test_empty_package() {
        let package = Package {
            apiVersion: "v1".to_string(),
            kind: "Package".to_string(),
            metadata: MetaData {
                name: "empty-package".to_string(),
                labels: None,
                annotations: None,
            },
            spec: PackageSpec {
                pattern: vec![],
                models: vec![],
            },
            status: None,
        };

        assert_eq!(package.get_name(), "empty-package");
        assert_eq!(package.get_models().len(), 0);
    }

    #[test]
    fn test_model_status_state_equality() {
        let running = ModelStatusState::Running;
        let none = ModelStatusState::None;
        let error = ModelStatusState::Error;

        // Test equality
        assert_eq!(running, ModelStatusState::Running);
        assert_eq!(none, ModelStatusState::None);
        assert_eq!(error, ModelStatusState::Error);
    }
}
