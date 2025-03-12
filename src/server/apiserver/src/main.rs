/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

mod grpc;
mod importer;
mod route;

async fn deploy_exist_package() {
    let _ = internal_deploy_exist_package().await;
}

async fn internal_deploy_exist_package() -> common::Result<()> {
    std::thread::sleep(std::time::Duration::from_millis(3000));

    let package_path = format!("{}/packages", common::get_config().yaml_storage);
    let entries = std::fs::read_dir(package_path)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "tar" {
                    if let Some(file_name) = path.file_stem() {
                        let _name = file_name.to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    tokio::join!(route::launch_tcp_listener(), deploy_exist_package());
}

#[cfg(test)]
mod tests {
    use common::spec::package::model::Model;
    use common::spec::package::network::Network;
    use common::spec::package::volume::Volume;
    use common::spec::package::Package;
    use common::spec::scenario::Scenario;

    enum Document {
        Scenario(Scenario),
        Package(Package),
        Volume(Volume),
        Network(Network),
        Model(Model),
    }

    #[tokio::test]
    async fn load_file_text() {
        let path = std::path::Path::new("./../../../examples/resources/bms.yaml");
        let yaml_contents = std::fs::read_to_string(path).unwrap();
        let docs: Vec<&str> = yaml_contents.split("---").collect();

        let mut parsed_docs: Vec<Document> = Vec::new();

        for doc in docs {
            let value: serde_yaml::Value = serde_yaml::from_str(doc).unwrap();
            if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
                match kind {
                    "Scenario" => {
                        let scenario: Scenario = serde_yaml::from_value(value).unwrap();
                        parsed_docs.push(Document::Scenario(scenario));
                    }
                    "Package" => {
                        let package: Package = serde_yaml::from_value(value).unwrap();
                        parsed_docs.push(Document::Package(package));
                    }
                    "Volume" => {
                        let volume: Volume = serde_yaml::from_value(value).unwrap();
                        parsed_docs.push(Document::Volume(volume));
                    }
                    "Network" => {
                        let network: Network = serde_yaml::from_value(value).unwrap();
                        parsed_docs.push(Document::Network(network));
                    }
                    "Pod" => {
                        let model: Model = serde_yaml::from_value(value).unwrap();
                        parsed_docs.push(Document::Model(model));
                    }
                    _ => {
                        println!("unknown");
                    }
                }
            }
        }

        for parsed_doc in parsed_docs {
            match parsed_doc {
                Document::Scenario(scenario) => {
                    println!("{:?}", scenario.get_name())
                }
                Document::Package(package) => {
                    println!("{:?}", package.get_name())
                }
                Document::Volume(volume) => {
                    println!("{:?}", volume.get_spec())
                }
                Document::Network(network) => {
                    println!("{:?}", network.get_spec())
                }
                Document::Model(model) => {
                    println!("{:?}", model.get_name())
                }
            }
        }
    }
}
