/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Create Model artifact from given Package information

use common::spec::artifact::{Artifact, Model, Network, Package, Scenario, Volume};

pub async fn yaml_split(body: &str) -> common::Result<(String, Vec<Model>)> {
    let docs: Vec<&str> = body.split("---").collect();
    let mut scenario_str = String::new();
    let mut package_str = String::new();
    let mut models: Vec<Model> = Vec::new();
    //let mut network_str = String::new();

    for doc in docs {
        let value: serde_yaml::Value = serde_yaml::from_str(doc)?;
        let artifact_str = serde_yaml::to_string(&value)?;

        if let Some(kind) = value.clone().get("kind").and_then(|k| k.as_str()) {
            let name: String = match kind {
                "Scenario" => serde_yaml::from_value::<Scenario>(value.clone())?.get_name(),
                "Package" => serde_yaml::from_value::<Package>(value.clone())?.get_name(),
                "Volume" => serde_yaml::from_value::<Volume>(value.clone())?.get_name(),
                "Network" => serde_yaml::from_value::<Network>(value.clone())?.get_name(),
                "Model" => serde_yaml::from_value::<Model>(value.clone())?.get_name(),
                _ => {
                    println!("unknown artifact");
                    continue;
                }
            };

            match kind {
                "Scenario" => scenario_str = artifact_str,
                "Package" => package_str = artifact_str,
                "Model" => {
                    let model = serde_yaml::from_value::<Model>(value)?;
                    models.push(model);
                }
                //"Network" => network_str = artifact_str,
                _ => continue,
            };
        }
    }

    if scenario_str.is_empty() {
        Err("There is not any scenario in yaml string".into())
    } else if package_str.is_empty() {
        //Missing Check is Added for Package
        Err("There is not any package in yaml string".into())
    } else {
        Ok((package_str, models)) //, network_str))
    }
}

/// Get combined `Network`, `Volume`, parsed `Model` information
///
/// ### Parametets
/// * `p: Package` - Package artifact
/// ### Description
/// Get base `Model` information from package spec  
/// Combine `Network`, `Volume`, parsed `Model` information
pub async fn get_complete_model(
    p: Package,
    node: String,
    models: Vec<Model>,
) -> common::Result<Vec<Model>> {
    let mut base_models: Vec<Model> = Vec::new();
    let mut model_name: String = String::new();
    for mi in p.get_models() {
        if mi.get_node() == node {
            model_name = mi.get_name();
            for model in models.iter() {
                if model.get_name() == model_name {
                    base_models.push(model.clone());
                } else {
                    println!("Model {} is not for this node {}", model.get_name(), node);
                    continue;
                }
            }
        } else {
            println!("Model {} is not for this node {}", mi.get_name(), node);
            continue;
        }
        //let mut key = format!("Model/{}", mi.get_name());
        //let base_model_str = common::etcd::get(&key).await?;
        //let model: Model = serde_yaml::from_str(&base_model_str)?;

        // if let Some(volume_name) = mi.get_resources().get_volume() {
        //     key = format!("Volume/{}", volume_name);
        //     let volume_str: String = common::etcd::get(&key).await?;
        //     let volume: Volume = serde_yaml::from_str(&volume_str)?;

        //     if let Some(volume_spec) = volume.get_spec() {
        //         model
        //             .get_podspec()
        //             .volumes
        //             .clone_from(volume_spec.get_volume());
        //     }
        // }

        // if let Some(network_name) = mi.get_resources().get_network() {
        //     key = format!("Network/{}", network_name);
        //     let network_str = common::etcd::get(&key).await?;
        //     let network: Network = serde_yaml::from_str(&network_str)?;

        //     if let Some(network_spec) = network.get_spec() {
        //         // TODO
        //     }
        //}
    }
    Ok(base_models)
}
