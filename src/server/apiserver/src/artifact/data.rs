/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

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
/// * `artifact_name: &str` - name of the newly released artifact
/// ### Return
/// * `Result<()>` - `Ok` if success, `Err` otherwise
pub async fn write_to_etcd(key: &str, artifact_str: &str) -> common::Result<()> {
    common::etcd::put(key, artifact_str).await?;
    Ok(())
}

/// Write yaml string of artifacts to etcd
///
/// ### Parameters
/// * `artifact_name: &str` - name of the newly released artifact
/// ### Return
/// * `Result<()>` - `Ok` if success, `Err` otherwise
pub async fn delete_at_etcd(key: &str) -> common::Result<()> {
    common::etcd::delete(key).await?;
    Ok(())
}
