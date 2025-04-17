use super::Artifact;
use super::Volume;

impl Artifact for Volume {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Volume {
    pub fn get_spec(&self) -> &Option<VolumeSpec> {
        &self.spec
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VolumeSpec {
    volumes: Option<Vec<crate::spec::k8s::pod::Volume>>,
}

impl VolumeSpec {
    pub fn get_volume(&self) -> &Option<Vec<crate::spec::k8s::pod::Volume>> {
        &self.volumes
    }
}
