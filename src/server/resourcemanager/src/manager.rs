/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Resource Manager module
//!
//! This module is responsible for:
//! - Parsing and validating resource YAML
//! - Determining resource kind
//! - Routing requests to appropriate handlers based on resource kind

use crate::grpc::sender::csi::CsiSender;
use crate::grpc::sender::pharos::PharosSender;
use common::external::csi::{VolumeCreateRequest, VolumeDeleteRequest};
use common::external::pharos::{NetworkRemoveRequest, NetworkSetupRequest};
use common::logd;
use common::resourcemanager::{Action, HandleResourceResponse};
use common::spec::artifact::{Network, Volume};

// Artifact kind constants
const KIND_NETWORK: &str = "Network";
const KIND_VOLUME: &str = "Volume";

/// ResourceManager handles YAML parsing, resource kind determination, and request routing
pub struct ResourceManager {
    /// Pharos sender for network resource operations
    pharos_sender: PharosSender,
    /// CSI sender for volume resource operations
    csi_sender: CsiSender,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceManager {
    /// Creates a new ResourceManager instance
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

    /// Helper function to create success response
    fn success_response(success: bool, message: String) -> HandleResourceResponse {
        HandleResourceResponse { success, message }
    }

    /// Helper function to create error response
    fn error_response(message: String) -> HandleResourceResponse {
        HandleResourceResponse {
            success: false,
            message,
        }
    }

    /// Handle resource request: parse YAML, determine kind, and route to appropriate handler
    pub async fn handle_resource(
        &mut self,
        yaml_str: &str,
        action: Action,
    ) -> HandleResourceResponse {
        logd!(1, "RESOURCE MANAGER: Processing resource request");
        logd!(2, "  Action: {:?}", action);

        // Parse YAML to determine resource kind
        let kind = match Self::parse_artifact_kind(yaml_str) {
            Some(k) => k,
            None => {
                return Self::error_response(
                    "Failed to parse resource kind from YAML".to_string(),
                );
            }
        };

        logd!(2, "  Resource Kind: {}", &kind);

        // Route to appropriate handler based on resource kind
        let result = match kind.as_str() {
            KIND_NETWORK => self.process_network(yaml_str, action).await,
            KIND_VOLUME => self.process_volume(yaml_str, action).await,
            _ => {
                return Self::error_response(format!("Unsupported resource kind: {}", kind));
            }
        };

        match result {
            Ok(response) => response,
            Err(e) => Self::error_response(e),
        }
    }

    /// Process Network resource
    async fn process_network(
        &mut self,
        yaml_str: &str,
        action: Action,
    ) -> Result<HandleResourceResponse, String> {
        let network: Network = serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("Failed to parse Network YAML: {}", e))?;

        let spec = network.get_spec();
        let network_name = spec.get_network_name().to_string();
        let network_mode = spec.get_network_mode().as_str().to_string();

        logd!(1, "RESOURCE MANAGER: Processing Network Resource");
        logd!(2, "  Network Name: {}", &network_name);
        logd!(2, "  Network Mode: {}", &network_mode);
        logd!(2, "  Action: {:?}", action);

        match action {
            Action::Apply => {
                let pharos_req = NetworkSetupRequest {
                    network_name: network_name.clone(),
                    network_mode,
                };

                logd!(2, "Sending NetworkSetupRequest to Pharos");
                match self.pharos_sender.setup_network(pharos_req).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        logd!(1, "Successfully created network resource: {}", &network_name);
                        Ok(Self::success_response(resp.success, resp.message))
                    }
                    Err(e) => {
                        logd!(4, "Failed to create network resource: {:?}", e);
                        Ok(Self::error_response(format!(
                            "Failed to create network resource: {}",
                            e
                        )))
                    }
                }
            }
            Action::Withdraw => {
                let pharos_req = NetworkRemoveRequest {
                    network_name: network_name.clone(),
                };

                logd!(2, "Sending NetworkRemoveRequest to Pharos");
                match self.pharos_sender.remove_network(pharos_req).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        logd!(1, "Successfully deleted network resource: {}", &network_name);
                        Ok(Self::success_response(resp.success, resp.message))
                    }
                    Err(e) => {
                        logd!(4, "Failed to delete network resource: {:?}", e);
                        Ok(Self::error_response(format!(
                            "Failed to delete network resource: {}",
                            e
                        )))
                    }
                }
            }
        }
    }

    /// Process Volume resource
    async fn process_volume(
        &mut self,
        yaml_str: &str,
        action: Action,
    ) -> Result<HandleResourceResponse, String> {
        let volume: Volume = serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("Failed to parse Volume YAML: {}", e))?;

        let spec = volume.get_spec();
        let volume_name = spec.get_volume_name().to_string();
        let capacity = spec.get_capacity().to_string();
        let mountpath = spec.get_mount_path().to_string();
        let asil_level = spec.get_asil_level().as_str().to_string();

        logd!(1, "RESOURCE MANAGER: Processing Volume Resource");
        logd!(2, "  Volume Name: {}", &volume_name);
        logd!(2, "  Capacity: {}", &capacity);
        logd!(2, "  Mount Path: {}", &mountpath);
        logd!(2, "  ASIL Level: {}", &asil_level);
        logd!(2, "  Action: {:?}", action);

        match action {
            Action::Apply => {
                let csi_req = VolumeCreateRequest {
                    volume_name: volume_name.clone(),
                    capacity,
                    mountpath,
                    asil_level,
                };

                logd!(2, "Sending VolumeCreateRequest to CSI");
                match self.csi_sender.create_volume(csi_req).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        logd!(1, "Successfully created volume resource: {}", &volume_name);
                        Ok(Self::success_response(resp.success, resp.message))
                    }
                    Err(e) => {
                        logd!(4, "Failed to create volume resource: {:?}", e);
                        Ok(Self::error_response(format!(
                            "Failed to create volume resource: {}",
                            e
                        )))
                    }
                }
            }
            Action::Withdraw => {
                let csi_req = VolumeDeleteRequest {
                    volume_name: volume_name.clone(),
                };

                logd!(2, "Sending VolumeDeleteRequest to CSI");
                match self.csi_sender.delete_volume(csi_req).await {
                    Ok(response) => {
                        let resp = response.into_inner();
                        logd!(1, "Successfully deleted volume resource: {}", &volume_name);
                        Ok(Self::success_response(resp.success, resp.message))
                    }
                    Err(e) => {
                        logd!(4, "Failed to delete volume resource: {:?}", e);
                        Ok(Self::error_response(format!(
                            "Failed to delete volume resource: {}",
                            e
                        )))
                    }
                }
            }
        }
    }
}
