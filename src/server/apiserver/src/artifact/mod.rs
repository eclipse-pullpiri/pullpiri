/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

pub mod data;

use common::spec::artifact::Artifact;
use common::spec::artifact::Model;
use common::spec::artifact::Network;
use common::spec::artifact::Package;
use common::spec::artifact::Scenario;
use common::spec::artifact::Volume;

/// Apply downloaded artifact to etcd
///
/// # parametets
/// * `body` - whole yaml string of piccolo artifact
/// # returns
/// * `Result(String)` - scenario yaml in downloaded artifact
/// # description
/// write artifact in etcd
pub async fn apply(body: &str) -> common::Result<String> {
    let docs: Vec<&str> = body.split("---").collect();
    let mut scenario_str = String::new();
    for doc in docs {
        let value: serde_yaml::Value = serde_yaml::from_str(doc)?;
        let artifact_str = serde_yaml::to_string(&value)?;
        if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
            match kind {
                "Scenario" => {
                    let name = serde_yaml::from_value::<Scenario>(value)?.get_name();
                    let key = format!("Scenario/{}", name);
                    data::write_to_etcd(&key, &artifact_str).await?;
                    scenario_str = artifact_str;
                }
                "Package" => {
                    let name = serde_yaml::from_value::<Package>(value)?.get_name();
                    let key = format!("Package/{}", name);
                    data::write_to_etcd(&key, &artifact_str).await?;
                }
                "Volume" => {
                    let name = serde_yaml::from_value::<Volume>(value)?.get_name();
                    let key = format!("Volume/{}", name);
                    data::write_to_etcd(&key, &artifact_str).await?;
                }
                "Network" => {
                    let name = serde_yaml::from_value::<Network>(value)?.get_name();
                    let key = format!("Network/{}", name);
                    data::write_to_etcd(&key, &artifact_str).await?;
                }
                "Model" => {
                    let name = serde_yaml::from_value::<Model>(value)?.get_name();
                    let key = format!("Model/{}", name);
                    data::write_to_etcd(&key, &artifact_str).await?;
                }
                _ => {
                    println!("unknown artifact");
                }
            }
        }
    }

    Ok(scenario_str)
}

/// Delete downloaded artifact to etcd
///
/// # parametets
/// * `body` - whole yaml string of piccolo artifact
/// # returns
/// * `Result(String)` - scenario yaml in downloaded artifact
/// # description
/// delete scenario yaml only, because other scenario can use a package with same name
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

#[cfg(test)]
mod tests {
    use common::spec::artifact::Artifact;
    use common::spec::artifact::Model;
    use common::spec::artifact::Network;
    use common::spec::artifact::Package;
    use common::spec::artifact::Scenario;
    use common::spec::artifact::Volume;

    enum Document {
        Scenario(Scenario),
        Package(Package),
        Volume(Volume),
        Network(Network),
        Model(Model),
    }

    #[tokio::test]
    async fn load_file_text() {
        let path = std::path::Path::new("./../../../examples/resources/bms-performance.yaml");
        let yaml_contents = std::fs::read_to_string(path).unwrap();
        let docs: Vec<&str> = yaml_contents.split("---").collect();

        let mut parsed_docs: Vec<Document> = Vec::new();

        for doc in docs {
            let value: serde_yaml::Value = serde_yaml::from_str(doc).unwrap();
            if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
                match kind {
                    "Scenario" => {
                        println!("{:#?}", value);
                        let s = serde_yaml::to_string(&value).unwrap();
                        println!("{}", s);
                        let scenario: Scenario = serde_yaml::from_value(value).unwrap();
                        println!("{:#?}", scenario);
                        let sc: Scenario = serde_yaml::from_str(&s).unwrap();
                        println!("{:#?}", sc);
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
                    "Model" => {
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
                    println!("{:?}", volume.get_name())
                }
                Document::Network(network) => {
                    println!("{:?}", network.get_name())
                }
                Document::Model(model) => {
                    println!("{:?}", model.get_name())
                }
            }
        }
    }
}
