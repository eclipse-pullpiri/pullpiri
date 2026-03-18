//! Example for basic operations.
//! - Creating KVS instance using `KvsBuilder` with `kvs_load` modes.
//! - Basic key-value operations: `get_value`, `get_value_as`, `set_value`, `get_all_keys`.
//! - Other key-value operations: `reset`, `key_exists`, `remove_key`.

use rust_kvs::prelude::*;
use std::collections::HashMap;
use tempfile::tempdir;

fn main() -> Result<(), ErrorCode> {
    // Temporary directory.
    let dir = tempdir()?;
    let dir_string = dir.path().to_string_lossy().to_string();

    // Instance ID for KVS object instances.
    let instance_id = InstanceId(0);

    {
        // Build KVS instance for given instance ID and temporary directory.
        // `kvs_load` is explicitly set to `KvsLoad::Optional`, but this is the default value.
        // KVS files are not required.
        let builder = KvsBuilder::new(instance_id)
            .dir(dir_string.clone())
            .kvs_load(KvsLoad::Optional);
        let kvs = builder.build()?;

        println!("-> `set_value` usage");
        kvs.set_value("number", 123.0)?;
        kvs.set_value("bool", true)?;
        kvs.set_value("string", "First")?;
        kvs.set_value("null", ())?;
        kvs.set_value(
            "array",
            vec![
                KvsValue::from(456.0),
                false.into(),
                "Second".to_string().into(),
            ],
        )?;
        kvs.set_value(
            "object",
            HashMap::from([
                (String::from("sub-number"), KvsValue::from(789.0)),
                ("sub-bool".into(), true.into()),
                ("sub-string".into(), "Third".to_string().into()),
                ("sub-null".into(), ().into()),
                (
                    "sub-array".into(),
                    KvsValue::from(vec![
                        KvsValue::from(1246.0),
                        false.into(),
                        "Fourth".to_string().into(),
                    ]),
                ),
            ]),
        )?;

        println!();

        // Flush KVS.
        kvs.flush()?;
    }

    {
        // Build KVS instance for given instance ID and temporary directory.
        // `kvs_load` is set to `KvsLoad::Required` - KVS files must already exist from previous KVS instance.
        let builder = KvsBuilder::new(instance_id)
            .dir(dir_string)
            .kvs_load(KvsLoad::Required);
        let kvs = builder.build()?;

        // `get_value` usage - print all existing keys with their values.
        // `KvsValue` is returned, underlying type can be determined at runtime.
        {
            println!("-> `get_value` usage");

            for key in kvs.get_all_keys()? {
                let value = kvs.get_value(&key)?;
                let value_type = match value {
                    KvsValue::I32(_) => "I32",
                    KvsValue::U32(_) => "U32",
                    KvsValue::I64(_) => "I64",
                    KvsValue::U64(_) => "U64",
                    KvsValue::F64(_) => "F64",
                    KvsValue::Boolean(_) => "Boolean",
                    KvsValue::String(_) => "String",
                    KvsValue::Null => "Null",
                    KvsValue::Array(_) => "Array",
                    KvsValue::Object(_) => "Object",
                };
                println!("{key:?} = {value:?} ({value_type:?})");
            }

            println!();
        }

        // `get_value_as` usage - print bool and string key-value pairs.
        // Underlying type is defined at compile time, but checked at runtime.
        // Type mismatch will cause `ConversionFailed`.
        {
            println!("-> `get_value_as` usage");

            let bool_key = "bool";
            let bool_value = kvs.get_value_as::<bool>(bool_key)?;
            println!("{bool_key:?} = {bool_value:?}");
            let string_key = "string";
            let string_value = kvs.get_value_as::<String>(string_key)?;
            println!("{string_key:?} = {string_value:?}");

            println!();
        }

        // Examples of other APIs.
        {
            println!("-> `key_exists` usage");

            let existing_key = "string";
            if kvs.key_exists(existing_key)? {
                println!("Key exists: {existing_key}");
            }

            let invalid_key = "invalid_key";
            if !kvs.key_exists(invalid_key)? {
                println!("Key not exists: {invalid_key}");
            }

            println!();
        }

        {
            println!("-> `remove_key` usage");

            let null_key = "null";
            kvs.remove_key(null_key)?;
            if !kvs.key_exists(null_key)? {
                println!("Key-value removed: {null_key}");
            }

            println!();
        }

        {
            println!("-> `reset` usage");

            kvs.reset()?;
            if kvs.get_all_keys()?.is_empty() {
                println!("KVS reset successful");
            }

            println!();
        }
    }

    Ok(())
}
