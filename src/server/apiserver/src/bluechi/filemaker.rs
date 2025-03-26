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
pub async fn make_files_from_pod(pods: Vec<Pod>) -> common::Result<Vec<String>> {
    let storage_directory = &common::setting::get_config().yaml_storage;
    if !std::path::Path::new(storage_directory).exists() {
        std::fs::create_dir_all(storage_directory)?;
    }

    let mut file_names: Vec<String> = Vec::new();

    for pod in pods {
        file_names.push(pod.get_name());
        make_kube_file(storage_directory, &pod.get_name())?;
        make_yaml_file(storage_directory, pod)?;
    }

    Ok(file_names)
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
