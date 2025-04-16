use super::Artifact;
use super::Model;

pub type ModelSpec = crate::spec::k8s::pod::PodSpec;

impl Artifact for Model {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Model {
    pub fn get_name(&self) -> String {
        self.metadata.name.clone()
    }

    pub fn get_podspec(&self) -> ModelSpec {
        self.spec.clone()
    }
}
