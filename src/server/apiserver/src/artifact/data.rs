/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

/// Read yaml string of artifacts from etcd
///
/// # parameters
/// * `artifact_name: &str` - name of the newly released artifact
/// # return
/// * `Result<(String)>` - `Ok()` contains yaml string if success
pub async fn read_from_etcd(artifact_name: &str) -> common::Result<String> {
    let raw = common::etcd::get(artifact_name).await?;
    Ok(raw)
}

/// Write yaml string of artifacts to etcd
///
/// # parameters
/// * `artifact_name: &str` - name of the newly released artifact
/// # return
/// * `Result<()>` - `Ok` if success, `Err` otherwise
pub async fn write_to_etcd(key: &str, artifact_str: &str) -> common::Result<()> {
    common::etcd::put(key, artifact_str).await?;
    Ok(())
}

/// Write yaml string of artifacts to etcd
///
/// # parameters
/// * `artifact_name: &str` - name of the newly released artifact
/// # return
/// * `Result<()>` - `Ok` if success, `Err` otherwise
pub async fn delete_at_etcd(key: &str) -> common::Result<()> {
    common::etcd::delete(key).await?;
    Ok(())
}

// TODO
// yaml to struct
// struct to yaml

/*fn export_artifact_data() {

}*/

pub async fn reload() {}
