/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
mod file_handler;
pub mod parser;

use parser::{package, scenario};
use std::path::Path;

/*
 * Parsing scenario & package
 * - Check URL and local file path
 * - Download if file does not exist in local path
 * - (Package only) Extract TAR archive
 * - Parsing
 * - return to apiserver
 */
pub async fn parse_package(package_name: &str) -> common::Result<package::PackageEtcd> {
    //url path
    let full_url: String = format!(
        "{}/packages/{}.tar",
        common::get_config().doc_registry,
        package_name
    );
    println!("full url : {}", full_url);

    //save path
    let full_save_path = format!(
        "{}/packages/{}.tar",
        common::get_config().yaml_storage,
        package_name
    );
    println!("full save path : {}", full_save_path);

    //download, decompress
    if !Path::new(&full_save_path).exists() {
        file_handler::download(&full_url, &full_save_path).await?;
    }
    file_handler::extract(&full_save_path)?;

    //parsing
    let parsing_path = format!(
        "{}/packages/{}",
        common::get_config().yaml_storage,
        package_name
    );
    package::parse(&parsing_path)
}

pub async fn get_scenario_from_file(scenario_path: &str) -> common::Result<scenario::ScenarioEtcd> {
    let full_url = format!(
        "{}/scenarios/{}.yaml",
        common::get_config().doc_registry,
        scenario_path
    );
    let full_save_path = format!(
        "{}/scenarios/{}.yaml",
        common::get_config().yaml_storage,
        scenario_path
    );

    if !Path::new(&full_save_path).exists() {
        file_handler::download(&full_url, &full_save_path).await?;
    }
    scenario::parse_from_yaml_path(&full_save_path)
}

pub async fn get_scenario_from_yaml(yaml_str: &str) -> common::Result<scenario::ScenarioEtcd> {
    scenario::parse_from_yaml_string(yaml_str)
}

#[cfg(test)]
mod tests {
    fn recursive_call(path: &str) -> std::io::Result<()> {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            let file_name = entry.file_name();
            let remote_file_path = format!("{}/{}", path, file_name.to_string_lossy());
            println!("{:?}\n{:?}\n", entry_path, remote_file_path);
            if entry_path.is_dir() {
                recursive_call(&remote_file_path)?;
            } else {
                let local_file = std::fs::File::open(&entry_path)?;
                println!("[F] {:?}", local_file);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn read_dir_recursively() {
        let path = "/usr/local";
        let result = recursive_call(path);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn downloading() {
        use crate::file_handler;
        let url = "http://localhost:41234/packages/bms-eco-mode.tar";
        let path = "/root/bms-eco-mode.tar";
        let result = file_handler::download(url, path).await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}
