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

// TryFrom<&KvsValue> for all supported types
use std::convert::TryFrom;

/// Key-value storage map type
pub type KvsMap = std::collections::HashMap<String, KvsValue>;

/// Key-value-storage value
#[derive(Clone, Debug, PartialEq)]
pub enum KvsValue {
    /// 32-bit signed integer
    I32(i32),

    /// 32-bit unsigned integer
    U32(u32),

    /// 64-bit signed integer
    I64(i64),

    /// 64-bit unsigned integer
    U64(u64),

    /// 64-bit float
    F64(f64),

    /// Boolean
    Boolean(bool),

    /// String
    String(String),

    /// Null
    Null,

    /// Array
    Array(Vec<KvsValue>),

    /// Object
    Object(KvsMap),
}

// Macro to implement From<T> for KvsValue for each supported type/variant.
// This allows concise and consistent conversion from basic Rust types to KvsValue.
macro_rules! impl_from_t_for_kvs_value {
    ($from:ty, $variant:ident) => {
        impl From<$from> for KvsValue {
            fn from(val: $from) -> Self {
                KvsValue::$variant(val)
            }
        }
    };
}

impl_from_t_for_kvs_value!(i32, I32);
impl_from_t_for_kvs_value!(u32, U32);
impl_from_t_for_kvs_value!(i64, I64);
impl_from_t_for_kvs_value!(u64, U64);
impl_from_t_for_kvs_value!(f64, F64);
impl_from_t_for_kvs_value!(bool, Boolean);
impl_from_t_for_kvs_value!(String, String);
impl_from_t_for_kvs_value!(Vec<KvsValue>, Array);
impl_from_t_for_kvs_value!(KvsMap, Object);

// Convert &str to KvsValue::String
impl From<&str> for KvsValue {
    fn from(val: &str) -> Self {
        KvsValue::String(val.to_string())
    }
}
// Convert unit type () to KvsValue::Null
impl From<()> for KvsValue {
    fn from(_: ()) -> Self {
        KvsValue::Null
    }
}

// Macro to implement TryFrom<&KvsValue> for T for each supported type/variant.
macro_rules! impl_tryfrom_kvs_value_to_t {
    ($to:ty, $variant:ident) => {
        impl std::convert::TryFrom<&KvsValue> for $to {
            type Error = String;
            fn try_from(value: &KvsValue) -> Result<Self, Self::Error> {
                if let KvsValue::$variant(ref n) = value {
                    Ok(n.clone())
                } else {
                    Err(format!("KvsValue is not a {}", stringify!($to)))
                }
            }
        }
    };
}

impl_tryfrom_kvs_value_to_t!(i32, I32);
impl_tryfrom_kvs_value_to_t!(u32, U32);
impl_tryfrom_kvs_value_to_t!(i64, I64);
impl_tryfrom_kvs_value_to_t!(u64, U64);
impl_tryfrom_kvs_value_to_t!(f64, F64);
impl_tryfrom_kvs_value_to_t!(bool, Boolean);
impl_tryfrom_kvs_value_to_t!(String, String);
impl_tryfrom_kvs_value_to_t!(Vec<KvsValue>, Array);
impl_tryfrom_kvs_value_to_t!(std::collections::HashMap<String, KvsValue>, Object);

impl TryFrom<&KvsValue> for () {
    type Error = &'static str;
    fn try_from(value: &KvsValue) -> Result<Self, Self::Error> {
        match value {
            KvsValue::Null => Ok(()),
            _ => Err("KvsValue is not a Null (unit type)"),
        }
    }
}

// Trait for extracting inner values from KvsValue
pub trait KvsValueGet {
    fn get_inner_value(val: &KvsValue) -> Option<&Self>;
}

impl KvsValue {
    pub fn get<T: KvsValueGet>(&self) -> Option<&T> {
        T::get_inner_value(self)
    }
}

macro_rules! impl_kvs_get_inner_value {
    ($to:ty, $variant:ident) => {
        impl KvsValueGet for $to {
            fn get_inner_value(v: &KvsValue) -> Option<&$to> {
                match v {
                    KvsValue::$variant(n) => Some(n),
                    _ => None,
                }
            }
        }
    };
}
impl_kvs_get_inner_value!(f64, F64);
impl_kvs_get_inner_value!(i32, I32);
impl_kvs_get_inner_value!(u32, U32);
impl_kvs_get_inner_value!(i64, I64);
impl_kvs_get_inner_value!(u64, U64);
impl_kvs_get_inner_value!(bool, Boolean);
impl_kvs_get_inner_value!(String, String);
impl_kvs_get_inner_value!(Vec<KvsValue>, Array);
impl_kvs_get_inner_value!(std::collections::HashMap<String, KvsValue>, Object);

impl KvsValueGet for () {
    fn get_inner_value(v: &KvsValue) -> Option<&()> {
        match v {
            KvsValue::Null => Some(&()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod kvs_value_tests {
    use crate::kvs_value::{KvsMap, KvsValue};

    #[test]
    fn test_i32_from_ok() {
        let v = KvsValue::from(-42i32);
        assert!(matches!(v, KvsValue::I32(x) if x == -42));
    }

    #[test]
    fn test_i32_tryfrom_ok() {
        let v = KvsValue::from(123i32);
        assert_eq!(i32::try_from(&v).unwrap(), 123);
    }

    #[test]
    fn test_i32_tryfrom_invalid_type() {
        let v = KvsValue::from("abc");
        let err = i32::try_from(&v).unwrap_err();
        assert_eq!(err, "KvsValue is not a i32");
    }

    #[test]
    fn test_i32_get_ok() {
        let v = KvsValue::from(123i32);
        assert_eq!(v.get::<i32>().unwrap().clone(), 123);
    }

    #[test]
    fn test_i32_get_invalid_type() {
        let v = KvsValue::from("abc");
        assert!(v.get::<i32>().is_none());
    }

    #[test]
    fn test_u32_from_ok() {
        let v = KvsValue::from(42u32);
        assert!(matches!(v, KvsValue::U32(x) if x == 42));
    }

    #[test]
    fn test_u32_tryfrom_ok() {
        let v = KvsValue::from(456u32);
        assert_eq!(u32::try_from(&v).unwrap(), 456);
    }

    #[test]
    fn test_u32_tryfrom_invalid_type() {
        let v = KvsValue::from(123i32);
        let err = u32::try_from(&v).unwrap_err();
        assert_eq!(err, "KvsValue is not a u32");
    }

    #[test]
    fn test_u32_get_ok() {
        let v = KvsValue::from(456u32);
        assert_eq!(v.get::<u32>().unwrap().clone(), 456);
    }

    #[test]
    fn test_u32_get_invalid_type() {
        let v = KvsValue::from(123i32);
        assert!(v.get::<u32>().is_none());
    }

    #[test]
    fn test_i64_from_ok() {
        let v = KvsValue::from(-123456789i64);
        assert!(matches!(v, KvsValue::I64(x) if x == -123456789));
    }

    #[test]
    fn test_i64_tryfrom_ok() {
        let v = KvsValue::from(789i64);
        assert_eq!(i64::try_from(&v).unwrap(), 789);
    }

    #[test]
    fn test_i64_tryfrom_invalid_type() {
        let v = KvsValue::from("abc");
        let err = i64::try_from(&v).unwrap_err();
        assert_eq!(err, "KvsValue is not a i64");
    }

    #[test]
    fn test_i64_get_ok() {
        let v = KvsValue::from(789i64);
        assert_eq!(v.get::<i64>().unwrap().clone(), 789);
    }

    #[test]
    fn test_i64_get_invalid_type() {
        let v = KvsValue::from("abc");
        assert!(v.get::<i64>().is_none());
    }

    #[test]
    fn test_u64_from_ok() {
        let v = KvsValue::from(123456789u64);
        assert!(matches!(v, KvsValue::U64(x) if x == 123456789));
    }

    #[test]
    fn test_u64_tryfrom_ok() {
        let v = KvsValue::from(101112u64);
        assert_eq!(u64::try_from(&v).unwrap(), 101112);
    }

    #[test]
    fn test_u64_tryfrom_invalid_type() {
        let v = KvsValue::from(123i32);
        let err = u64::try_from(&v).unwrap_err();
        assert_eq!(err, "KvsValue is not a u64");
    }

    #[test]
    fn test_f64_from_ok() {
        let v = KvsValue::from(1.23f64);
        assert!(matches!(v, KvsValue::F64(x) if x == 1.23));
    }

    #[test]
    fn test_f64_tryfrom_ok() {
        let v = KvsValue::from(456.78f64);
        assert_eq!(f64::try_from(&v).unwrap(), 456.78f64);
    }

    #[test]
    fn test_f64_tryfrom_invalid_type() {
        let v = KvsValue::from(true);
        let err = f64::try_from(&v).unwrap_err();
        assert_eq!(err, "KvsValue is not a f64");
    }

    #[test]
    fn test_f64_get_ok() {
        let v = KvsValue::from(456.78f64);
        assert_eq!(v.get::<f64>().unwrap().clone(), 456.78f64);
    }

    #[test]
    fn test_f64_get_invalid_type() {
        let v = KvsValue::from(true);
        assert!(v.get::<f64>().is_none());
    }

    #[test]
    fn test_bool_from_ok() {
        let v = KvsValue::from(true);
        assert!(matches!(v, KvsValue::Boolean(true)));
    }

    #[test]
    fn test_bool_tryfrom_ok() {
        let v = KvsValue::from(true);
        assert!(bool::try_from(&v).unwrap());
    }

    #[test]
    fn test_bool_tryfrom_invalid_type() {
        let v = KvsValue::from(vec![KvsValue::from(1i32)]);
        let err = bool::try_from(&v).unwrap_err();
        assert_eq!(err, "KvsValue is not a bool");
    }

    #[test]
    fn test_bool_get_ok() {
        let v = KvsValue::from(true);
        assert!(*v.get::<bool>().unwrap());
    }

    #[test]
    fn test_bool_get_invalid_type() {
        let v = KvsValue::from(vec![KvsValue::from(1i32)]);
        assert!(v.get::<bool>().is_none());
    }

    #[test]
    fn test_string_from_ok() {
        let v = KvsValue::from(String::from("hello"));
        assert!(matches!(v, KvsValue::String(ref s) if s == "hello"));
    }

    #[test]
    fn test_string_tryfrom_ok() {
        let v = KvsValue::from("abc");
        assert_eq!(String::try_from(&v).unwrap(), "abc");
    }

    #[test]
    fn test_string_tryfrom_invalid_type() {
        let v = KvsValue::from(345.6f64);
        let err = String::try_from(&v).unwrap_err();
        assert_eq!(err, "KvsValue is not a String");
    }

    #[test]
    fn test_string_get_ok() {
        let v = KvsValue::from("abc");
        assert_eq!(v.get::<String>().unwrap().clone(), "abc");
    }

    #[test]
    fn test_string_get_invalid_type() {
        let v = KvsValue::from(345.6f64);
        assert!(v.get::<String>().is_none());
    }

    #[test]
    fn test_str_from_ok() {
        let v = KvsValue::from("world");
        assert!(matches!(v, KvsValue::String(ref s) if s == "world"));
    }

    #[test]
    fn test_unit_from_ok() {
        let v = KvsValue::from(());
        assert!(matches!(v, KvsValue::Null));
    }

    #[test]
    fn test_unit_tryfrom_ok() {
        let v = KvsValue::from(());
        <()>::try_from(&v).unwrap();
    }

    #[test]
    fn test_unit_tryfrom_invalid_type() {
        let v = KvsValue::from("");
        let err = <()>::try_from(&v).unwrap_err();
        assert_eq!(err, "KvsValue is not a Null (unit type)");
    }

    #[test]
    fn test_unit_get_ok() {
        let v = KvsValue::from(());
        v.get::<()>().unwrap();
    }

    #[test]
    fn test_unit_get_invalid_type() {
        let v = KvsValue::from("");
        assert!(v.get::<()>().is_none());
    }

    #[test]
    fn test_vec_from_ok() {
        let v = KvsValue::from(vec![KvsValue::from(1i32), KvsValue::from(2i32)]);
        assert!(matches!(v, KvsValue::Array(ref arr) if arr.len() == 2));
    }

    #[test]
    fn test_vec_tryfrom_ok() {
        let arr = vec![KvsValue::from(1i32), KvsValue::from(2i32)];
        let v = KvsValue::from(arr.clone());
        assert_eq!(Vec::<KvsValue>::try_from(&v).unwrap(), arr);
    }

    #[test]
    fn test_vec_tryfrom_invalid_type() {
        let v = KvsValue::from("");
        let err = Vec::<KvsValue>::try_from(&v).unwrap_err();
        assert_eq!(err, "KvsValue is not a Vec<KvsValue>");
    }

    #[test]
    fn test_vec_get_ok() {
        let arr = vec![KvsValue::from(1i32), KvsValue::from(2i32)];
        let v = KvsValue::from(arr.clone());
        assert_eq!(v.get::<Vec<KvsValue>>().unwrap().clone(), arr);
    }

    #[test]
    fn test_vec_get_invalid_type() {
        let v = KvsValue::from("");
        assert!(v.get::<Vec<KvsValue>>().is_none());
    }

    #[test]
    fn test_array_access() {
        let arr = vec![KvsValue::from(10i32), KvsValue::from(20i32)];
        let v = KvsValue::from(arr.clone());
        assert!(matches!(v, KvsValue::Array(_)), "Expected Array variant");
        if let KvsValue::Array(inner) = &v {
            assert_eq!(inner.first(), Some(&KvsValue::I32(10)));
            assert_eq!(inner.get(1), Some(&KvsValue::I32(20)));
            assert_eq!(inner.get(2), None);
        }
    }

    #[test]
    fn test_kvsmap_from_ok() {
        let mut map = KvsMap::new();
        map.insert("a".to_string(), KvsValue::from(1i32));
        let v = KvsValue::from(map.clone());
        if let KvsValue::Object(ref obj) = v {
            assert!(obj.contains_key("a"));
            assert!(matches!(obj.get("a"), Some(KvsValue::I32(1))));
        } else {
            panic!("Expected KvsValue::Object");
        }
    }

    #[test]
    fn test_kvsmap_tryfrom_ok() {
        let mut map = KvsMap::new();
        map.insert("x".to_string(), KvsValue::from(1i32));
        let v = KvsValue::from(map.clone());
        assert_eq!(KvsMap::try_from(&v).unwrap(), map);
    }

    #[test]
    fn test_kvsmap_tryfrom_invalid_type() {
        let v = KvsValue::from("");
        let err = KvsMap::try_from(&v).unwrap_err();
        assert_eq!(
            err,
            "KvsValue is not a std::collections::HashMap<String, KvsValue>"
        );
    }

    #[test]
    fn test_kvsmap_get_ok() {
        let mut map = KvsMap::new();
        map.insert("x".to_string(), KvsValue::from(1i32));
        let v = KvsValue::from(map.clone());
        assert_eq!(v.get::<KvsMap>().unwrap().clone(), map);
    }

    #[test]
    fn test_kvsmap_get_invalid_type() {
        let v = KvsValue::from("");
        assert!(v.get::<KvsMap>().is_none());
    }
}
