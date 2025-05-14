use super::Artifact;
use super::Volume;

impl Artifact for Volume {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Volume {
    pub fn get_spec(&self) -> &Option<VolumeSpec> {
        &self.spec
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VolumeSpec {
    volumes: Option<Vec<crate::spec::k8s::pod::Volume>>,
}

impl VolumeSpec {
    pub fn get_volume(&self) -> &Option<Vec<crate::spec::k8s::pod::Volume>> {
        &self.volumes
    }
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::MetaData;
    // Test case to verify the `get_name` function returns the correct name.
    #[tokio::test]
    async fn test_get_name_valid() {
        let volume = Volume {
            apiVersion: String::from("v1"), // Required field for Volume struct
            kind: String::from("Volume"),  // Required field for Volume struct
            metadata: MetaData {
                name: String::from("test-volume"), // Valid name
                annotations: Some(std::collections::HashMap::new()), // Empty annotations
                labels: Some(std::collections::HashMap::new()),      // Empty labels
            },
            spec: None, // No spec provided
        };
        assert_eq!(volume.get_name(), "test-volume");
    }

    // Test case to verify the `get_name` function handles an empty name correctly.
    #[tokio::test]
    async fn test_get_name_empty() {
        let volume = Volume {
            apiVersion: String::from("v1"), // Required field for Volume struct
            kind: String::from("Volume"),  // Required field for Volume struct
            metadata: MetaData {
                name: String::from(""), // Empty name
                annotations: Some(std::collections::HashMap::new()), // Empty annotations
                labels: Some(std::collections::HashMap::new()),      // Empty labels
            },
            spec: None, // No spec provided
        };
        assert_eq!(volume.get_name(), "");
    }

    // Test case to verify the `get_spec` function returns `None` when no spec is provided.
    #[tokio::test]
    async fn test_get_spec_none() {
        let volume = Volume {
            apiVersion: String::from("v1"), // Required field for Volume struct
            kind: String::from("Volume"),  // Required field for Volume struct
            metadata: MetaData {
                name: String::from("test-volume"), // Valid name
                annotations: Some(std::collections::HashMap::new()), // Empty annotations
                labels: Some(std::collections::HashMap::new()),      // Empty labels
            },
            spec: None, // No spec provided
        };
        assert_eq!(volume.get_spec(), &None);
    }

    // Test case to verify the `get_spec` function returns the correct spec when provided.
    #[tokio::test]
    async fn test_get_spec_some() {
        let volume_spec = VolumeSpec {
            volumes: Some(vec![]), // Empty volumes list
        };
        let volume = Volume {
            apiVersion: String::from("v1"), // Required field for Volume struct
            kind: String::from("Volume"),  // Required field for Volume struct
            metadata: MetaData {
                name: String::from("test-volume"), // Valid name
                annotations: Some(std::collections::HashMap::new()), // Empty annotations
                labels: Some(std::collections::HashMap::new()),      // Empty labels
            },
            spec: Some(volume_spec.clone()), // Spec provided
        };
        assert_eq!(volume.get_spec(), &Some(volume_spec));
    }

    // Test case to verify the `get_volume` function returns `None` when no volumes are provided.
    #[tokio::test]
    async fn test_get_volume_none() {
        let volume_spec = VolumeSpec { volumes: None }; // No volumes provided
        assert_eq!(volume_spec.get_volume(), &None);
    }

    // Test case to verify the `get_volume` function returns the correct volumes when provided.
    #[tokio::test]
    async fn test_get_volume_some() {
        let volumes = vec![]; // Empty list of volumes
        let volume_spec = VolumeSpec {
            volumes: Some(volumes.clone()), // Volumes provided
        };
        assert_eq!(volume_spec.get_volume(), &Some(volumes));
    }

    // Negative test case to verify the `get_volume` function does not return incorrect values.
    #[tokio::test]
    async fn test_get_volume_invalid() {
        let volume_spec = VolumeSpec { volumes: None }; // No volumes provided
        assert_ne!(volume_spec.get_volume(), &Some(vec![])); // Should not match an empty list
    }

    // Negative test case to verify the `get_spec` function does not return incorrect values.
    #[tokio::test]
    async fn test_get_spec_invalid() {
        let volume_spec = VolumeSpec {
            volumes: Some(vec![]), // Empty volumes list
        };
        let volume = Volume {
            apiVersion: String::from("v1"), // Required field for Volume struct
            kind: String::from("Volume"),  // Required field for Volume struct
            metadata: MetaData {
                name: String::from("test-volume"), // Valid name
                annotations: Some(std::collections::HashMap::new()), // Empty annotations
                labels: Some(std::collections::HashMap::new()),      // Empty labels
            },
            spec: None, // No spec provided
        };
        assert_ne!(volume.get_spec(), &Some(volume_spec)); // Should not match the provided spec
    }
}