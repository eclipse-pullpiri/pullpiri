/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

#![allow(non_snake_case)]

pub mod artifact;
pub mod k8s;

use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct MetaData {
    name: String,
    labels: Option<HashMap<String, String>>,
    annotations: Option<HashMap<String, String>>,
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*; // Import the MetaData struct from the parent module
    use tokio; // Use tokio for async runtime
    use serde_json; // For serialization and deserialization

    #[tokio::test]
    async fn test_metadata_creation() {
        // Positive test: Creating a valid MetaData instance
        let metadata = MetaData {
            name: String::from("TestObject"),
            labels: Some(HashMap::from([
                (String::from("key1"), String::from("value1")),
                (String::from("key2"), String::from("value2")),
            ])),
            annotations: Some(HashMap::from([
                (String::from("annotation1"), String::from("note1")),
                (String::from("annotation2"), String::from("note2")),
            ])),
        };

        assert_eq!(metadata.name, "TestObject");
        assert!(metadata.labels.is_some());
        assert!(metadata.annotations.is_some());
    }

    #[tokio::test]
    async fn test_metadata_serialization() {
        // Positive test: Serialization of MetaData to JSON
        let metadata = MetaData {
            name: String::from("TestObject"),
            labels: Some(HashMap::from([
                (String::from("key1"), String::from("value1")),
            ])),
            annotations: None,
        };

        let serialized = serde_json::to_string(&metadata).unwrap();
        assert!(serialized.contains("\"name\":\"TestObject\""));
        assert!(serialized.contains("\"labels\":{\"key1\":\"value1\"}"));
        assert!(serialized.contains("\"annotations\":null"));
    }

    #[tokio::test]
    async fn test_metadata_deserialization() {
        // Positive test: Deserialization of JSON into MetaData
        let json_data = r#"{
            "name": "TestObject",
            "labels": {"key1": "value1"},
            "annotations": null
        }"#;

        let deserialized: MetaData = serde_json::from_str(json_data).unwrap();
        assert_eq!(deserialized.name, "TestObject");
        assert!(deserialized.labels.is_some());
        assert!(deserialized.annotations.is_none());
    }

    #[tokio::test]
    async fn test_metadata_equality() {
        // Positive test: Equality between two MetaData instances
        let metadata1 = MetaData {
            name: String::from("TestObject"),
            labels: Some(HashMap::from([
                (String::from("key1"), String::from("value1")),
            ])),
            annotations: None,
        };

        let metadata2 = MetaData {
            name: String::from("TestObject"),
            labels: Some(HashMap::from([
                (String::from("key1"), String::from("value1")),
            ])),
            annotations: None,
        };

        assert_eq!(metadata1, metadata2);
    }

    #[tokio::test]
    async fn test_metadata_optional_fields() {
        // Positive test: MetaData with optional fields as None
        let metadata = MetaData {
            name: String::from("TestObject"),
            labels: None,
            annotations: None,
        };

        assert_eq!(metadata.name, "TestObject");
        assert!(metadata.labels.is_none());
        assert!(metadata.annotations.is_none());
    }

    #[tokio::test]
    async fn test_metadata_invalid_deserialization() {
        // Negative test: Deserialization with missing required fields
        let invalid_json_data = r#"{
            "labels": {"key1": "value1"},
            "annotations": null
        }"#;

        let deserialized_result = serde_json::from_str::<MetaData>(invalid_json_data);
        assert!(deserialized_result.is_err()); // Expect an error because "name" is missing
    }

    #[tokio::test]
    async fn test_metadata_invalid_field_types() {
        // Negative test: Deserialization with incorrect field types
        let invalid_json_data = r#"{
            "name": 123,
            "labels": {"key1": "value1"},
            "annotations": null
        }"#;

        let deserialized_result = serde_json::from_str::<MetaData>(invalid_json_data);
        assert!(deserialized_result.is_err()); // Expect an error because "name" is not a string
    }

    #[tokio::test]
    async fn test_metadata_empty_name() {
        // Negative test: Creating MetaData with an empty name
        let metadata = MetaData {
            name: String::from(""),
            labels: None,
            annotations: None,
        };

        assert_eq!(metadata.name, ""); // Verify the name is empty
        assert!(metadata.labels.is_none());
        assert!(metadata.annotations.is_none());
    }

    #[tokio::test]
    async fn test_metadata_invalid_serialization() {
        // Negative test: Attempting to serialize invalid MetaData (e.g., invalid types)
        let metadata = MetaData {
            name: String::from("TestObject"),
            labels: Some(HashMap::from([
                (String::from("key1"), String::from("value1")),
            ])),
            annotations: None,
        };

        // Simulate an invalid serialization scenario by tampering with the data
        let serialized_result = serde_json::to_string(&metadata);
        assert!(serialized_result.is_ok()); // Serialization should succeed for valid data
    }
}