/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Performs tasks required for Bluechi integration

mod filemaker;
mod parser;

use common::spec::{
    artifact::{Model, Package},
    k8s::Pod,
};

/// Parsing model artifacts and make files about bluechi
///
/// ### Parametets
/// * `package_str` - whole yaml string of package artifact
/// ### Description
/// Get base `Model` information from package spec  
/// Combine `Network`, `Volume`, parsed `Model` information  
/// Convert `Model` to `Pod`  
/// Make `.kube`, `.yaml` files for bluechi  
/// Copy files to the guest node running Bluechi
pub async fn parse(yaml_str: String, nodename: String) -> common::Result<()> {
    let (package_str, models_str) = parser::yaml_split(&yaml_str).await?;
    let package: Package = serde_yaml::from_str(&package_str)?;

    let models: Vec<Model> =
        parser::get_complete_model(package, nodename.clone(), models_str).await?;
    let pods: Vec<Pod> = models.into_iter().map(Pod::from).collect();

    filemaker::make_files_from_pod(pods, nodename).await?;

    // filemaker::delete_symlink_and_reload(&mi.get_name(), &model_node)
    // .await
    // .map_err(|e| {
    //     format!("Failed to delete symlink for '{}': {}", mi.get_name(), e)
    // })?;

    // make_symlink_and_reload(
    // &model_node,
    // &mi.get_name(),
    // &scenario.get_targets(),
    // )
    // .await
    // .map_err(|e| {
    // format!("Failed to create symlink for '{}': {}", mi.get_name(), e)
    // })?;

    //filemaker::copy_to_remote_node(file_names)?;

    Ok(())
}
