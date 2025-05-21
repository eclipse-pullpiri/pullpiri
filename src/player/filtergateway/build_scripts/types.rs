// Module for DDS type definitions
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Definition of DDS data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdsData {
    pub name: String,
    pub value: String,
    pub fields: HashMap<String, String>,
}

/// Convert IDL type to Rust type
pub fn idl_to_rust_type(idl_type: &str) -> &str {
    match idl_type {
        "boolean" => "bool",
        "short" | "int16_t" => "i16",
        "unsigned short" | "uint16_t" => "u16",
        "long" | "int32_t" => "i32",
        "unsigned long" | "uint32_t" => "u32",
        "long long" | "int64_t" => "i64",
        "unsigned long long" | "uint64_t" => "u64",
        "float" => "f32",
        "double" => "f64",
        "string" | "std::string" => "String",
        "octet" | "byte" => "u8",
        "char" => "char",
        _ => "String", // Default to String for complex types
    }
}
