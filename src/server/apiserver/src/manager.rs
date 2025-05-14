/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Controls the flow of data between each module.

use common::filtergateway::{Action, HandleScenarioRequest};

/// Launch REST API listener and reload scenario data in etcd
pub async fn initialize() {
    tokio::join!(crate::route::launch_tcp_listener(), reload());
}

/// (under construction) Send request message to piccolo cloud
///
/// ### Parametets
/// TBD
/// ### Description
/// TODO
#[allow(dead_code)]
async fn send_download_request() {}

/// Reload all scenario data in etcd
///
/// ### Parametets
/// * None
/// ### Description
/// This function is called once when the apiserver starts.
async fn reload() {
    let scenarios_result = crate::artifact::data::read_all_scenario_from_etcd().await;

    if let Ok(scenarios) = scenarios_result {
        for scenario in scenarios {
            let req = HandleScenarioRequest {
                action: Action::Apply.into(),
                scenario,
            };
            if let Err(status) = crate::grpc::sender::filtergateway::send(req).await {
                println!("{:#?}", status);
            }
        }
    } else {
        println!("{:#?}", scenarios_result);
    }
}

/// Apply downloaded artifact
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Description
/// write artifact in etcd  
/// (optional) make yaml, kube files for Bluechi  
/// send a gRPC message to gateway
pub async fn apply_artifact(body: &str) -> common::Result<()> {
    let (scenario, package) = crate::artifact::apply(body).await?;

    crate::bluechi::parse(package).await?;

    let req = HandleScenarioRequest {
        action: Action::Apply.into(),
        scenario,
    };
    crate::grpc::sender::filtergateway::send(req).await?;

    Ok(())
}

/// Withdraw downloaded artifact
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Description
/// delete artifact in etcd  
/// (optional) delete yaml, kube files for Bluechi  
/// send a gRPC message to gateway
pub async fn withdraw_artifact(body: &str) -> common::Result<()> {
    let scenario = crate::artifact::withdraw(body).await?;

    let req = HandleScenarioRequest {
        action: Action::Withdraw.into(),
        scenario,
    };
    crate::grpc::sender::filtergateway::send(req).await?;

    Ok(())
}

//UNIT TEST CASES
#[cfg(test)]
mod tests {
    use super::*;
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
    io.piccolo.annotations.package-name: helloworldwithdraw_artifact
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
  pattern: //Missing Pattern List
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
    - name: helloworld-corewithdraw_artifact
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
    io.piccolo.annotations.package-network: defaultwithdraw_artifact
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

    // Test for `apply_artifact` - successful casewithdraw_artifact
    #[tokio::test]
    async fn test_apply_artifact_success() {
        let result = apply_artifact(VALID_ARTIFACT_YAML).await;
        assert!(
            result.is_ok(),
            "apply_artifact() failed: {:?}",
            result.err()
        );
    }

    // Test for `apply_artifact` - Success when passing known/Unknown artifact Yaml
    #[tokio::test]
    async fn test_apply_artifact_known_unknown_yaml() {
        let result = apply_artifact(VALID_ARTIFACT_YAML_KNOWN_UNKNOWN).await;
        assert!(
            result.is_ok(),
            "apply_artifact() failed: {:?}",
            result.err()
        );
    }

    // Test for `apply_artifact` - failure due to missing `action` field
    #[tokio::test]
    async fn test_apply_artifact_failure_missing_action() {
        let result = apply_artifact(INVALID_ARTIFACT_YAML_MISSING_ACTION).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with missing `action` field"
        );
    }

    // Test for `apply_artifact` - failure due to missing required fields (`kind` and `metadata.name`)
    #[tokio::test]
    async fn test_apply_artifact_failure_missing_required_fields() {
        let result = apply_artifact(INVALID_ARTIFACT_YAML_MISSING_REQUIRED_FIELDS).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with missing required fields"
        );
    }

    // Test for `apply_artifact` - failure due to malformed structure (Missing the list of patterns)
    #[tokio::test]
    async fn test_apply_artifact_failure_malformed_structure() {
        let result = apply_artifact(INVALID_ARTIFACT_YAML_MALFORMED_STRUCTURE).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with malformed structure"
        );
    }

    // Test for `apply_artifact` - failure due to extra fields (`target` not under `spec`)
    #[tokio::test]
    async fn test_apply_artifact_failure_extra_fields() {
        let result = apply_artifact(INVALID_ARTIFACT_YAML_EXTRA_FIELDS).await;
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with extra fields outside of `spec`"
        );
    }

    // Test for `apply_artifact` - failure due to Empty Yaml
    #[tokio::test]
    async fn test_apply_artifact_empty_yaml() {
        let result = apply_artifact(INVALID_ARTIFACT_YAML_EMPTY).await;
        // Check if it's an error and print it
        if let Err(e) = &result {
            println!("apply_artifact() failed with error: {:?}", e);
        }
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with empty yaml"
        );
    }

    // Test for `apply_artifact` - failure due to Unknown Artifact Yaml
    #[tokio::test]
    async fn test_apply_artifact_unknown_yaml() {
        let result = apply_artifact(INVALID_ARTIFACT_YAML_UNKNOWN).await;
        // Check if it's an error and print it
        if let Err(e) = &result {
            println!("apply_artifact() failed with error: {:?}", e);
        }
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with UNKNOWN yaml"
        );
    }

    // Test for `apply_artifact` - failure due to invalid known/unknown without scenario Artifact Yaml
    #[tokio::test]
    async fn test_apply_artifact_invalid_known_unknown_without_scenario_yaml() {
        let result = apply_artifact(INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_SCENARIO).await;
        // Check if it's an error and print it
        if let Err(e) = &result {
            println!("apply_artifact() failed with error: {:?}", e);
        }
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with INVALID Unknown/kNOWN Artifact Yaml"
        );
    }

    // Test for `apply_artifact` - failure due to invalid known/unknown without package Artifact Yaml
    #[tokio::test]
    async fn test_apply_artifact_invalid_known_unknown_without_package_yaml() {
        let result = apply_artifact(INVALID_ARTIFACT_YAML_KNOWN_UNKNOWN_WITHOUT_PACKAGE).await;
        // Check if it's an error and print it
        if let Err(e) = &result {
            println!("apply_artifact() failed with error: {:?}", e);
        }
        assert!(
            result.is_err(),
            "apply_artifact() unexpectedly succeeded with INVALID Unknown/kNOWN Artifact Yaml"
        );
    }

    // Test for `withdraw_artifact` - successful case
    #[tokio::test]
    async fn test_withdraw_artifact_success() {
        let result = withdraw_artifact(VALID_ARTIFACT_YAML).await;
        assert!(
            result.is_ok(),
            "withdraw_artifact() failed: {:?}",
            result.err()
        );
    }

    // Test for `withdraw_artifact` - failure due to missing `action` field
    #[tokio::test]
    async fn test_withdraw_artifact_failure_missing_action() {
        let result = withdraw_artifact(INVALID_ARTIFACT_YAML_MISSING_ACTION).await;
        assert!(
            result.is_err(),
            "withdraw_artifact() unexpectedly succeeded with missing `action` field"
        );
    }

    // Test for `withdraw_artifact` - failure due to missing required fields
    #[tokio::test]
    async fn test_withdraw_artifact_failure_missing_required_fields() {
        let result = withdraw_artifact(INVALID_ARTIFACT_YAML_MISSING_REQUIRED_FIELDS).await;
        assert!(
            result.is_err(),
            "withdraw_artifact() unexpectedly succeeded with missing required fields"
        );
    }

    // Test for `withdraw_artifact` - failure due to malformed structure
    #[tokio::test]
    async fn test_withdraw_artifact_failure_malformed_structure() {
        let result = withdraw_artifact(INVALID_ARTIFACT_YAML_MALFORMED_STRUCTURE).await;
        assert!(
            result.is_err(),
            "withdraw_artifact() unexpectedly succeeded with malformed structure"
        );
    }

    // Test for `withdraw_artifact` - failure due to extra fields
    #[tokio::test]
    async fn test_withdraw_artifact_failure_extra_fields() {
        let result = withdraw_artifact(INVALID_ARTIFACT_YAML_EXTRA_FIELDS).await;
        assert!(
            result.is_err(),
            "withdraw_artifact() unexpectedly succeeded with extra fields outside of `spec`"
        );
    }

    // Test for `reload()` - successful case
    #[tokio::test]
    async fn test_reload_success() {
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), reload()).await;
        assert!(result.is_ok(), "reload() failed to complete in time");
    }

    // Test for `send_download_request()` - currently unimplemented (but we can still test its existence)
    #[tokio::test]
    async fn test_send_download_request() {
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(5), send_download_request()).await;
        assert!(result.is_ok(), "send_download_request() failed to execute");
    }
}
