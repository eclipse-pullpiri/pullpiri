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

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::MetaData;

    // Helper function to create a test Network instance
    fn create_test_network(name: &str, dummy_value: Option<&str>) -> Network {
        Network {
            apiVersion: "v1".to_string(),
            kind: "Network".to_string(),
            metadata: MetaData {
                name: name.to_string(),
                labels: None,
                annotations: None,
            },
            spec: dummy_value.map(|v| NetworkSpec {
                dummy: Some(v.to_string()),
            }),
        }
    }

    #[test]
    fn test_artifact_trait_implementation() {
        let network = create_test_network("test-network", None);
        
        // Test Artifact trait implementation
        assert_eq!(network.get_name(), "test-network");
    }

    #[test]
    fn test_get_spec_with_spec() {
        let dummy_value = "test-dummy-value";
        let network = create_test_network("test-network", Some(dummy_value));
        
        // Test get_spec when spec exists
        let spec = network.get_spec();
        assert!(spec.is_some());
        let network_spec = spec.as_ref().unwrap();
        assert_eq!(network_spec.get_network(), &Some(dummy_value.to_string()));
    }

    #[test]
    fn test_get_spec_without_spec() {
        let network = create_test_network("test-network", None);
        
        // Test get_spec when spec is None
        let spec = network.get_spec();
        assert!(spec.is_none());
    }

    #[test]
    fn test_network_spec_get_network() {
        let dummy_value = "test-dummy-value";
        let network_spec = NetworkSpec {
            dummy: Some(dummy_value.to_string()),
        };
        
        // Test NetworkSpec's get_network method
        assert_eq!(network_spec.get_network(), &Some(dummy_value.to_string()));
    }

    #[test]
    fn test_network_spec_get_network_none() {
        let network_spec = NetworkSpec {
            dummy: None,
        };
        
        // Test NetworkSpec's get_network when dummy is None
        assert_eq!(network_spec.get_network(), &None);
    }

    #[test]
    fn test_network_serialization_deserialization() {
        let network = create_test_network("test-network", Some("dummy-value"));
        
        // Test serialization
        let serialized = serde_json::to_string(&network).unwrap();
        assert!(serialized.contains("test-network"));
        assert!(serialized.contains("dummy-value"));
        
        // Test deserialization
        let deserialized: Network = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.metadata.name, "test-network");
        assert_eq!(
            deserialized.spec.unwrap().dummy.unwrap(), 
            "dummy-value"
        );
    }

    #[test]
    fn test_partial_eq_implementation() {
        let network1 = create_test_network("network1", Some("value1"));
        let network2 = create_test_network("network1", Some("value1"));
        let network3 = create_test_network("network2", Some("value2"));
        
        // Test equality
        assert_eq!(network1, network2);
        
        // Test inequality
        assert_ne!(network1, network3);
    }

    #[test]
    fn test_debug_implementation() {
        let network = create_test_network("debug-network", Some("debug-value"));
        
        // Test Debug implementation (just verify it doesn't panic)
        let debug_output = format!("{:?}", network);
        assert!(debug_output.contains("debug-network"));
        assert!(debug_output.contains("debug-value"));
    }
}
