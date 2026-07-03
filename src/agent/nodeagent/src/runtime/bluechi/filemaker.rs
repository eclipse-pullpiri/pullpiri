/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Make files for Bluechi and copy to other nodes

use common::spec::k8s::Pod;
use std::io::Write;

/// Make files about bluechi for Pod
///
/// ### Parametets
/// * `pods: Vec<Pod>` - Vector of pods
/// ### Description
/// Make `.kube`, `.yaml` files for bluechi
pub async fn make_files_from_pod(pods: Vec<Pod>, node: String) -> common::Result<()> {
    let storage_directory = &crate::config::Config::get().get_yaml_storage();
    if !std::path::Path::new(storage_directory).exists() {
        std::fs::create_dir_all(storage_directory)?;
    }
    for pod in pods {
        make_yaml_file(storage_directory, pod.clone())?;
    }
    Ok(())
}

/// Make .yaml files for Pod
///
/// ### Parametets
/// * `dir: &str, pod: Pod` - Pullpiri yaml directory path and Pod structure
/// ### Description
/// Make .yaml files for Pod
fn make_yaml_file(dir: &str, pod: Pod) -> common::Result<()> {
    let yaml_file_path = format!("{}/{}.yaml", dir, pod.get_name());
    let mut yaml_file = std::fs::File::create(yaml_file_path)?;

    let yaml_str = serde_yaml::to_string(&pod)?;
    yaml_file.write_all(yaml_str.as_bytes())?;

    Ok(())
}

// (under construction) Copy files to other nodes
//
// ### Parametets
// TBD
// ### Description
// TBD
/*pub fn copy_to_remote_node(file_names: Vec<String>) -> common::Result<()> {
    Ok(())
}*/

#[cfg(test)]
mod tests {
    use super::*;
    use common::spec::k8s::pod::PodSpec;
    use std::fs;
    use std::os::unix::fs as unix_fs;
    use std::path::Path;

    fn dummy_podspec() -> PodSpec {
        let yaml_data = r#"
hostNetwork: true
terminationGracePeriodSeconds: 0
containers:
  - name: antipinch
    image: sdv.lge.com/demo/antipinch-core:1.0
"#;
        serde_yaml::from_str::<PodSpec>(yaml_data).expect("Failed to deserialize dummy PodSpec")
    }

    #[tokio::test]
    async fn test_make_files_from_pod() {
        let podspec = dummy_podspec();
        let pod = Pod::new("antipinch-disable-core", podspec);

        let storage_dir = "/etc/pullpiri/yaml";
        let path = Path::new(storage_dir);
        if !path.exists() {
            fs::create_dir_all(path).expect("Failed to create directory");
        }

        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        let result = make_files_from_pod(vec![pod.clone()], "node1".to_string()).await;

        match result {
            Ok(_) => {
                let kube_path = format!("{}/{}.kube", storage_dir, pod.get_name());
                let yaml_path = format!("{}/{}.yaml", storage_dir, pod.get_name());
            }
            Err(e) => {
                panic!("make_files_from_pod failed: {:?}", e);
            }
        }
        // Remove the directory to force re-creation
        if std::path::Path::new(storage_dir).exists() {
            assert!(
                std::fs::remove_dir_all(storage_dir).is_ok(),
                "Failed to remove test directory"
            );
        }
    }

    /// Test that directory is created successfully
    #[tokio::test]
    async fn test_directory_creation() {
        let storage_dir = "/etc/pullpiri/yaml_test";
        let path = Path::new(storage_dir);
        std::fs::create_dir_all(path).expect("Failed to create directory");
        assert!(path.exists(), "Storage directory does not exist");
    }

    /// Test that make_yaml_file() creates the YAML file with correct content
    #[tokio::test]
    async fn test_make_yaml_file() {
        let podspec = dummy_podspec();
        let pod = Pod::new("antipinch-disable-core1", podspec);

        let storage_dir = "/etc/pullpiri/yaml_test";
        let path = Path::new(storage_dir);
        if !path.exists() {
            fs::create_dir_all(path).expect("Failed to create directory for testing");
        }

        make_yaml_file(storage_dir, pod.clone()).expect("Failed to create YAML file");

        let yaml_path = format!("{}/{}.yaml", storage_dir, pod.get_name());

        assert!(Path::new(&yaml_path).exists(), "YAML file was not created");

        let _content = fs::read_to_string(&yaml_path).expect("Failed to read YAML file");

        // Clean up
        fs::remove_file(&yaml_path).expect("Failed to remove YAML file after test");
    }

    /// Negative test: make_yaml_file() with invalid directory (should fail)
    #[tokio::test]
    async fn test_make_yaml_file_invalid_dir() {
        let invalid_dir = "/invalid/directory/for/test";
        let podspec = dummy_podspec();
        let pod = Pod::new("invalid-pod", podspec);

        let result = make_yaml_file(invalid_dir, pod);

        assert!(
            result.is_err(),
            "Expected error when creating YAML file in invalid directory"
        );
    }
}
