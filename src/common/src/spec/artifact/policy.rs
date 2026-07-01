/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
use super::Artifact;
use super::Policy;

impl Artifact for Policy {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Policy {
    pub fn get_placement(&self) -> &Placement {
        &self.spec.placement
    }

    pub fn get_procedure(&self) -> &Procedure {
        &self.spec.procedure
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PolicySpec {
    pub placement: Placement,
    pub procedure: Procedure,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Placement {
    /// List of nodes where the workload can be deployed.
    /// The first node is preferred, and remaining nodes serve as fallback options.
    pub availableNodes: Vec<String>,
}

impl Placement {
    pub fn get_available_nodes(&self) -> &Vec<String> {
        &self.availableNodes
    }

    /// Get the preferred node (first in availableNodes)
    pub fn get_preferred_node(&self) -> Option<&str> {
        self.availableNodes.first().map(|s| s.as_str())
    }

    /// Get fallback nodes (all nodes except the first one)
    pub fn get_fallback_nodes(&self) -> &[String] {
        if self.availableNodes.len() > 1 {
            &self.availableNodes[1..]
        } else {
            &[]
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Procedure {
    pub r#type: String,
    pub strategy: String,
    pub trigger: Trigger,
}

impl Procedure {
    pub fn get_type(&self) -> &str {
        &self.r#type
    }

    pub fn get_strategy(&self) -> &str {
        &self.strategy
    }

    pub fn get_trigger(&self) -> &Trigger {
        &self.trigger
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Trigger {
    pub resourceThreshold: Option<ResourceThreshold>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ResourceThreshold {
    pub cpu: Option<u32>,
    pub memory: Option<u32>,
}

impl ResourceThreshold {
    pub fn get_cpu(&self) -> Option<u32> {
        self.cpu
    }

    pub fn get_memory(&self) -> Option<u32> {
        self.memory
    }
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::MetaData;

    fn create_test_policy() -> Policy {
        Policy {
            apiVersion: "v1".to_string(),
            kind: "Policy".to_string(),
            metadata: MetaData {
                name: "test-policy".to_string(),
                labels: None,
                annotations: None,
            },
            spec: PolicySpec {
                placement: Placement {
                    availableNodes: vec!["HPC".to_string(), "cloud".to_string()],
                },
                procedure: Procedure {
                    r#type: "offloading".to_string(),
                    strategy: "redeployment".to_string(),
                    trigger: Trigger {
                        resourceThreshold: Some(ResourceThreshold {
                            cpu: Some(50),
                            memory: Some(50),
                        }),
                    },
                },
            },
        }
    }

    #[test]
    fn test_artifact_trait_implementation() {
        let policy = create_test_policy();
        assert_eq!(policy.get_name(), "test-policy");
    }

    #[test]
    fn test_get_placement() {
        let policy = create_test_policy();
        let placement = policy.get_placement();

        assert_eq!(placement.get_available_nodes().len(), 2);
        assert_eq!(placement.get_preferred_node(), Some("HPC"));
        assert_eq!(placement.get_fallback_nodes(), &["cloud".to_string()]);
    }

    #[test]
    fn test_get_procedure() {
        let policy = create_test_policy();
        let procedure = policy.get_procedure();

        assert_eq!(procedure.get_type(), "offloading");
        assert_eq!(procedure.get_strategy(), "redeployment");
    }

    #[test]
    fn test_get_resource_threshold() {
        let policy = create_test_policy();
        let threshold = policy
            .get_procedure()
            .get_trigger()
            .resourceThreshold
            .as_ref()
            .unwrap();

        assert_eq!(threshold.get_cpu(), Some(50));
        assert_eq!(threshold.get_memory(), Some(50));
    }

    #[test]
    fn test_policy_serialization() {
        let policy = create_test_policy();
        let serialized = serde_json::to_string(&policy).unwrap();

        assert!(serialized.contains("\"availableNodes\""));
        assert!(serialized.contains("\"resourceThreshold\""));
    }

    #[test]
    fn test_policy_deserialization() {
        let json_data = r#"{
            "apiVersion": "v1",
            "kind": "Policy",
            "metadata": {
                "name": "policy_helloworld"
            },
            "spec": {
                "placement": {
                    "availableNodes": ["HPC", "cloud"]
                },
                "procedure": {
                    "type": "offloading",
                    "strategy": "redeployment",
                    "trigger": {
                        "resourceThreshold": {
                            "cpu": 50,
                            "memory": 50
                        }
                    }
                }
            }
        }"#;

        let policy: Policy = serde_json::from_str(json_data).unwrap();
        assert_eq!(policy.get_name(), "policy_helloworld");
        assert_eq!(policy.get_placement().get_preferred_node(), Some("HPC"));
    }
}
