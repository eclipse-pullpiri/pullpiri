/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use super::Artifact;
use super::Volume;

impl Artifact for Volume {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Volume {
    pub fn get_spec(&self) -> &VolumeSpec {
        &self.spec
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VolumeSpec {
    #[serde(rename = "volumeName")]
    volume_name: String,

    capacity: String, // e.g., "10Gi", "500Mi"

    #[serde(rename = "mountPath")]
    mount_path: String,

    #[serde(rename = "asilLevel")]
    asil_level: AsilLevel,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum AsilLevel {
    QM, // Quality Managed
    A,
    B,
    C,
    D,
}

impl VolumeSpec {
    pub fn get_volume_name(&self) -> &str {
        &self.volume_name
    }

    pub fn get_capacity(&self) -> &str {
        &self.capacity
    }

    pub fn get_mount_path(&self) -> &str {
        &self.mount_path
    }

    pub fn get_asil_level(&self) -> &AsilLevel {
        &self.asil_level
    }
}

impl AsilLevel {
    pub fn as_str(&self) -> &str {
        match self {
            AsilLevel::QM => "QM",
            AsilLevel::A => "A",
            AsilLevel::B => "B",
            AsilLevel::C => "C",
            AsilLevel::D => "D",
        }
    }
}
