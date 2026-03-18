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
use crate::kvs_api::{KvsApi, SnapshotId};
use crate::kvs_value::{KvsMap, KvsValue};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MockKvs {
    pub map: Arc<Mutex<KvsMap>>,
    pub fail: bool,
}

impl Default for MockKvs {
    fn default() -> Self {
        let map = Arc::new(Mutex::new(KvsMap::new()));
        Self { map, fail: false }
    }
}

impl MockKvs {
    pub fn new(kvs_map: KvsMap, fail: bool) -> Result<Self, ErrorCode> {
        let map = Arc::new(Mutex::new(kvs_map));
        Ok(MockKvs { map, fail })
    }
}

impl KvsApi for MockKvs {
    fn reset(&self) -> Result<(), ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        self.map.lock().unwrap().clear();
        Ok(())
    }
    fn reset_key(&self, key: &str) -> Result<(), ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        let mut map = self.map.lock().unwrap();
        if map.contains_key(key) {
            map.remove(key);
            Ok(())
        } else {
            Err(ErrorCode::KeyDefaultNotFound)
        }
    }
    fn get_all_keys(&self) -> Result<Vec<String>, ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        Ok(self.map.lock().unwrap().keys().cloned().collect())
    }
    fn key_exists(&self, key: &str) -> Result<bool, ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        Ok(self.map.lock().unwrap().contains_key(key))
    }
    fn get_value(&self, key: &str) -> Result<KvsValue, ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        self.map
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or(ErrorCode::KeyNotFound)
    }
    fn get_value_as<T>(&self, key: &str) -> Result<T, ErrorCode>
    where
        for<'a> T: TryFrom<&'a KvsValue> + Clone,
        for<'a> <T as TryFrom<&'a KvsValue>>::Error: std::fmt::Debug,
    {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        let v = self.get_value(key)?;
        T::try_from(&v).map_err(|_| ErrorCode::ConversionFailed)
    }
    fn get_default_value(&self, _key: &str) -> Result<KvsValue, ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        Err(ErrorCode::KeyNotFound)
    }
    fn is_value_default(&self, _key: &str) -> Result<bool, ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        Ok(false)
    }
    fn set_value<S: Into<String>, V: Into<KvsValue>>(
        &self,
        key: S,
        value: V,
    ) -> Result<(), ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        self.map.lock().unwrap().insert(key.into(), value.into());
        Ok(())
    }
    fn remove_key(&self, key: &str) -> Result<(), ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        self.map.lock().unwrap().remove(key);
        Ok(())
    }
    fn flush(&self) -> Result<(), ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        Ok(())
    }
    fn snapshot_count(&self) -> usize {
        if self.fail {
            return 9999;
        }
        0
    }
    fn snapshot_max_count(&self) -> usize {
        0
    }
    fn snapshot_restore(&self, _id: SnapshotId) -> Result<(), ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        Ok(())
    }
    fn get_kvs_filename(&self, _id: SnapshotId) -> Result<std::path::PathBuf, ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        Err(ErrorCode::FileNotFound)
    }
    fn get_hash_filename(&self, _id: SnapshotId) -> Result<std::path::PathBuf, ErrorCode> {
        if self.fail {
            return Err(ErrorCode::UnmappedError);
        }
        Err(ErrorCode::FileNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kvs_value::KvsValue;
    use KvsApi;
    use SnapshotId;

    #[test]
    fn test_mock_kvs_pass_and_fail_cases() {
        // Pass case
        let kvs = MockKvs::default();
        assert!(kvs.set_value("a", 1.0).is_ok());
        assert_eq!(kvs.get_value("a").unwrap(), KvsValue::from(1.0));
        assert_eq!(kvs.get_all_keys().unwrap(), vec!["a".to_string()]);
        assert!(kvs.key_exists("a").unwrap());
        assert!(kvs.remove_key("a").is_ok());
        assert!(!kvs.key_exists("a").unwrap());
        assert_eq!(kvs.snapshot_count(), 0);
        assert!(kvs.flush().is_ok());
        assert!(kvs.reset().is_ok());

        // Failure case
        let kvs_fail = MockKvs {
            fail: true,
            ..Default::default()
        };
        assert!(kvs_fail.set_value("a", 1.0).is_err());
        assert!(kvs_fail.get_value("a").is_err());
        assert!(kvs_fail.get_all_keys().is_err());
        assert!(kvs_fail.key_exists("a").is_err());
        assert!(kvs_fail.remove_key("a").is_err());
        assert_eq!(kvs_fail.snapshot_count(), 9999);
        assert!(kvs_fail.flush().is_err());
        assert!(kvs_fail.reset().is_err());
        assert!(kvs_fail.reset_key("a").is_err());
        assert!(kvs_fail.get_default_value("a").is_err());
        assert!(kvs_fail.is_value_default("a").is_err());
        assert!(kvs_fail.get_kvs_filename(SnapshotId(0)).is_err());
        assert!(kvs_fail.get_hash_filename(SnapshotId(0)).is_err());
        assert!(kvs_fail.snapshot_restore(SnapshotId(0)).is_err());
    }
}
