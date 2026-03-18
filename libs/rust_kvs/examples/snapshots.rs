//! Example for snapshots handling.
//! - Snapshot count and max count.
//! - Snapshot restore.

use rust_kvs::prelude::*;
use tempfile::tempdir;

fn main() -> Result<(), ErrorCode> {
    // Temporary directory.
    let dir = tempdir()?;
    let dir_string = dir.path().to_string_lossy().to_string();

    // Instance ID for KVS object instances.
    let instance_id = InstanceId(0);

    {
        println!("-> `snapshot_count` and `snapshot_max_count` usage");

        // Build KVS instance for given instance ID and temporary directory.
        let builder = KvsBuilder::new(instance_id).dir(dir_string.clone());
        let kvs = builder.build()?;

        let max_count = kvs.snapshot_max_count() as u32;
        println!("Max snapshot count: {max_count:?}");

        // Snapshots are created and rotated on flush.
        let counter_key = "counter";
        for index in 0..max_count {
            kvs.set_value(counter_key, index)?;
            kvs.flush()?;
            println!("Snapshot count: {:?}", kvs.snapshot_count());
        }

        println!();
    }

    {
        println!("-> `snapshot_restore` usage");

        // Build KVS instance for given instance ID and temporary directory.
        let builder = KvsBuilder::new(instance_id).dir(dir_string.clone());
        let kvs = builder.build()?;

        let max_count = kvs.snapshot_max_count() as u32;
        let counter_key = "counter";
        for index in 0..max_count {
            kvs.set_value(counter_key, index)?;
            kvs.flush()?;
        }

        // Print current counter value, then restore oldest snapshot.
        println!("{counter_key} = {:?}", kvs.get_value(counter_key)?);
        kvs.snapshot_restore(SnapshotId(2))?;
        println!("{counter_key} = {:?}", kvs.get_value(counter_key)?);

        println!();
    }

    Ok(())
}
