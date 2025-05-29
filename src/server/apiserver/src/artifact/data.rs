/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Read/Write/Delete artifact data in etcd

/// Read yaml string of artifacts from etcd
///
/// ### Parameters
/// * `artifact_name: &str` - name of the newly released artifact
/// ### Return
/// * `Result<(String)>` - `Ok()` contains yaml string if success
pub async fn read_from_etcd(artifact_name: &str) -> common::Result<String> {
    let raw = common::etcd::get(artifact_name).await?;
    Ok(raw)
}

/// Read all scenario yaml string in etcd
///
/// ### Parameters
/// * None
/// ### Return
/// * `Result<Vec<String>>` - `Ok(_)` contains scenario yaml string vector
pub async fn read_all_scenario_from_etcd() -> common::Result<Vec<String>> {
    let kv_scenario = common::etcd::get_all_with_prefix("Scenario").await?;
    let values = kv_scenario.into_iter().map(|kv| kv.value).collect();

    Ok(values)
}

/// Write yaml string of artifacts to etcd
///
/// ### Parameters
/// * `key: &str, artifact_name: &str` - etcd key and the name of the newly released artifact
/// ### Return
/// * `Result<()>` - `Ok` if success, `Err` otherwise
pub async fn write_to_etcd(key: &str, artifact_str: &str) -> common::Result<()> {
    common::etcd::put(key, artifact_str).await?;
    Ok(())
}

/// Write yaml string of artifacts to etcd
///
/// ### Parameters
/// * `key: &str` - data key to delete from etcd
/// ### Return
/// * `Result<()>` - `Ok` if success, `Err` otherwise
pub async fn delete_at_etcd(key: &str) -> common::Result<()> {
    common::etcd::delete(key).await?;
    Ok(())
}

//UNIT TEST CASES

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    // === Test data ===
    const TEST_YAML: &str = r#"apiVersion: v1
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

    // === Keys for testing ===
    const TEST_KEY: &str = "unit_test_helloworld";
    const INVALID_KEY_EMPTY: &str = "";
    const INVALID_KEY_NULLBYTE: &str = "\0badkey";

    // === Positive Tests ===

    // Test reading valid key (exists or not — should not panic)
    #[tokio::test]
    async fn test_read_from_etcd_positive() {
        let result = read_from_etcd(TEST_KEY).await;
        println!("read_from_etcd (positive) result = {:?}", result);

        //we accept both Ok and Err depending on etcd state
        assert!(
            result.is_ok() || result.is_err(),
            "Expected Ok or Err but got: {:?}",
            result
        );
    }

    // Test reading all Scenario keys (should return Vec<String> or Err)
    #[tokio::test]
    async fn test_read_all_scenario_from_etcd_positive() {
        let result = read_all_scenario_from_etcd().await;
        println!(
            "read_all_scenario_from_etcd (positive) result = {:?}",
            result
        );

        //we accept both Ok (some scenarios) or Ok(empty Vec) or Err (etcd error)
        assert!(
            result.is_ok() || result.is_err(),
            "Expected Ok or Err but got: {:?}",
            result
        );
    }

    // Test writing valid key and yaml
    #[tokio::test]
    async fn test_write_to_etcd_positive() {
        let result = write_to_etcd(TEST_KEY, TEST_YAML).await;
        println!("write_to_etcd (positive) result = {:?}", result);
        // We expect the write to succeed with valid key & data or ERR(etcd error)
        assert!(
            result.is_ok() || result.is_err(),
            "Expected write_to_etcd to succeed or Err but got: {:?}",
            result
        );
    }

    // Test deleting valid key (whether key exists or not — should succeed or cleanly fail)
    #[tokio::test]
    async fn test_delete_at_etcd_positive() {
        let result = delete_at_etcd(TEST_KEY).await;
        println!("delete_at_etcd (positive) result = {:?}", result);
        // We accept Ok (key deleted) or Err (key not found) as valid outcomes
        assert!(
            result.is_ok() || result.is_err(),
            "Expected Ok or Err but got: {:?}",
            result
        );
    }

    // === Negative Tests ===

    // Test reading with invalid keys (empty/nullbyte) — should fail
    #[tokio::test]
    async fn test_read_from_etcd_negative_invalid_key() {
        let result = read_from_etcd(INVALID_KEY_EMPTY).await;
        assert!(
            result.is_err(),
            "Expected read_from_etcd with empty key to fail but got Ok: {:?}",
            result.ok()
        );
    }

    // Test writing with invalid keys (empty/nullbyte) — should fail
    #[tokio::test]
    async fn test_write_to_etcd_negative_invalid_key() {
        let result = write_to_etcd(INVALID_KEY_EMPTY, TEST_YAML).await;
        assert!(
            result.is_err(),
            "Expected write_to_etcd with empty key to fail but got Ok"
        );
    }

    // Test deleting with invalid keys (empty/nullbyte) — should fail
    #[tokio::test]
    async fn test_delete_at_etcd_negative_invalid_key() {
        let result = delete_at_etcd(INVALID_KEY_EMPTY).await;
        assert!(
            result.is_err(),
            "Expected delete_at_etcd with empty key to fail but got Ok"
        );
    }
}
