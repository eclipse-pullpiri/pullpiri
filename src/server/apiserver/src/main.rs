/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
mod artifact;
mod grpc;
//mod importer;
mod route;

async fn initialize() {
    tokio::join!(route::launch_tcp_listener(), artifact::data::reload());
}

#[tokio::main]
async fn main() {
    initialize().await
}

#[cfg(test)]
mod tests {
    use common::spec::artifact::Artifact;
    use common::spec::artifact::Model;
    use common::spec::artifact::Network;
    use common::spec::artifact::Volume;
    use common::spec::artifact::Package;
    use common::spec::artifact::Scenario;

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
