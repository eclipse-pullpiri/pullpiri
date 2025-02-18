// SPDX-License-Identifier: Apache-2.0

use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
pub struct ScenarioEtcd {
    pub name: String,
    pub condition: String,
    pub action: String,
    pub target: String,
}

pub fn parse_from_yaml_path(path: &str) -> common::Result<ScenarioEtcd> {
    let mut f = File::open(path)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;

    parse_from_yaml_string(&contents)
}

pub fn parse_from_yaml_string(yaml: &str) -> common::Result<ScenarioEtcd> {
    let scene: common::spec::scenario::Scenario = serde_yaml::from_str(yaml)?;
    let name = scene.get_name();
    let condition = scene.get_conditions();
    let action = scene.get_actions();
    let target = scene.get_targets();

    Ok(ScenarioEtcd {
        name,
        condition: serde_yaml::to_string(&condition)?,
        action,
        target,
    })
}
