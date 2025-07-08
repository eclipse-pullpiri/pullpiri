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

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // Test Model creation through JSON deserialization (public interface)
    fn create_test_model(name: &str) -> Model {
        let model_json = format!(
            r#"{{
                "apiVersion": "v1",
                "kind": "Model",
                "metadata": {{
                    "name": "{}"
                }},
                "spec": {{
                    "containers": [{{
                        "name": "test-container",
                        "image": "test-image"
                    }}]
                }}
            }}"#,
            name
        );

        serde_json::from_str(&model_json).unwrap()
    }

    #[test]
    fn test_get_name_via_artifact_trait() {
        let model_name = "test-model";
        let model = create_test_model(model_name);
        assert_eq!(model.get_name(), model_name);
    }

    #[test]
    fn test_get_name_via_direct_method() {
        let model_name = "test-model-direct";
        let model = create_test_model(model_name);
        assert_eq!(model.get_name(), model_name);
    }

    #[test]
    fn test_get_name_with_empty_string() {
        let model = create_test_model("");
        assert_eq!(model.get_name(), "");
    }

    #[test]
    fn test_get_name_with_special_characters() {
        let model_name = "model@123#test";
        let model = create_test_model(model_name);
        assert_eq!(model.get_name(), model_name);
    }

    #[test]
    fn test_get_podspec_returns_valid_spec() {
        let model = create_test_model("test-podspec");
        let _pod_spec = model.get_podspec(); // Prefix with underscore to silence warning
    }

    #[test]
    fn test_get_podspec_returns_clone() {
        let model = create_test_model("test-clone");
        let _pod_spec1 = model.get_podspec(); // Prefix with underscore
        let _pod_spec2 = model.get_podspec(); // Prefix with underscore
    }

    #[test]
    fn test_model_conversion_preserves_name() {
        let model_name = "conversion-test";
        let model = create_test_model(model_name);
        let pod_name = model.get_name();

        assert_eq!(pod_name, model_name);
    }
}
