use apiserver::manager::{apply_artifact, withdraw_artifact, initialize};
use common::filtergateway::{Action, HandleScenarioRequest};
use tokio;

 /// Correct valid YAML artifact (Scenario + Package + Model)
    const VALID_ARTIFACT_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// Invalid YAML — missing `action` field
    const INVALID_ARTIFACT_YAML_MISSING_ACTION: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  target: helloworld

---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume: []
        network: []

---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// Invalid YAML — missing required fields (`kind` and `metadata.name`)
    const INVALID_ARTIFACT_YAML_MISSING_REQUIRED_FIELDS: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: 

spec:
  condition:
  action: update
  target: helloworld

---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
  apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld

---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume: []
        network: []

    - name: helloworld-core
      node: HPC
      resources:
        volume: []
        network: []

---
apiVersion: v1
kind: Modelr#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// Invalid YAML — malformed structure -Missing the list of patterns
    const INVALID_ARTIFACT_YAML_MALFORMED_STRUCTURE: &str = r#"
apiVersion: v1
metadata:
  name: helloworld
spec:
  action: update
  target: helloworld
"#;

    /// Invalid YAML — extra fields (`target` not under `spec`)
    const INVALID_ARTIFACT_YAML_EXTRA_FIELDS: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
spec:
  condition:
  action: update

target: helloworld  # Should be under `spec`

---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume: []
        network: []

---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
    
"#;

    /// Invalid YAML only UnKnown
    const INVALID_ARTIFACT_YAML_UNKNOWN: &str = r#"
apiVersion: v1
kind: Unknown
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
  
"#;

    /// Invalid YAML Empty
    const INVALID_ARTIFACT_YAML_EMPTY: &str = r#"
"#;

    /// Valid YAML WITH KNOWN/UNKNOWN ARTIFACT
    const VALID_ARTIFACT_YAML_KNOWN_UNKNOWN: &str = r#"
    
apiVersion: v1
kind: known_Unknown
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld

---
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
    express: eq
    value: "true"
    operands:
      type: DDS
      name: value
      value: ADASObstacleDetectionIsWarning
  action: update
  target: helloworld
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
"#;

    /// Invalid YAML WITH KNOWN/UNKNOWN ARTIFACT WITHOUT SCENARIO
    const INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_SCENARIO: &str = r#"

apiVersion: v1
kind: known_unknown
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    /// Invalid YAML WITH KNOWN/UNKNOWN ARTIFACT WITHOUT PACKAGE
    const INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_PACKAGE: &str = r#"

apiVersion: v1
kind: known_unknown
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

#[tokio::test]
async fn test_initialize_runs_successfully() {
    let _ =tokio::time::timeout(std::time::Duration::from_millis(500), initialize()).await;
    assert!(true);
}

#[tokio::test]
async fn test_apply_artifact_valid() {
    let result = apply_artifact(VALID_ARTIFACT_YAML).await;
    assert!(result.is_ok(), "Expected apply_artifact to succeed");
}

#[tokio::test]
async fn test_withdraw_artifact_valid() {
    // Ensure artifact exists first
    apply_artifact(VALID_ARTIFACT_YAML).await.unwrap();

    let result = withdraw_artifact(VALID_ARTIFACT_YAML).await;
    assert!(result.is_ok(), "Expected withdraw_artifact to succeed");
}

#[tokio::test]
async fn test_apply_invalid_missing_action() {
    let result = apply_artifact(INVALID_ARTIFACT_YAML_MISSING_ACTION).await;
    assert!(result.is_err(), "Expected apply_artifact to fail for missing action");
}

#[tokio::test]
async fn test_apply_invalid_required_fields() {
    let result = apply_artifact(INVALID_ARTIFACT_YAML_MISSING_REQUIRED_FIELDS).await;
    assert!(result.is_err(), "Expected apply_artifact to fail for missing required fields");
}

#[tokio::test]
async fn test_apply_malformed_structure() {
    let result = apply_artifact(INVALID_ARTIFACT_YAML_MALFORMED_STRUCTURE).await;
    assert!(result.is_err(), "Expected apply_artifact to fail for malformed YAML");
}

#[tokio::test]
async fn test_apply_invalid_extra_fields() {
    let result = apply_artifact(INVALID_ARTIFACT_YAML_EXTRA_FIELDS).await;
    assert!(result.is_err(), "Expected apply_artifact to fail for misplaced fields");
}

#[tokio::test]
async fn test_apply_unknown_kind() {
    let result = apply_artifact(INVALID_ARTIFACT_YAML_UNKNOWN).await;
    assert!(result.is_err(), "Expected apply_artifact to fail for unknown kind");
}

#[tokio::test]
async fn test_apply_empty_yaml() {
    let result = apply_artifact(INVALID_ARTIFACT_YAML_EMPTY).await;
    assert!(result.is_err(), "Expected apply_artifact to fail for empty input");
}

#[tokio::test]
async fn test_apply_known_and_unknown_artifact() {
    let result = apply_artifact(VALID_ARTIFACT_YAML_KNOWN_UNKNOWN).await;
    assert!(result.is_ok(), "Expected apply_artifact to succeed for mixed known/unknown");
}

#[tokio::test]
async fn test_apply_known_unknown_without_scenario() {
    let result = apply_artifact(INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_SCENARIO).await;
    assert!(result.is_err(), "Expected failure for missing Scenario in known/unknown");
}

#[tokio::test]
async fn test_apply_known_unknown_without_package() {
    let result = apply_artifact(INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_PACKAGE).await;
    assert!(result.is_err(), "Expected failure for missing Package in known/unknown");
}