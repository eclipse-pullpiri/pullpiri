/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Create Model artifact from given Package information

use common::spec::artifact::{Model, Network, Package, Volume};

/// Get combined `Network`, `Volume`, parsed `Model` information
///
/// ### Parametets
/// * `p: Package` - Package artifact
/// ### Description
/// Get base `Model` information from package spec  
/// Combine `Network`, `Volume`, parsed `Model` information
pub async fn get_complete_model(p: Package) -> common::Result<Vec<Model>> {
    let mut models: Vec<Model> = Vec::new();

    for mi in p.get_models() {
        let mut key = format!("Model/{}", mi.get_name());
        let base_model_str = common::etcd::get(&key).await?;
        let model: Model = serde_yaml::from_str(&base_model_str)?;

        if let Some(volume_name) = mi.get_resources().get_volume() {
            key = format!("Volume/{}", volume_name);
            let volume_str = common::etcd::get(&key).await?;
            let volume: Volume = serde_yaml::from_str(&volume_str)?;

            if let Some(volume_spec) = volume.get_spec() {
                model
                    .get_podspec()
                    .volumes
                    .clone_from(volume_spec.get_volume());
            }
        }

        if let Some(network_name) = mi.get_resources().get_network() {
            key = format!("Network/{}", network_name);
            let network_str = common::etcd::get(&key).await?;
            let network: Network = serde_yaml::from_str(&network_str)?;

            if let Some(network_spec) = network.get_spec() {
                // TODO
            }
        }

        models.push(model);
    }

    Ok(models)
}
