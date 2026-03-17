//! Example for default values.
//! - Creating KVS instance using `KvsBuilder` with `defaults` modes.
//! - Default-specific APIs: `get_default_value`, `is_value_default`.
//! - Key-value operations behavior on defaults available.

use rust_kvs::prelude::*;
use std::path::PathBuf;
use tempfile::tempdir;
use tinyjson::JsonValue;

/// Utility function for creating file containing default values.
fn create_defaults_file(dir_path: PathBuf, instance_id: InstanceId) -> Result<(), ErrorCode> {
    // Path to expected defaults file.
    // E.g., `/tmp/xyz/kvs_0_default.json`.
    let defaults_file_path = dir_path.join(format!("kvs_{instance_id}_default.json"));

    // Create defaults.
    // `KvsValue` is converted to `JsonValue` to ensure types are tagged.
    let kvs_value = KvsValue::from(KvsMap::from([
        ("k1".to_string(), KvsValue::I32(-123i32)),
        ("k2".to_string(), KvsValue::U64(654321u64)),
        ("k3".to_string(), KvsValue::String("example".to_string())),
    ]));
    let json_value = JsonValue::from(kvs_value);

    // Stringify and save to file.
    let json_str = json_value.stringify()?;
    std::fs::write(&defaults_file_path, &json_str)?;

    Ok(())
}

fn main() -> Result<(), ErrorCode> {
    // Temporary directory.
    let dir = tempdir()?;
    let dir_string = dir.path().to_string_lossy().to_string();

    // Instance ID for KVS object instances.
    let instance_id = InstanceId(0);

    // Create and save defaults file.
    create_defaults_file(dir.path().to_path_buf(), instance_id)?;

    // Build KVS instance for given instance ID and temporary directory.
    // `defaults` is set to `KvsDefaults::Required` - defaults are required.
    let builder = KvsBuilder::new(instance_id)
        .dir(dir_string)
        .defaults(KvsDefaults::Required);
    let kvs = builder.build()?;

    // Set explicit value to `k1`.
    kvs.set_value("k1", -4321i32)?;

    {
        println!("-> `is_value_default` usage");

        // `k1` contains explicit value - returns `false`.
        let k1_key = "k1";
        let is_k1_default = kvs.is_value_default(k1_key)?;
        if !is_k1_default {
            println!("{k1_key} is not default");
        }

        // `k2` contains default value - returns `true`.
        let k2_key = "k2";
        let is_k2_default = kvs.is_value_default(k2_key)?;
        if is_k2_default {
            println!("{k2_key} is default");
        }

        println!();
    }

    {
        println!("-> `get_default_value` usage");

        let k1_key = "k1";
        let k1_value = kvs.get_default_value(k1_key)?;
        println!("{k1_key:?} = {k1_value:?}");

        let k2_key = "k2";
        let k2_value = kvs.get_default_value(k2_key)?;
        println!("{k2_key:?} = {k2_value:?}");

        println!();
    }

    {
        println!("-> Other APIs usage");

        let all_keys = kvs.get_all_keys()?;
        println!("All KVS keys: {all_keys:?}");

        let k3_key = "k3";
        let k3_value = kvs.get_value(k3_key)?;
        println!("{k3_key:?} = {k3_value:?}");

        if !kvs.key_exists(k3_key)? {
            println!("{k3_key} key not exists");
        }

        println!();
    }

    {
        println!("-> `reset_key` usage");

        let k1_key = "k1";
        kvs.reset_key(k1_key)?;
        if !kvs.key_exists(k1_key)? {
            println!("{k1_key} key not found");
        }
        if kvs.is_value_default(k1_key)? {
            println!("{k1_key} default available");
        }

        println!();
    }

    Ok(())
}
