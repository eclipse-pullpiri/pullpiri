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
use crate::kvs::{GenericKvs, KvsParameters};
use crate::kvs_api::{InstanceId, KvsDefaults, KvsLoad, SnapshotId};
use crate::kvs_backend::{KvsBackend, KvsPathResolver};
use crate::kvs_value::KvsMap;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};

/// Maximum number of instances.
const KVS_MAX_INSTANCES: usize = 10;

/// KVS instance data.
/// Expected to be shared between instance pool and instances.
pub(crate) struct KvsData {
    /// Storage data.
    pub(crate) kvs_map: KvsMap,

    /// Optional default values.
    pub(crate) defaults_map: KvsMap,
}

impl From<PoisonError<MutexGuard<'_, KvsData>>> for ErrorCode {
    fn from(_cause: PoisonError<MutexGuard<'_, KvsData>>) -> Self {
        ErrorCode::MutexLockFailed
    }
}

/// KVS instance inner representation.
pub(crate) struct KvsInner {
    /// KVS instance parameters.
    pub(crate) parameters: KvsParameters,

    /// KVS instance data.
    pub(crate) data: Arc<Mutex<KvsData>>,
}

static KVS_POOL: LazyLock<Mutex<[Option<KvsInner>; KVS_MAX_INSTANCES]>> =
    LazyLock::new(|| Mutex::new([const { None }; KVS_MAX_INSTANCES]));

impl From<PoisonError<MutexGuard<'_, [Option<KvsInner>; KVS_MAX_INSTANCES]>>> for ErrorCode {
    fn from(_cause: PoisonError<MutexGuard<'_, [Option<KvsInner>; KVS_MAX_INSTANCES]>>) -> Self {
        ErrorCode::MutexLockFailed
    }
}

/// KVS instance parameters, as used by `KvsBuilder`.
/// Parameters set are changed, unset are taken from provided `KvsParameters`.
#[derive(Clone)]
struct KvsBuilderParameters {
    /// Instance ID.
    pub instance_id: InstanceId,

    /// Defaults handling mode.
    pub defaults: Option<KvsDefaults>,

    /// KVS load mode.
    pub kvs_load: Option<KvsLoad>,

    /// Working directory.
    pub working_dir: Option<PathBuf>,

    /// Maximum number of snapshots to store.
    pub snapshot_max_count: Option<usize>,
}

impl KvsBuilderParameters {
    pub fn new(instance_id: InstanceId) -> Self {
        KvsBuilderParameters {
            instance_id,
            defaults: None,
            kvs_load: None,
            working_dir: None,
            snapshot_max_count: None,
        }
    }

    pub fn create_parameters(self, kvs_parameters: &KvsParameters) -> KvsParameters {
        let mut new_parameters = kvs_parameters.clone();

        if let Some(defaults) = self.defaults {
            new_parameters.defaults = defaults;
        }

        if let Some(kvs_load) = self.kvs_load {
            new_parameters.kvs_load = kvs_load;
        }

        if let Some(working_dir) = self.working_dir {
            new_parameters.working_dir = working_dir;
        }

        if let Some(snapshot_max_count) = self.snapshot_max_count {
            new_parameters.snapshot_max_count = snapshot_max_count;
        }

        new_parameters
    }
}

/// Key-value-storage builder.
pub struct GenericKvsBuilder<Backend: KvsBackend, PathResolver: KvsPathResolver = Backend> {
    /// KVS instance parameters.
    parameters: KvsBuilderParameters,

    /// Marker for `Backend`.
    _backend_marker: PhantomData<Backend>,

    /// Marker for `PathResolver`.
    _path_resolver_marker: PhantomData<PathResolver>,
}

impl<Backend: KvsBackend, PathResolver: KvsPathResolver> GenericKvsBuilder<Backend, PathResolver> {
    /// Create a builder to open the key-value-storage
    ///
    /// Only the instance ID must be set. All other settings are using default values until changed
    /// via the builder API.
    ///
    /// # Parameters
    ///   * `instance_id`: Instance ID
    ///
    /// # Return Values
    ///   * KvsBuilder instance
    pub fn new(instance_id: InstanceId) -> Self {
        let parameters = KvsBuilderParameters::new(instance_id);

        Self {
            parameters,
            _backend_marker: PhantomData,
            _path_resolver_marker: PhantomData,
        }
    }

    /// Return maximum number of allowed KVS instances.
    ///
    /// # Return Values
    ///   * Max number of KVS instances
    pub fn max_instances() -> usize {
        KVS_MAX_INSTANCES
    }

    /// Configure defaults handling mode.
    ///
    /// # Parameters
    ///   * `mode`: defaults handling mode (default: [`KvsDefaults::Optional`](KvsDefaults::Optional))
    ///
    /// # Return Values
    ///   * KvsBuilder instance
    pub fn defaults(mut self, mode: KvsDefaults) -> Self {
        self.parameters.defaults = Some(mode);
        self
    }

    /// Configure KVS load mode.
    ///
    /// # Parameters
    ///   * `mode`: KVS load mode (default: [`KvsLoad::Optional`](KvsLoad::Optional))
    ///
    /// # Return Values
    ///   * KvsBuilder instance
    pub fn kvs_load(mut self, mode: KvsLoad) -> Self {
        self.parameters.kvs_load = Some(mode);
        self
    }

    /// Set the key-value-storage permanent storage directory
    ///
    /// # Parameters
    ///   * `dir`: Path to permanent storage
    ///
    /// # Return Values
    ///   * KvsBuilder instance
    pub fn dir<P: Into<String>>(mut self, dir: P) -> Self {
        self.parameters.working_dir = Some(PathBuf::from(dir.into()));
        self
    }

    /// Set the maximum number of snapshots to store.
    ///
    /// # Parameters
    ///   * `snapshot_max_count`: Maximum number of snapshots to store.
    ///
    /// # Return Values
    ///   * KvsBuilder instance
    pub fn snapshot_max_count(mut self, snapshot_max_count: usize) -> Self {
        self.parameters.snapshot_max_count = Some(snapshot_max_count);
        self
    }

    /// Finalize the builder and open the key-value-storage
    ///
    /// Calls `Kvs::open` with the configured settings.
    ///
    /// # Features
    ///   * `FEAT_REQ__KVS__default_values`
    ///   * `FEAT_REQ__KVS__multiple_kvs`
    ///   * `FEAT_REQ__KVS__integrity_check`
    ///
    /// # Return Values
    ///   * Ok: KVS instance
    ///   * `ErrorCode::ValidationFailed`: KVS hash validation failed
    ///   * `ErrorCode::JsonParserError`: JSON parser error
    ///   * `ErrorCode::KvsFileReadError`: KVS file read error
    ///   * `ErrorCode::KvsHashFileReadError`: KVS hash file read error
    ///   * `ErrorCode::UnmappedError`: Generic error
    pub fn build(self) -> Result<GenericKvs<Backend, PathResolver>, ErrorCode> {
        let instance_id = self.parameters.instance_id;
        let instance_id_index: usize = instance_id.into();

        // Check if instance already exists.
        {
            let kvs_pool = KVS_POOL.lock()?;
            let kvs_inner_option = match kvs_pool.get(instance_id_index) {
                Some(kvs_pool_entry) => match kvs_pool_entry {
                    // If instance exists then parameters must match.
                    Some(kvs_inner) => {
                        let kvs_parameters = self
                            .parameters
                            .clone()
                            .create_parameters(&kvs_inner.parameters);
                        if kvs_inner.parameters == kvs_parameters {
                            Ok(Some(kvs_inner))
                        } else {
                            Err(ErrorCode::InstanceParametersMismatch)
                        }
                    }
                    // Instance not found - not an error, will initialize later.
                    None => Ok(None),
                },
                // Instance ID out of range.
                None => Err(ErrorCode::InvalidInstanceId),
            }?;

            // Return existing instance if initialized.
            if let Some(kvs_inner) = kvs_inner_option {
                return Ok(GenericKvs::<Backend, PathResolver>::new(
                    kvs_inner.data.clone(),
                    kvs_inner.parameters.clone(),
                ));
            }
        }

        // Initialize KVS instance with provided parameters.
        // Get parameters object.
        let kvs_parameters = self
            .parameters
            .create_parameters(&KvsParameters::new(instance_id));
        // Load file containing defaults.
        let defaults_path =
            PathResolver::defaults_file_path(&kvs_parameters.working_dir, instance_id);
        let defaults_map = match kvs_parameters.defaults {
            KvsDefaults::Ignored => KvsMap::new(),
            KvsDefaults::Optional => {
                if defaults_path.exists() {
                    Backend::load_kvs(&defaults_path, None)?
                } else {
                    KvsMap::new()
                }
            }
            KvsDefaults::Required => Backend::load_kvs(&defaults_path, None)?,
        };

        // Load KVS and hash files.
        let snapshot_id = SnapshotId(0);
        let kvs_path =
            PathResolver::kvs_file_path(&kvs_parameters.working_dir, instance_id, snapshot_id);
        let hash_path =
            PathResolver::hash_file_path(&kvs_parameters.working_dir, instance_id, snapshot_id);
        let kvs_map = match kvs_parameters.kvs_load {
            KvsLoad::Ignored => KvsMap::new(),
            KvsLoad::Optional => {
                if kvs_path.exists() && hash_path.exists() {
                    Backend::load_kvs(&kvs_path, Some(&hash_path))?
                } else {
                    KvsMap::new()
                }
            }
            KvsLoad::Required => Backend::load_kvs(&kvs_path, Some(&hash_path))?,
        };

        // Shared object containing data.
        let data = Arc::new(Mutex::new(KvsData {
            kvs_map,
            defaults_map,
        }));

        // Initialize entry in pool and return new KVS instance.
        {
            let mut kvs_pool = KVS_POOL.lock()?;
            let kvs_pool_entry = match kvs_pool.get_mut(instance_id_index) {
                Some(entry) => entry,
                None => return Err(ErrorCode::InvalidInstanceId),
            };

            let _ = kvs_pool_entry.insert(KvsInner {
                parameters: kvs_parameters.clone(),
                data: data.clone(),
            });
        }

        Ok(GenericKvs::new(data, kvs_parameters))
    }
}

#[cfg(test)]
mod kvs_builder_tests {
    use crate::error_code::ErrorCode;
    use crate::json_backend::JsonBackend;
    use crate::kvs_api::{InstanceId, KvsApi, KvsDefaults, KvsLoad, SnapshotId};
    use crate::kvs_backend::{KvsBackend, KvsPathResolver};
    use crate::kvs_builder::{GenericKvsBuilder, KVS_MAX_INSTANCES, KVS_POOL};
    use crate::kvs_value::{KvsMap, KvsValue};
    use std::ops::DerefMut;
    use std::path::{Path, PathBuf};
    use std::sync::{LazyLock, Mutex, MutexGuard};
    use tempfile::tempdir;

    /// Serial test execution mutex.
    static SERIAL_TEST: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    /// Execute test serially with KVS pool uninitialized.
    fn lock_and_reset<'a>() -> MutexGuard<'a, ()> {
        // Tests in this group must be executed serially.
        let serial_lock: MutexGuard<'a, ()> = SERIAL_TEST.lock().unwrap();

        // Reset `KVS_POOL` state to uninitialized.
        // This is to mitigate `InstanceParametersMismatch` errors between tests.
        let mut pool = KVS_POOL.lock().unwrap();
        *pool.deref_mut() = [const { None }; KVS_MAX_INSTANCES];

        serial_lock
    }

    /// KVS backend type used for tests.
    /// Tests reuse JSON backend to ensure valid load/save behavior.
    type TestBackend = JsonBackend;
    type TestKvsBuilder = GenericKvsBuilder<TestBackend>;

    #[test]
    fn test_new_ok() {
        let _lock = lock_and_reset();

        // Check only if panic happens.
        let instance_id = InstanceId(0);
        let _ = TestKvsBuilder::new(instance_id);
    }

    #[test]
    fn test_max_instances() {
        assert_eq!(TestKvsBuilder::max_instances(), KVS_MAX_INSTANCES);
    }

    #[test]
    fn test_parameters_instance_id() {
        let _lock = lock_and_reset();

        let instance_id = InstanceId(1);
        let builder = TestKvsBuilder::new(instance_id);
        let kvs = builder.build().unwrap();

        assert_eq!(kvs.parameters().instance_id, instance_id);
        // Check default values.
        assert_eq!(kvs.parameters().defaults, KvsDefaults::Optional);
        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Optional);
        assert_eq!(kvs.parameters().working_dir, PathBuf::new());
        assert_eq!(kvs.snapshot_max_count(), 3);
    }

    #[test]
    fn test_parameters_defaults() {
        let _lock = lock_and_reset();

        let instance_id = InstanceId(1);
        let builder = TestKvsBuilder::new(instance_id).defaults(KvsDefaults::Ignored);
        let kvs = builder.build().unwrap();
        assert_eq!(kvs.parameters().instance_id, instance_id);
        assert_eq!(kvs.parameters().defaults, KvsDefaults::Ignored);
        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Optional);
        assert_eq!(kvs.parameters().working_dir, PathBuf::new());
        assert_eq!(kvs.snapshot_max_count(), 3);
    }

    #[test]
    fn test_parameters_kvs_load() {
        let _lock = lock_and_reset();

        let instance_id = InstanceId(1);
        let builder = TestKvsBuilder::new(instance_id).kvs_load(KvsLoad::Ignored);
        let kvs = builder.build().unwrap();
        assert_eq!(kvs.parameters().instance_id, instance_id);
        assert_eq!(kvs.parameters().defaults, KvsDefaults::Optional);
        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Ignored);
        assert_eq!(kvs.parameters().working_dir, PathBuf::new());
        assert_eq!(kvs.snapshot_max_count(), 3);
    }

    #[test]
    fn test_parameters_dir() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(5);
        let builder = TestKvsBuilder::new(instance_id).dir(dir_string.clone());
        let kvs = builder.build().unwrap();
        assert_eq!(kvs.parameters().instance_id, instance_id);
        assert_eq!(kvs.parameters().defaults, KvsDefaults::Optional);
        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Optional);
        assert_eq!(kvs.parameters().working_dir, dir.path());
        assert_eq!(kvs.snapshot_max_count(), 3);
    }

    #[test]
    fn test_parameters_snapshot_max_count() {
        let _lock = lock_and_reset();

        let instance_id = InstanceId(1);
        let builder = TestKvsBuilder::new(instance_id).snapshot_max_count(1234);
        let kvs = builder.build().unwrap();
        assert_eq!(kvs.parameters().instance_id, instance_id);
        assert_eq!(kvs.parameters().defaults, KvsDefaults::Optional);
        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Optional);
        assert_eq!(kvs.parameters().working_dir, PathBuf::new());
        assert_eq!(kvs.snapshot_max_count(), 1234);
    }

    #[test]
    fn test_parameters_chained() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(1);
        let builder = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Ignored)
            .kvs_load(KvsLoad::Ignored)
            .dir(dir_string)
            .snapshot_max_count(1234);
        let kvs = builder.build().unwrap();
        assert_eq!(kvs.parameters().instance_id, instance_id);
        assert_eq!(kvs.parameters().defaults, KvsDefaults::Ignored);
        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Ignored);
        assert_eq!(kvs.parameters().working_dir, dir.path());
        assert_eq!(kvs.snapshot_max_count(), 1234);
    }

    #[test]
    fn test_build_ok() {
        let _lock = lock_and_reset();

        let instance_id = InstanceId(1);
        let builder = TestKvsBuilder::new(instance_id);
        let _ = builder.build().unwrap();
    }

    #[test]
    fn test_build_instance_exists_same_params() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        // Create two instances with same parameters.
        let instance_id = InstanceId(1);
        let builder1 = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Ignored)
            .kvs_load(KvsLoad::Ignored)
            .dir(dir_string.clone());
        let _ = builder1.build().unwrap();

        let builder2 = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Ignored)
            .kvs_load(KvsLoad::Ignored)
            .dir(dir_string);
        let kvs = builder2.build().unwrap();

        // Assert params as expected.
        assert_eq!(kvs.parameters().instance_id, instance_id);
        assert_eq!(kvs.parameters().defaults, KvsDefaults::Ignored);
        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Ignored);
        assert_eq!(kvs.parameters().working_dir, dir.path());
    }

    #[test]
    fn test_build_instance_exists_different_params() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        // Create two instances with different parameters.
        let instance_id = InstanceId(1);
        let builder1 = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Ignored)
            .kvs_load(KvsLoad::Optional)
            .dir(dir_string.clone());
        let _ = builder1.build().unwrap();

        let builder2 = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Optional)
            .kvs_load(KvsLoad::Ignored)
            .dir(dir_string);
        let result = builder2.build();

        assert!(result.is_err_and(|e| e == ErrorCode::InstanceParametersMismatch));
    }

    #[test]
    fn test_build_instance_exists_params_not_set() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        // Create two instances, first with parameters, second without.
        let instance_id = InstanceId(1);
        let builder1 = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Ignored)
            .kvs_load(KvsLoad::Ignored)
            .dir(dir_string.clone());
        let _ = builder1.build().unwrap();

        let builder2 = TestKvsBuilder::new(instance_id);
        let kvs = builder2.build().unwrap();

        // Assert params as expected.
        assert_eq!(kvs.parameters().instance_id, instance_id);
        assert_eq!(kvs.parameters().defaults, KvsDefaults::Ignored);
        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Ignored);
        assert_eq!(kvs.parameters().working_dir, dir.path());
    }

    #[test]
    fn test_build_instance_exists_single_matching_param_set() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        // Create two instances, first with parameters, second only with `defaults`.
        let instance_id = InstanceId(1);
        let builder1 = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Ignored)
            .kvs_load(KvsLoad::Ignored)
            .dir(dir_string.clone());
        let _ = builder1.build().unwrap();

        let builder2 = TestKvsBuilder::new(instance_id).defaults(KvsDefaults::Ignored);
        let kvs = builder2.build().unwrap();

        // Assert params as expected.
        assert_eq!(kvs.parameters().instance_id, instance_id);
        assert_eq!(kvs.parameters().defaults, KvsDefaults::Ignored);
        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Ignored);
        assert_eq!(kvs.parameters().working_dir, dir.path());
    }

    #[test]
    fn test_build_instance_exists_single_mismatched_param_set() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        // Create two instances, first with parameters, second only with `defaults`.
        let instance_id = InstanceId(1);
        let builder1 = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Ignored)
            .kvs_load(KvsLoad::Ignored)
            .dir(dir_string.clone());
        let _ = builder1.build().unwrap();

        let builder2 = TestKvsBuilder::new(instance_id).defaults(KvsDefaults::Optional);
        let result = builder2.build();

        assert!(result.is_err_and(|e| e == ErrorCode::InstanceParametersMismatch));
    }

    #[test]
    fn test_build_instance_id_out_of_range() {
        let _lock = lock_and_reset();

        let instance_id = InstanceId(123);
        let result = TestKvsBuilder::new(instance_id).build();
        assert!(result.is_err_and(|e| e == ErrorCode::InvalidInstanceId));
    }

    /// Generate and store file containing example default values.
    fn create_defaults_file(
        working_dir: &Path,
        instance_id: InstanceId,
    ) -> Result<PathBuf, ErrorCode> {
        let defaults_file_path = TestBackend::defaults_file_path(working_dir, instance_id);
        let kvs_map = KvsMap::from([
            ("number1".to_string(), KvsValue::F64(123.0)),
            ("bool1".to_string(), KvsValue::Boolean(true)),
            ("string1".to_string(), KvsValue::String("Hello".to_string())),
        ]);
        TestBackend::save_kvs(&kvs_map, &defaults_file_path, None)?;

        Ok(defaults_file_path)
    }

    /// Generate and store files containing example KVS and hash data.
    fn create_kvs_files(
        working_dir: &Path,
        instance_id: InstanceId,
        snapshot_id: SnapshotId,
    ) -> Result<(PathBuf, PathBuf), ErrorCode> {
        let kvs_file_path = TestBackend::kvs_file_path(working_dir, instance_id, snapshot_id);
        let hash_file_path = TestBackend::hash_file_path(working_dir, instance_id, snapshot_id);
        let kvs_map = KvsMap::from([
            ("number1".to_string(), KvsValue::F64(321.0)),
            ("bool1".to_string(), KvsValue::Boolean(false)),
            ("string1".to_string(), KvsValue::String("Hi".to_string())),
        ]);
        TestBackend::save_kvs(&kvs_map, &kvs_file_path, Some(&hash_file_path))?;

        Ok((kvs_file_path, hash_file_path))
    }

    #[test]
    fn test_build_defaults_ignored() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_defaults_file(dir.path(), instance_id).unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Ignored)
            .dir(dir_string);
        let kvs = builder.build().unwrap();

        assert_eq!(kvs.parameters().defaults, KvsDefaults::Ignored);
        let kvs_pool = KVS_POOL.lock().unwrap();
        let kvs_pool_entry = kvs_pool.get(2).unwrap();
        let kvs_data = kvs_pool_entry.as_ref().unwrap();
        assert_eq!(kvs_data.data.lock().unwrap().defaults_map, KvsMap::new());
    }

    #[test]
    fn test_build_defaults_optional_not_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        let builder = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Optional)
            .dir(dir_string);
        let kvs = builder.build().unwrap();

        assert_eq!(kvs.parameters().defaults, KvsDefaults::Optional);
        let kvs_pool = KVS_POOL.lock().unwrap();
        let kvs_pool_entry = kvs_pool.get(2).unwrap();
        let kvs_data = kvs_pool_entry.as_ref().unwrap();
        assert_eq!(kvs_data.data.lock().unwrap().defaults_map, KvsMap::new());
    }

    #[test]
    fn test_build_defaults_optional_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_defaults_file(dir.path(), instance_id).unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Optional)
            .dir(dir_string);
        let kvs = builder.build().unwrap();

        assert_eq!(kvs.parameters().defaults, KvsDefaults::Optional);
        let kvs_pool = KVS_POOL.lock().unwrap();
        let kvs_pool_entry = kvs_pool.get(2).unwrap();
        let kvs_data = kvs_pool_entry.as_ref().unwrap();
        assert_eq!(kvs_data.data.lock().unwrap().defaults_map.len(), 3);
    }

    #[test]
    fn test_build_defaults_required_not_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        let builder = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Required)
            .dir(dir_string);
        let result = builder.build();

        assert!(result.is_err_and(|e| e == ErrorCode::FileNotFound));
    }

    #[test]
    fn test_build_defaults_required_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_defaults_file(dir.path(), instance_id).unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .defaults(KvsDefaults::Required)
            .dir(dir_string);
        let kvs = builder.build().unwrap();

        assert_eq!(kvs.parameters().defaults, KvsDefaults::Required);
        let kvs_pool = KVS_POOL.lock().unwrap();
        let kvs_pool_entry = kvs_pool.get(2).unwrap();
        let kvs_data = kvs_pool_entry.as_ref().unwrap();
        assert_eq!(kvs_data.data.lock().unwrap().defaults_map.len(), 3);
    }

    #[test]
    fn test_build_kvs_load_ignored() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_kvs_files(dir.path(), instance_id, SnapshotId(0)).unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .kvs_load(KvsLoad::Ignored)
            .dir(dir_string);
        let kvs = builder.build().unwrap();

        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Ignored);
        let kvs_pool = KVS_POOL.lock().unwrap();
        let kvs_pool_entry = kvs_pool.get(2).unwrap();
        let kvs_data = kvs_pool_entry.as_ref().unwrap();
        assert_eq!(kvs_data.data.lock().unwrap().kvs_map, KvsMap::new());
    }

    #[test]
    fn test_build_kvs_load_optional_not_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        let builder = TestKvsBuilder::new(instance_id)
            .kvs_load(KvsLoad::Optional)
            .dir(dir_string);
        let kvs = builder.build().unwrap();

        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Optional);
        let kvs_pool = KVS_POOL.lock().unwrap();
        let kvs_pool_entry = kvs_pool.get(2).unwrap();
        let kvs_data = kvs_pool_entry.as_ref().unwrap();
        assert_eq!(kvs_data.data.lock().unwrap().kvs_map, KvsMap::new());
    }

    #[test]
    #[ignore = "Not handled properly yet"]
    fn test_build_kvs_load_optional_kvs_provided_hash_not_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_kvs_files(dir.path(), instance_id, SnapshotId(0)).unwrap();
        std::fs::remove_file(TestBackend::hash_file_path(
            dir.path(),
            instance_id,
            SnapshotId(0),
        ))
        .unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .kvs_load(KvsLoad::Optional)
            .dir(dir_string);
        let result = builder.build();

        assert!(result.is_err_and(|e| e == ErrorCode::KvsHashFileReadError));
    }

    #[test]
    #[ignore = "Not handled properly yet"]
    fn test_build_kvs_load_optional_kvs_not_provided_hash_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_kvs_files(dir.path(), instance_id, SnapshotId(0)).unwrap();
        std::fs::remove_file(TestBackend::kvs_file_path(
            dir.path(),
            instance_id,
            SnapshotId(0),
        ))
        .unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .kvs_load(KvsLoad::Optional)
            .dir(dir_string);
        let result = builder.build();

        assert!(result.is_err_and(|e| e == ErrorCode::FileNotFound));
    }

    #[test]
    fn test_build_kvs_load_optional_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_kvs_files(dir.path(), instance_id, SnapshotId(0)).unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .kvs_load(KvsLoad::Optional)
            .dir(dir_string);
        let kvs = builder.build().unwrap();

        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Optional);
        let kvs_pool = KVS_POOL.lock().unwrap();
        let kvs_pool_entry = kvs_pool.get(2).unwrap();
        let kvs_data = kvs_pool_entry.as_ref().unwrap();
        assert_eq!(kvs_data.data.lock().unwrap().kvs_map.len(), 3);
    }

    #[test]
    fn test_build_kvs_load_required_not_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        let builder = TestKvsBuilder::new(instance_id)
            .kvs_load(KvsLoad::Required)
            .dir(dir_string);
        let result = builder.build();

        assert!(result.is_err_and(|e| e == ErrorCode::FileNotFound));
    }

    #[test]
    #[ignore = "Not handled properly yet"]
    fn test_build_kvs_load_required_kvs_provided_hash_not_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_kvs_files(dir.path(), instance_id, SnapshotId(0)).unwrap();
        std::fs::remove_file(TestBackend::hash_file_path(
            dir.path(),
            instance_id,
            SnapshotId(0),
        ))
        .unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .kvs_load(KvsLoad::Required)
            .dir(dir_string);
        let result = builder.build();

        assert!(result.is_err_and(|e| e == ErrorCode::KvsHashFileReadError));
    }

    #[test]
    #[ignore = "Not handled properly yet"]
    fn test_build_kvs_load_required_kvs_not_provided_hash_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_kvs_files(dir.path(), instance_id, SnapshotId(0)).unwrap();
        std::fs::remove_file(TestBackend::kvs_file_path(
            dir.path(),
            instance_id,
            SnapshotId(0),
        ))
        .unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .kvs_load(KvsLoad::Required)
            .dir(dir_string);
        let result = builder.build();

        assert!(result.is_err_and(|e| e == ErrorCode::FileNotFound));
    }

    #[test]
    fn test_build_kvs_load_required_provided() {
        let _lock = lock_and_reset();

        let dir = tempdir().unwrap();
        let dir_string = dir.path().to_string_lossy().to_string();

        let instance_id = InstanceId(2);
        create_kvs_files(dir.path(), instance_id, SnapshotId(0)).unwrap();
        let builder = TestKvsBuilder::new(instance_id)
            .kvs_load(KvsLoad::Required)
            .dir(dir_string);
        let kvs = builder.build().unwrap();

        assert_eq!(kvs.parameters().kvs_load, KvsLoad::Required);
        let kvs_pool = KVS_POOL.lock().unwrap();
        let kvs_pool_entry = kvs_pool.get(2).unwrap();
        let kvs_data = kvs_pool_entry.as_ref().unwrap();
        assert_eq!(kvs_data.data.lock().unwrap().kvs_map.len(), 3);
    }
}
