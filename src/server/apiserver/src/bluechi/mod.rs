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
pub async fn parse(package_str: String) -> common::Result<()> {
    let package: Package = serde_yaml::from_str(&package_str)?;

    let models: Vec<Model> = parser::get_complete_model(package).await?;
    let pods: Vec<Pod> = models.into_iter().map(Pod::from).collect();

    let file_names = filemaker::make_files_from_pod(pods).await?;
    filemaker::copy_to_remote_node(file_names)?;

    Ok(())
}
