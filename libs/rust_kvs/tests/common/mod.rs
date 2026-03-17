//! Common test utilities.

// Common test utilities are placed in `common/mod.rs` on purpose.
// This is to ensure file is not improperly detected as empty test file.

use rust_kvs::kvs_value::KvsValue;
use std::iter::zip;

/// Compare `KvsValue` objects.
///
/// # Parameters
///   * `left`: left value
///   * `right`: right value
///
/// # Return Value
///   * `true` if provided values are same.
pub fn compare_kvs_values(left: &KvsValue, right: &KvsValue) -> bool {
    match (left, right) {
        (KvsValue::I32(l), KvsValue::I32(r)) => l == r,
        (KvsValue::U32(l), KvsValue::U32(r)) => l == r,
        (KvsValue::I64(l), KvsValue::I64(r)) => l == r,
        (KvsValue::U64(l), KvsValue::U64(r)) => l == r,
        (KvsValue::F64(l), KvsValue::F64(r)) => l == r,
        (KvsValue::Boolean(l), KvsValue::Boolean(r)) => l == r,
        (KvsValue::String(l), KvsValue::String(r)) => l == r,
        (KvsValue::Null, KvsValue::Null) => true,
        (KvsValue::Array(l), KvsValue::Array(r)) => {
            // Check size.
            if l.len() != r.len() {
                return false;
            }

            // Iterate over elements.
            for (lv, rv) in zip(l, r) {
                if !compare_kvs_values(lv, rv) {
                    return false;
                }
            }

            true
        }
        (KvsValue::Object(l), KvsValue::Object(r)) => {
            // Check size.
            if l.len() != r.len() {
                return false;
            }

            // Check keys.
            if l.keys().ne(r.keys()) {
                return false;
            }

            // Iterate over elements.
            let keys = l.keys();
            for k in keys {
                if !compare_kvs_values(&l[k], &r[k]) {
                    return false;
                }
            }

            true
        }
        // Return false for all other type combinations (mismatched or unsupported types)
        (_, _) => false,
    }
}
