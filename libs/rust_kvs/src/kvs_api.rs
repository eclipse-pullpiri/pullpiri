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
use crate::kvs_value::KvsValue;
use core::fmt;
use std::path::PathBuf;

/// Instance ID
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceId(pub usize);

impl fmt::Display for InstanceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<InstanceId> for usize {
    fn from(value: InstanceId) -> Self {
        value.0
    }
}

/// Snapshot ID
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapshotId(pub usize);

impl fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<SnapshotId> for usize {
    fn from(value: SnapshotId) -> Self {
        value.0
    }
}

/// Defaults handling mode.
#[derive(Clone, Debug, PartialEq)]
pub enum KvsDefaults {
    /// Defaults are not loaded.
    Ignored,

    /// Defaults are loaded if available.
    Optional,

    /// Defaults must be loaded.
    Required,
}

/// KVS load mode.
#[derive(Clone, Debug, PartialEq)]
pub enum KvsLoad {
    /// KVS is not loaded.
    Ignored,

    /// KVS is loaded if available.
    Optional,

    /// KVS must be loaded.
    Required,
}

pub trait KvsApi {
    fn reset(&self) -> Result<(), ErrorCode>;
    fn reset_key(&self, key: &str) -> Result<(), ErrorCode>;
    fn get_all_keys(&self) -> Result<Vec<String>, ErrorCode>;
    fn key_exists(&self, key: &str) -> Result<bool, ErrorCode>;
    fn get_value(&self, key: &str) -> Result<KvsValue, ErrorCode>;
    fn get_value_as<T>(&self, key: &str) -> Result<T, ErrorCode>
    where
        for<'a> T: TryFrom<&'a KvsValue> + Clone,
        for<'a> <T as TryFrom<&'a KvsValue>>::Error: std::fmt::Debug;
    fn get_default_value(&self, key: &str) -> Result<KvsValue, ErrorCode>;
    fn is_value_default(&self, key: &str) -> Result<bool, ErrorCode>;
    fn set_value<S: Into<String>, J: Into<KvsValue>>(
        &self,
        key: S,
        value: J,
    ) -> Result<(), ErrorCode>;
    fn remove_key(&self, key: &str) -> Result<(), ErrorCode>;
    fn flush(&self) -> Result<(), ErrorCode>;
    fn snapshot_count(&self) -> usize;
    fn snapshot_max_count(&self) -> usize;
    fn snapshot_restore(&self, snapshot_id: SnapshotId) -> Result<(), ErrorCode>;
    fn get_kvs_filename(&self, snapshot_id: SnapshotId) -> Result<PathBuf, ErrorCode>;
    fn get_hash_filename(&self, snapshot_id: SnapshotId) -> Result<PathBuf, ErrorCode>;
}

#[cfg(test)]
mod kvs_api_tests {
    use crate::kvs_api::{InstanceId, SnapshotId};

    #[test]
    fn test_instance_id_to_string() {
        let id = InstanceId(123);
        assert_eq!(id.to_string(), "123");
    }

    #[test]
    fn test_instance_id_to_usize() {
        let id = InstanceId(999);
        assert_eq!(usize::from(id), 999);
    }

    #[test]
    fn test_snapshot_id_fmt() {
        let id = SnapshotId(4321);
        assert_eq!(id.to_string(), "4321");
    }

    #[test]
    fn test_snapshot_id_to_usize() {
        let id = SnapshotId(0);
        assert_eq!(usize::from(id), 0);
    }
}
