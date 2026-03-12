/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::grpc::sender::csi::CsiSender;
use crate::grpc::sender::pharos::PharosSender;
use common::external::csi::{VolumeCreateRequest, VolumeDeleteRequest};
use common::external::pharos::{NetworkRemoveRequest, NetworkSetupRequest};
use common::resourcemanager::resource_manager_service_server::ResourceManagerService;
use common::resourcemanager::{Action, HandleResourceRequest, HandleResourceResponse};
use common::spec::artifact::{Network, Volume};
use tonic::Response;

// Artifact kind constants
const KIND_NETWORK: &str = "Network";
const KIND_VOLUME: &str = "Volume";

#[allow(dead_code)]
pub struct ResourceManagerGrpcServer {
    /// Pharos sender for network resource operations
    pharos_sender: PharosSender,
    /// CSI sender for volume resource operations
    csi_sender: CsiSender,
}

#[allow(dead_code)]
impl ResourceManagerGrpcServer {
    /// Creates a new ResourceManagerGrpcServer instance
    pub fn new() -> Self {
        Self {
            pharos_sender: PharosSender::new(),
            csi_sender: CsiSender::new(),
        }
    }

    /// Parse YAML and extract artifact kind
    fn parse_artifact_kind(yaml_str: &str) -> Option<String> {
        let value: serde_yaml::Value = serde_yaml::from_str(yaml_str).ok()?;
        value.get("kind")?.as_str().map(|s| s.to_string())
    }

    /// Process Network resource
    async fn process_network(&self, yaml_str: &str, action: Action) -> Result<HandleResourceResponse, String> {
        let network: Network = serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("Failed to parse Network YAML: {}", e))?;

        let spec = network.get_spec();
        let network_name = spec.get_network_name().to_string();
        let network_mode = spec.get_network_mode().as_str().to_string();

        println!("RESOURCE MANAGER: Processing Network Resource");
        println!("  Network Name: {}", &network_name);
        println!("  Network Mode: {}", &network_mode);
        println!("  Action: {:?}", action);

        match action {
            Action::Apply => {
                let pharos_req = NetworkSetupRequest {
                    network_name: network_name.clone(),
                    network_mode,
                };

                println!("Sending NetworkSetupRequest to Pharos");
                match self.pharos_sender.clone().setup_network(pharos_req).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        println!("Successfully created network resource: {}", &network_name);
                        Ok(HandleResourceResponse {
                            success: resp.success,
                            message: resp.message,
                        })
                    }
                    Err(e) => {
                        println!("Failed to create network resource: {:?}", e);
                        Ok(HandleResourceResponse {
                            success: false,
                            message: format!("Failed to create network resource: {}", e),
                        })
                    }
                }
            }
            Action::Withdraw => {
                let pharos_req = NetworkRemoveRequest {
                    network_name: network_name.clone(),
                };

                println!("Sending NetworkRemoveRequest to Pharos");
                match self.pharos_sender.clone().remove_network(pharos_req).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        println!("Successfully deleted network resource: {}", &network_name);
                        Ok(HandleResourceResponse {
                            success: resp.success,
                            message: resp.message,
                        })
                    }
                    Err(e) => {
                        println!("Failed to delete network resource: {:?}", e);
                        Ok(HandleResourceResponse {
                            success: false,
                            message: format!("Failed to delete network resource: {}", e),
                        })
                    }
                }
            }
        }
    }

    /// Process Volume resource
    async fn process_volume(&self, yaml_str: &str, action: Action) -> Result<HandleResourceResponse, String> {
        let volume: Volume = serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("Failed to parse Volume YAML: {}", e))?;

        let spec = volume.get_spec();
        let volume_name = spec.get_volume_name().to_string();
        let capacity = spec.get_capacity().to_string();
        let mountpath = spec.get_mount_path().to_string();
        let asil_level = spec.get_asil_level().as_str().to_string();

        println!("RESOURCE MANAGER: Processing Volume Resource");
        println!("  Volume Name: {}", &volume_name);
        println!("  Capacity: {}", &capacity);
        println!("  Mount Path: {}", &mountpath);
        println!("  ASIL Level: {}", &asil_level);
        println!("  Action: {:?}", action);

        match action {
            Action::Apply => {
                let csi_req = VolumeCreateRequest {
                    volume_name: volume_name.clone(),
                    capacity,
                    mountpath,
                    asil_level,
                };

                println!("Sending VolumeCreateRequest to CSI");
                match self.csi_sender.clone().create_volume(csi_req).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        println!("Successfully created volume resource: {}", &volume_name);
                        Ok(HandleResourceResponse {
                            success: resp.success,
                            message: resp.message,
                        })
                    }
                    Err(e) => {
                        println!("Failed to create volume resource: {:?}", e);
                        Ok(HandleResourceResponse {
                            success: false,
                            message: format!("Failed to create volume resource: {}", e),
                        })
                    }
                }
            }
            Action::Withdraw => {
                let csi_req = VolumeDeleteRequest {
                    volume_name: volume_name.clone(),
                };

                println!("Sending VolumeDeleteRequest to CSI");
                match self.csi_sender.clone().delete_volume(csi_req).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        println!("Successfully deleted volume resource: {}", &volume_name);
                        Ok(HandleResourceResponse {
                            success: resp.success,
                            message: resp.message,
                        })
                    }
                    Err(e) => {
                        println!("Failed to delete volume resource: {:?}", e);
                        Ok(HandleResourceResponse {
                            success: false,
                            message: format!("Failed to delete volume resource: {}", e),
                        })
                    }
                }
            }
        }
    }
}

#[tonic::async_trait]
impl ResourceManagerService for ResourceManagerGrpcServer {
    async fn handle_resource(
        &self,
        request: tonic::Request<HandleResourceRequest>,
    ) -> Result<tonic::Response<HandleResourceResponse>, tonic::Status> {
        let req = request.into_inner();
        let yaml_str = &req.resource_yaml;
        let action = Action::try_from(req.action).unwrap_or(Action::Apply);

        println!("RESOURCE MANAGER: Received resource YAML");
        println!("  Action: {:?}", action);

        // Parse YAML to determine resource kind
        let kind = match Self::parse_artifact_kind(yaml_str) {
            Some(k) => k,
            None => {
                return Ok(Response::new(HandleResourceResponse {
                    success: false,
                    message: "Failed to parse resource kind from YAML".to_string(),
                }));
            }
        };

        println!("  Resource Kind: {}", &kind);

        // Process based on resource kind
        let result = match kind.as_str() {
            KIND_NETWORK => self.process_network(yaml_str, action).await,
            KIND_VOLUME => self.process_volume(yaml_str, action).await,
            _ => {
                return Ok(Response::new(HandleResourceResponse {
                    success: false,
                    message: format!("Unsupported resource kind: {}", kind),
                }));
            }
        };

        match result {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Ok(Response::new(HandleResourceResponse {
                success: false,
                message: e,
            })),
        }
    }
}
