mod model;
mod network;
mod node;
mod package;
mod scenario;
mod volume;

use super::MetaData;
use serde::{Deserialize, Serialize};

pub trait Artifact {
    fn get_name(&self) -> String;
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Scenario {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: scenario::ScenarioSpec,
    status: Option<scenario::ScenarioStatus>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Package {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: package::PackageSpec,
    status: Option<package::PackageStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Volume {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: Option<volume::VolumeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Network {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: Option<network::NetworkSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: Option<node::NodeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Model {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: model::ModelSpec,
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_scenario_artifact_trait() {
        let scenario_json = r#"
        {
            "apiVersion": "v1",
            "kind": "Scenario",
            "metadata": {
                "name": "test-scenario"
            },
            "spec": {
                "action": "test-action",
                "target": "test-target"
            }
        }"#;

        let scenario: Scenario = serde_json::from_str(scenario_json).unwrap();

        assert_eq!(scenario.get_name(), "test-scenario");
        assert_eq!(scenario.get_actions(), "test-action");
        assert_eq!(scenario.get_targets(), "test-target");
    }

    #[test]
    fn test_package_artifact_trait() {
        let package_json = r#"
        {
            "apiVersion": "v1",
            "kind": "Package",
            "metadata": {
                "name": "test-package"
            },
            "spec": {
                "pattern": [{"type": "test-pattern"}],
                "models": [
                    {
                        "name": "model1",
                        "node": "node1",
                        "resources": {
                            "volume": "vol1",
                            "network": "net1"
                        }
                    }
                ]
            }
        }"#;

        let package: Package = serde_json::from_str(package_json).unwrap();

        assert_eq!(package.get_name(), "test-package");
        let models = package.get_models();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].get_name(), "model1");
        assert_eq!(models[0].get_node(), "node1");
        assert_eq!(
            models[0].get_resources().get_volume(),
            Some("vol1".to_string())
        );
        assert_eq!(
            models[0].get_resources().get_network(),
            Some("net1".to_string())
        );
    }

    #[test]
    fn test_volume_artifact_trait() {
        let volume_json = r#"
        {
            "apiVersion": "v1",
            "kind": "Volume",
            "metadata": {
                "name": "test-volume"
            }
        }"#;

        let volume: Volume = serde_json::from_str(volume_json).unwrap();

        assert_eq!(volume.get_name(), "test-volume");
        assert!(volume.get_spec().is_none());
    }

    #[test]
    fn test_network_artifact_trait() {
        let network_json = r#"
        {
            "apiVersion": "v1",
            "kind": "Network",
            "metadata": {
                "name": "test-network"
            }
        }"#;

        let network: Network = serde_json::from_str(network_json).unwrap();

        assert_eq!(network.get_name(), "test-network");
        assert!(network.get_spec().is_none());
    }

    #[test]
    fn test_model_artifact_trait() {
        let model_json = r#"
        {
            "apiVersion": "v1",
            "kind": "Model",
            "metadata": {
                "name": "test-model"
            },
            "spec": {
                "containers": []
            }
        }"#;

        let model: Model = serde_json::from_str(model_json).unwrap();

        assert_eq!(model.get_name(), "test-model");
        let _podspec = model.get_podspec();
    }

    #[test]
    fn test_serialization_deserialization() {
        // Scenario
        let scenario_json = r#"
        {
            "apiVersion": "v1",
            "kind": "Scenario",
            "metadata": {
                "name": "test-scenario"
            },
            "spec": {
                "action": "test-action",
                "target": "test-target"
            }
        }"#;

        let scenario: Scenario = serde_json::from_str(scenario_json).unwrap();
        let serialized = serde_json::to_string(&scenario).unwrap();
        let deserialized: Scenario = serde_json::from_str(&serialized).unwrap();
        assert_eq!(scenario, deserialized);

        // Volume
        let volume_json = r#"
        {
            "apiVersion": "v1",
            "kind": "Volume",
            "metadata": {
                "name": "test-volume"
            }
        }"#;

        let volume: Volume = serde_json::from_str(volume_json).unwrap();
        let serialized = serde_json::to_string(&volume).unwrap();
        let deserialized: Volume = serde_json::from_str(&serialized).unwrap();
        assert_eq!(volume, deserialized);

        // Network
        let network_json = r#"
        {
            "apiVersion": "v1",
            "kind": "Network",
            "metadata": {
                "name": "test-network"
            }
        }"#;

        let network: Network = serde_json::from_str(network_json).unwrap();
        let serialized = serde_json::to_string(&network).unwrap();
        let deserialized: Network = serde_json::from_str(&serialized).unwrap();
        assert_eq!(network, deserialized);

        // Model
        let model_json = r#"
        {
            "apiVersion": "v1",
            "kind": "Model",
            "metadata": {
                "name": "test-model"
            },
            "spec": {
                "containers": []
            }
        }"#;

        let model: Model = serde_json::from_str(model_json).unwrap();
        let serialized = serde_json::to_string(&model).unwrap();
        let deserialized: Model = serde_json::from_str(&serialized).unwrap();
        assert_eq!(model, deserialized);
    }
}
