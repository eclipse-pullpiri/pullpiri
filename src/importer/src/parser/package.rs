// SPDX-License-Identifier: Apache-2.0

use crate::file_handler;
use common::spec::package;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
pub struct PackageEtcd {
    pub name: String,
    pub model_names: Vec<String>,
    pub nodes: Vec<String>,
}

pub fn parse(path: &str) -> common::Result<PackageEtcd> {
    file_handler::create_exist_folder(path)?;

    let package_yaml = format!("{}/package.yaml", path);
    let mut f = File::open(package_yaml)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;

    let package: package::Package = serde_yaml::from_str(&contents)?;
    let name: String = package.get_name();

    let mut model_names: Vec<String> = Vec::new();
    let mut models: Vec<String> = Vec::new();
    let mut nodes: Vec<String> = Vec::new();
    let mut networks: Vec<String> = Vec::new();
    let mut volumes: Vec<String> = Vec::new();

    for m in package.get_models() {
        let model: package::model::Model = model_parse(path, &m.get_name()).unwrap();
        model_names.push(m.get_name());

        nodes.push(m.get_node());

        let network_yaml = m.get_resources().get_network();
        networks.push(serde_yaml::to_string(&network_parse(path, &network_yaml))?);

        let volume_yaml = m.get_resources().get_volume();
        volumes.push(serde_yaml::to_string(&volume_parse(path, &volume_yaml))?);

        if let Some(volume_spec) = &volume_parse(path, &volume_yaml) {
            model
                .get_podspec()
                .volumes
                .clone_from(volume_spec.get_volume());
        }

        models.push(serde_yaml::to_string(&model)?);

        //make kube,yaml file
        file_handler::perform(&m.get_name(), &serde_yaml::to_string(&model)?, &name)?;
    }

    if let Err(e) = file_handler::copy_to_remote_node(path) {
        println!("[E] cannot copy package files to remote node: {:?}", e);
    }

    Ok(PackageEtcd {
        name,
        model_names,
        nodes,
    })
}

fn parse_yaml<T>(path: &str, name: &str, subdir: &str) -> Result<T, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    let yaml_path = format!("{}/{}/{}.yaml", path, subdir, name);
    let mut f = File::open(yaml_path)?;
    let mut contents = String::new();
    f.read_to_string(&mut contents)?;
    let parsed: T = serde_yaml::from_str(&contents)?;
    Ok(parsed)
}

fn model_parse(path: &str, name: &str) -> Result<package::model::Model, Box<dyn Error>> {
    parse_yaml(path, name, "models")
}

fn network_parse(path: &str, name: &Option<String>) -> Option<package::network::NetworkSpec> {
    if let Some(n) = name {
        let network: package::network::Network = parse_yaml(path, n, "networks").unwrap();
        return network.get_spec().clone();
    }
    None
}

fn volume_parse(path: &str, name: &Option<String>) -> Option<package::volume::VolumeSpec> {
    if let Some(n) = name {
        if let Ok(volume) = parse_yaml::<package::volume::Volume>(path, n, "volumes") {
            return volume.get_spec().clone();
        }
    }
    None
}
