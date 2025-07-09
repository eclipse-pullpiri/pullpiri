/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Make files for Bluechi and copy to other nodes

use common::spec::k8s::Pod;
use std::io::Write;
const SYSTEMD_PATH: &str = "/etc/containers/systemd/";

/// Make files about bluechi for Pod
///
/// ### Parametets
/// * `pods: Vec<Pod>` - Vector of pods
/// ### Description
/// Make `.kube`, `.yaml` files for bluechi
pub async fn make_files_from_pod(pods: Vec<Pod>, node: String) -> common::Result<()> {
    let storage_directory = &common::setting::get_config().yaml_storage;
    if !std::path::Path::new(storage_directory).exists() {
        std::fs::create_dir_all(storage_directory)?;
    }
    for pod in pods {
        make_kube_file(storage_directory, &pod.get_name())?;
        make_yaml_file(storage_directory, pod.clone())?;
        delete_symlink(&pod.get_name())
            .await
            .map_err(|e| format!("Failed to delete symlink for '{}': {}", pod.get_name(), e))?;
        make_symlink(&node, &pod.get_name())
            .await
            .map_err(|e| format!("Failed to create symlink for '{}': {}", pod.get_name(), e))?;
    }
    Ok(())
}

pub async fn make_symlink(node_name: &str, model_name: &str) -> common::Result<()> {
    println!(
        "make_symlink_and_reload'{:?}' on host node '{:?}'",
        model_name, node_name
    );
    let original: String = format!(
        "{0}/{1}.kube",
        common::setting::get_config().yaml_storage,
        model_name
    );
    let link = format!("{}{}.kube", SYSTEMD_PATH, model_name);

    let _ = std::os::unix::fs::symlink(original, link)?;

    Ok(())
}

pub async fn delete_symlink(model_name: &str) -> common::Result<()> {
    // host node
    let kube_symlink_path = format!("{}{}.kube", SYSTEMD_PATH, model_name);
    let _ = std::fs::remove_file(&kube_symlink_path);

    Ok(())
}

/// Make .kube files for Pod
///
/// ### Parametets
/// * `dir: &str, pod_name: &str` - Piccolo yaml directory path and pod name
/// ### Description
/// Make .kube files for Pod
fn make_kube_file(dir: &str, pod_name: &str) -> common::Result<()> {
    let kube_file_path = format!("{}/{}.kube", dir, pod_name);
    let yaml_file_path = format!("{}/{}.yaml", dir, pod_name);
    let mut kube_file = std::fs::File::create(kube_file_path)?;

    let kube_contents = format!(
        r#"[Unit]
Description=A kubernetes yaml based {} service
After=network.target

[Install]
# Start by default on boot
WantedBy=multi-user.target default.target

[Kube]
Yaml={}

[Service]
Restart=no
"#,
        pod_name, yaml_file_path
    );
    kube_file.write_all(kube_contents.as_bytes())?;

    Ok(())
}

/// Make .yaml files for Pod
///
/// ### Parametets
/// * `dir: &str, pod: Pod` - Piccolo yaml directory path and Pod structure
/// ### Description
/// Make .yaml files for Pod
fn make_yaml_file(dir: &str, pod: Pod) -> common::Result<()> {
    let yaml_file_path = format!("{}/{}.yaml", dir, pod.get_name());
    let mut yaml_file = std::fs::File::create(yaml_file_path)?;

    let yaml_str = serde_yaml::to_string(&pod)?;
    yaml_file.write_all(yaml_str.as_bytes())?;

    Ok(())
}

/// (under construction) Copy Bluechi files to other nodes
///
/// ### Parametets
/// TBD
/// ### Description
/// TBD
pub fn copy_to_remote_node(file_names: Vec<String>) -> common::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::spec::k8s::pod::PodSpec;
    use serde_yaml;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Returns a dummy PodSpec for testing
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

    /// Test that make_files_from_pod() creates expected files
    #[tokio::test]
    async fn test_make_files_from_pod() {
        let podspec = dummy_podspec();
        let pod = Pod::new("antipinch-disable-core", podspec);

        let storage_dir = "/etc/piccolo/yaml";
        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        let result = make_files_from_pod(vec![pod.clone()], "HPC".to_string()).await;

        match result {
            Ok(created_files) => {
                //assert_eq!(created_files, vec![pod.get_name()]);

                let kube_path = format!("{}/{}.kube", storage_dir, pod.get_name());
                let yaml_path = format!("{}/{}.yaml", storage_dir, pod.get_name());

                assert!(Path::new(&kube_path).exists(), "Kube file was not created");
                assert!(Path::new(&yaml_path).exists(), "YAML file was not created");

                // Clean up
                fs::remove_file(kube_path).expect("Failed to remove kube file");
                fs::remove_file(yaml_path).expect("Failed to remove yaml file");
            }
            Err(e) => {
                panic!("make_files_from_pod failed: {:?}", e);
            }
        }
        // Remove the directory to force re-creation
        if std::path::Path::new(storage_dir).exists() {
            std::fs::remove_dir_all(storage_dir).unwrap();
        }
    }

    /// Test that directory is created successfully
    #[tokio::test]
    async fn test_directory_creation() {
        let storage_dir = "/etc/piccolo/yaml_test";
        let path = Path::new(storage_dir);
        std::fs::create_dir_all(path).expect("Failed to create directory");
        assert!(path.exists(), "Storage directory does not exist");
    }

    /// Test that make_kube_file() creates the .kube file with correct content
    #[tokio::test]
    async fn test_make_kube_file() {
        let storage_dir = "/etc/piccolo/yaml_test";
        let pod_name = "antipinch-disable-core";

        let path = Path::new(storage_dir);
        if !path.exists() {
            fs::create_dir_all(path).expect("Failed to create directory");
        }

        make_kube_file(storage_dir, pod_name).expect("Failed to create kube file");

        let kube_file_path = format!("{}/{}.kube", storage_dir, pod_name);

        assert!(
            Path::new(&kube_file_path).exists(),
            "Kube file was not created"
        );

        let content = fs::read_to_string(&kube_file_path).expect("Failed to read kube file");

        assert!(content.contains("[Unit]"), "Kube file missing [Unit]");
        assert!(
            content.contains("Description=A kubernetes yaml based"),
            "Kube file missing description"
        );
        assert!(
            content.contains(&format!("Yaml={}/{}.yaml", storage_dir, pod_name)),
            "Kube file missing Yaml reference"
        );

        // Clean up
        fs::remove_file(kube_file_path).expect("Failed to remove kube file");
    }

    /// Test that make_yaml_file() creates the YAML file with correct content
    #[tokio::test]
    async fn test_make_yaml_file() {
        let podspec = dummy_podspec();
        let pod = Pod::new("antipinch-disable-core1", podspec);

        let storage_dir = "/etc/piccolo/yaml_test";
        let path = Path::new(storage_dir);
        if !path.exists() {
            fs::create_dir_all(path).expect("Failed to create directory for testing");
        }

        make_yaml_file(storage_dir, pod.clone()).expect("Failed to create YAML file");

        let yaml_path = format!("{}/{}.yaml", storage_dir, pod.get_name());

        assert!(Path::new(&yaml_path).exists(), "YAML file was not created");

        let content = fs::read_to_string(&yaml_path).expect("Failed to read YAML file");

        // Clean up
        fs::remove_file(&yaml_path).expect("Failed to remove YAML file after test");
    }

    /// Negative test: make_kube_file() with invalid directory (should fail)
    #[tokio::test]
    async fn test_make_kube_file_invalid_dir() {
        let invalid_dir = "/invalid/directory/for/test";
        let pod_name = "invalid-pod";

        let result = make_kube_file(invalid_dir, pod_name);

        assert!(
            result.is_err(),
            "Expected error when creating kube file in invalid directory"
        );
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
