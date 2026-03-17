// Copyright (c) 2025 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache License Version 2.0 which is available at
// <https://www.apache.org/licenses/LICENSE-2.0>
//
// SPDX-License-Identifier: Apache-2.0

use crate::error_code::ErrorCode;
use crate::kvs_api::{InstanceId, SnapshotId};
use crate::kvs_value::KvsMap;
use std::path::{Path, PathBuf};

/// KVS backend interface.
pub trait KvsBackend {
    /// Load KvsMap from given file.
    fn load_kvs(kvs_path: &Path, hash_path: Option<&PathBuf>) -> Result<KvsMap, ErrorCode>;

    /// Store KvsMap at given file path.
    fn save_kvs(
        kvs_map: &KvsMap,
        kvs_path: &Path,
        hash_path: Option<&PathBuf>,
    ) -> Result<(), ErrorCode>;
}

/// KVS path resolver interface.
pub trait KvsPathResolver {
    /// Get KVS file name.
    fn kvs_file_name(instance_id: InstanceId, snapshot_id: SnapshotId) -> String;

    /// Get KVS file path in working directory.
    fn kvs_file_path(
        working_dir: &Path,
        instance_id: InstanceId,
        snapshot_id: SnapshotId,
    ) -> PathBuf;

    /// Get hash file name.
    fn hash_file_name(instance_id: InstanceId, snapshot_id: SnapshotId) -> String;

    /// Get hash file path in working directory.
    fn hash_file_path(
        working_dir: &Path,
        instance_id: InstanceId,
        snapshot_id: SnapshotId,
    ) -> PathBuf;

    /// Get defaults file name.
    fn defaults_file_name(instance_id: InstanceId) -> String;

    /// Get defaults file path in working directory.
    fn defaults_file_path(working_dir: &Path, instance_id: InstanceId) -> PathBuf;
}
