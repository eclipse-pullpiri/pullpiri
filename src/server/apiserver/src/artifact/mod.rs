/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Convert string-type artifacts to struct and access etcd

pub mod data;

use common::spec::artifact::Artifact;
use common::spec::artifact::Model;
use common::spec::artifact::Network;
use common::spec::artifact::Package;
use common::spec::artifact::Scenario;
use common::spec::artifact::Volume;

/// Apply downloaded artifact to etcd
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Returns
/// * `Result(String, String)` - scenario and package yaml in downloaded artifact
/// ### Description
/// Write artifact in etcd
pub async fn apply(body: &str) -> common::Result<(String, String)> {
    let docs: Vec<&str> = body.split("---").collect();
    let mut scenario_str = String::new();
    let mut package_str = String::new();

    for doc in docs {
        let value: serde_yaml::Value = serde_yaml::from_str(doc)?;
        let artifact_str = serde_yaml::to_string(&value)?;

        if let Some(kind) = value.clone().get("kind").and_then(|k| k.as_str()) {
            let name: String = match kind {
                "Scenario" => serde_yaml::from_value::<Scenario>(value)?.get_name(),
                "Package" => serde_yaml::from_value::<Package>(value)?.get_name(),
                "Volume" => serde_yaml::from_value::<Volume>(value)?.get_name(),
                "Network" => serde_yaml::from_value::<Network>(value)?.get_name(),
                "Model" => serde_yaml::from_value::<Model>(value)?.get_name(),
                _ => {
                    println!("unknown artifact");
                    continue;
                }
            };
            let key = format!("{}/{}", kind, name);
            data::write_to_etcd(&key, &artifact_str).await?;

            match kind {
                "Scenario" => scenario_str = artifact_str,
                "Package" => package_str = artifact_str,
                _ => continue,
            };
        }
    }

    if scenario_str.is_empty() {
        Err("There is not any scenario in yaml string".into())
    } else {
        Ok((scenario_str, package_str))
    }
}

/// Delete downloaded artifact to etcd
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Returns
/// * `Result(String)` - scenario yaml in downloaded artifact
/// ### Description
/// Delete scenario yaml only, because other scenario can use a package with same name
pub async fn withdraw(body: &str) -> common::Result<String> {
    let docs: Vec<&str> = body.split("---").collect();
    for doc in docs {
        let value: serde_yaml::Value = serde_yaml::from_str(doc)?;
        let artifact_str = serde_yaml::to_string(&value)?;

        if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
            match kind {
                "Scenario" => {
                    let name = serde_yaml::from_value::<Scenario>(value)?.get_name();
                    let key = format!("Scenario/{}", name);
                    data::delete_at_etcd(&key).await?;
                    return Ok(artifact_str);
                }
                _ => {
                    println!("unused artifact");
                }
            }
        }
    }

    Err("There is not any scenario in yaml string".into())
}
