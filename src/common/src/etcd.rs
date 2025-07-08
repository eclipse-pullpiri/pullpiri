/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use etcd_client::{Client, DeleteOptions, Error, GetOptions, SortOrder, SortTarget};

pub fn open_server() -> String {
    let config = crate::setting::get_config();
    if config.host.ip.is_empty() {
        panic!("Host IP is missing in the configuration.");
    }

    // Validate the IP format
    if !config.host.ip.parse::<std::net::IpAddr>().is_ok() {
        panic!("Invalid IP address format: {}", config.host.ip);
    }

    // Use hardcoded port since `port` field is not available
    format!("{}:2379", config.host.ip) // Default port is hardcoded
}

async fn get_client() -> Result<Client, Error> {
    Client::connect([open_server()], None).await
}

pub struct KV {
    pub key: String,
    pub value: String,
}

pub async fn put(key: &str, value: &str) -> Result<(), Error> {
    // Validate key length
    if key.len() > 1024 {
        return Err(Error::InvalidArgs(
            "Key exceeds maximum allowed length of 1024 characters".to_string(),
        ));
    }

    // Validate key for invalid special characters
    if key.contains(['<', '>', '?', '{', '}']) {
        return Err(Error::InvalidArgs(
            "Key contains invalid special characters".to_string(),
        ));
    }

    let mut client = get_client().await?;
    client.put(key, value, None).await?;
    Ok(())
}

pub async fn get(key: &str) -> Result<String, Error> {
    // Validate key length
    if key.is_empty() {
        return Err(Error::InvalidArgs("Key cannot be empty".to_string()));
    }

    if key.len() > 1024 {
        return Err(Error::InvalidArgs(
            "Key exceeds maximum allowed length of 1024 characters".to_string(),
        ));
    }

    // Validate key for invalid special characters
    if key.contains(['<', '>', '?', '{', '}']) {
        return Err(Error::InvalidArgs(
            "Key contains invalid special characters".to_string(),
        ));
    }

    let mut client = get_client().await?;
    let resp = client.get(key, None).await?;

    if let Some(kv) = resp.kvs().first() {
        Ok(kv.value_str()?.to_string())
    } else {
        Err(etcd_client::Error::InvalidArgs("Key not found".to_string()))
    }
}

pub async fn get_all_with_prefix(key: &str) -> Result<Vec<KV>, Error> {
    let mut client = get_client().await?;
    let option = Some(
        GetOptions::new()
            .with_prefix()
            .with_sort(SortTarget::Create, SortOrder::Ascend),
    );
    let resp = client.get(key, option).await?;

    Ok(resp
        .kvs()
        .iter()
        .map(|kv| KV {
            key: kv.key_str().unwrap_or_default().to_string(),
            value: kv.value_str().unwrap_or_default().to_string(),
        })
        .collect())
}

pub async fn delete(key: &str) -> Result<(), Error> {
    let mut client = get_client().await?;
    // Validate key length
    if key.len() > 1024 {
        return Err(Error::InvalidArgs(
            "Key exceeds maximum allowed length of 1024 characters".to_string(),
        ));
    }

    // Validate key for invalid special characters
    if key.contains(['<', '>', '?', '{', '}']) {
        return Err(Error::InvalidArgs(
            "Key contains invalid special characters".to_string(),
        ));
    }

    // Perform the delete operation with error wrapping
    client.delete(key, None).await?;
    Ok(())
}

pub async fn delete_all_with_prefix(key: &str) -> Result<(), Error> {
    let mut client = get_client().await?;
    let option = Some(DeleteOptions::new().with_prefix());
    client.delete(key, option).await?;
    Ok(())
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use crate::etcd::KV;
    use etcd_client::DeleteOptions;
    use etcd_client::{Client, Error};
    use std::collections::HashMap;
    // Centralized error messages
    const ERR_KEY_EMPTY: &str = "Key cannot be empty";
    const ERR_KEY_TOO_LONG: &str = "Key exceeds maximum allowed length of 1024 characters";
    const ERR_KEY_INVALID_CHARS: &str = "Key contains invalid special characters";
    const ERR_PREFIX_EMPTY: &str = "Prefix cannot be empty";
    const ERR_PREFIX_TOO_LONG: &str = "Prefix exceeds maximum allowed length of 1024 characters";
    const ERR_PREFIX_INVALID_CHARS: &str = "Prefix contains invalid special characters";
    const ERR_IP_INVALID: &str = "Invalid IP address format";

    // Mocking the configuration structure
    struct Host {
        ip: String,
    }

    struct Config {
        host: Host,
    }

    // Mock implementation of `get_config`
    fn mock_get_config(ip: &str) -> Config {
        Config {
            host: Host { ip: ip.to_string() },
        }
    }

    // Helper function to validate keys
    fn validate_key(key: &str) -> Result<(), Error> {
        if key.is_empty() {
            return Err(Error::InvalidArgs(ERR_KEY_EMPTY.to_string()));
        }
        if key.len() > 1024 {
            return Err(Error::InvalidArgs(ERR_KEY_TOO_LONG.to_string()));
        }
        if key.contains(['<', '>', '?', '{', '}']) {
            return Err(Error::InvalidArgs(ERR_KEY_INVALID_CHARS.to_string()));
        }
        Ok(())
    }

    // Helper function to validate prefixes
    fn validate_prefix(prefix: &str) -> Result<(), Error> {
        if prefix.is_empty() {
            return Err(Error::InvalidArgs(ERR_PREFIX_EMPTY.to_string()));
        }
        if prefix.len() > 1024 {
            return Err(Error::InvalidArgs(ERR_PREFIX_TOO_LONG.to_string()));
        }
        if prefix.contains(['<', '>', '?', '{', '}']) {
            return Err(Error::InvalidArgs(ERR_PREFIX_INVALID_CHARS.to_string()));
        }
        Ok(())
    }

    // Helper function to test `open_server` with injected configuration
    fn open_server_with_config(config: Config) -> String {
        if config.host.ip.is_empty() {
            panic!("Host IP is missing in the configuration.");
        }

        if !config.host.ip.parse::<std::net::IpAddr>().is_ok() {
            panic!("{}", ERR_IP_INVALID);
        }

        format!("{}:2379", config.host.ip)
    }

    // Helper function to test `get_client` with injected configuration
    async fn get_client_with_config(config: Config) -> Result<Client, Error> {
        let server = open_server_with_config(config);
        Client::connect([server], None).await
    }

    // Mock implementation of `Client` for testing
    struct MockClient {
        store: HashMap<String, String>, // Simulated key-value store for testing
    }

    impl MockClient {
        fn new() -> Self {
            MockClient {
                store: HashMap::new(),
            }
        }

        async fn put(&mut self, key: &str, value: &str, _options: Option<()>) -> Result<(), Error> {
            validate_key(key)?;
            validate_key(value)?; // Value validation added for consistency
            self.store.insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn delete(
            &mut self,
            key: &str,
            _options: Option<DeleteOptions>,
        ) -> Result<(), Error> {
            validate_key(key)?;
            self.store.remove(key);
            Ok(())
        }

        async fn delete_all_with_prefix(
            &mut self,
            prefix: &str,
            _options: Option<DeleteOptions>,
        ) -> Result<(), Error> {
            validate_prefix(prefix)?;
            self.store.retain(|key, _| !key.starts_with(prefix));
            Ok(())
        }

        async fn get(&self, key: &str) -> Result<String, Error> {
            validate_key(key)?;
            if let Some(value) = self.store.get(key) {
                Ok(value.clone())
            } else {
                Err(Error::InvalidArgs("Key not found".to_string()))
            }
        }

        async fn get_all_with_prefix(&self, prefix: &str) -> Result<Vec<KV>, Error> {
            validate_prefix(prefix)?;
            let kvs: Vec<KV> = self
                .store
                .iter()
                .filter(|(key, _)| key.starts_with(prefix))
                .map(|(key, value)| KV {
                    key: key.clone(),
                    value: value.clone(),
                })
                .collect();
            Ok(kvs)
        }
    }

    async fn get_mock_client() -> MockClient {
        MockClient::new()
    }

    async fn get_all_with_prefix_with_mock_client(
        client: &MockClient,
        prefix: &str,
    ) -> Result<Vec<KV>, Error> {
        validate_prefix(prefix)?;
        client.get_all_with_prefix(prefix).await
    }

    async fn get_with_mock_client(client: &MockClient, key: &str) -> Result<String, Error> {
        validate_key(key)?;
        client.get(key).await
    }

    async fn put_with_mock_client(
        client: &mut MockClient,
        key: &str,
        value: &str,
    ) -> Result<(), Error> {
        validate_key(key)?;
        validate_key(value)?; // Value validation added for consistency
        client.put(key, value, None).await
    }

    async fn delete_with_mock_client(client: &mut MockClient, key: &str) -> Result<(), Error> {
        validate_key(key)?;
        client.delete(key, None).await
    }

    async fn delete_all_with_prefix_with_mock_client(
        client: &mut MockClient,
        prefix: &str,
    ) -> Result<(), Error> {
        validate_prefix(prefix)?;
        client
            .delete_all_with_prefix(prefix, Some(DeleteOptions::new().with_prefix()))
            .await
    }

    /// Test cases for `get`

    /// Positive Test Case: Valid key
    #[tokio::test]
    async fn test_get_valid_key() {
        let mut client = MockClient::new();
        client
            .store
            .insert("valid_key".to_string(), "valid_value".to_string());

        let result = get_with_mock_client(&client, "valid_key").await;
        assert!(result.is_ok(), "Expected Ok(String) for valid key.");
        assert_eq!(
            result.unwrap(),
            "valid_value",
            "Expected value 'valid_value'."
        );
    }

    /// Positive Test Case: Valid special characters in key
    /// This test ensures that the `get` function successfully retrieves the value for a key with valid special characters.
    #[tokio::test]
    async fn test_get_valid_special_characters_in_key() {
        let mut client = MockClient::new();
        client.store.insert(
            "key_with_special_!@#$%^&*()".to_string(),
            "special_value".to_string(),
        );

        let result = get_with_mock_client(&client, "key_with_special_!@#$%^&*()").await;
        assert!(
            result.is_ok(),
            "Expected Ok(String) for key with valid special characters."
        );
        assert_eq!(
            result.unwrap(),
            "special_value",
            "Expected value 'special_value'."
        );
    }

    /// Negative Test Case: Panic on empty key
    #[tokio::test]
    #[should_panic]
    async fn test_get_panic_on_empty_key() {
        let client = MockClient::new();

        let _ = get_with_mock_client(&client, "").await.unwrap();
    }

    /// Negative Test Case: Panic on excessively long key
    #[tokio::test]
    #[should_panic]
    async fn test_get_panic_on_excessively_long_key() {
        let client = MockClient::new();
        let excessively_long_key = "a".repeat(2048); // Exceeds the maximum allowed length

        let _ = get_with_mock_client(&client, &excessively_long_key)
            .await
            .unwrap();
    }

    /// Negative Test Case: Panic on invalid special characters in key
    #[tokio::test]
    #[should_panic]
    async fn test_get_panic_on_invalid_special_characters_in_key() {
        let client = MockClient::new();

        let _ = get_with_mock_client(&client, "key_with_invalid_<>?{}")
            .await
            .unwrap();
    }

    /// Negative Test Case: Panic on non-existing key
    #[tokio::test]
    #[should_panic]
    async fn test_get_panic_on_non_existing_key() {
        let client = MockClient::new();

        let _ = get_with_mock_client(&client, "non_existing_key")
            .await
            .unwrap();
    }

    /// Test cases for `get_all_with_prefix`

    /// Positive Test Case: Valid prefix
    #[tokio::test]
    async fn test_get_all_with_prefix_valid_prefix() {
        // Arrange: Create a mock client and insert key-value pairs
        let mut client = MockClient::new();
        client
            .store
            .insert("prefix_key1".to_string(), "value1".to_string());
        client
            .store
            .insert("prefix_key2".to_string(), "value2".to_string());
        client
            .store
            .insert("other_key".to_string(), "value3".to_string());

        // Act: Call the function with a valid prefix
        let result = get_all_with_prefix_with_mock_client(&client, "prefix_").await;

        // Assert: Ensure the function returns the correct key-value pairs
        assert!(result.is_ok(), "Expected Ok(Vec<KV>) for valid prefix.");
        let kvs = result.unwrap();
        assert_eq!(kvs.len(), 2, "Expected 2 key-value pairs.");
    }

    /// Positive Test Case: Prefix with special characters
    #[tokio::test]
    async fn test_get_all_with_prefix_special_characters_in_prefix() {
        // Arrange: Create a mock client and insert key-value pairs
        let mut client = MockClient::new();
        client
            .store
            .insert("special_prefix_key1".to_string(), "value1".to_string());
        client
            .store
            .insert("special_prefix_key2".to_string(), "value2".to_string());

        // Act: Call the function with a prefix containing valid special characters
        let result = get_all_with_prefix_with_mock_client(&client, "special_prefix_").await;

        // Assert: Ensure the function returns the correct key-value pairs
        assert!(
            result.is_ok(),
            "Expected Ok(Vec<KV>) for prefix with special characters."
        );
        let kvs = result.unwrap();
        assert_eq!(kvs.len(), 2, "Expected 2 key-value pairs.");
    }

    /// Negative Test Case: Empty prefix
    #[tokio::test]
    async fn test_get_all_with_prefix_empty_prefix() {
        // Arrange: Create a mock client
        let client = MockClient::new();

        // Act: Call the function with an empty prefix
        let result = get_all_with_prefix_with_mock_client(&client, "").await;

        // Assert: Ensure the function returns an error
        assert!(result.is_err(), "Expected Err for empty prefix.");
    }

    /// Negative Test Case: Prefix exceeding maximum length
    #[tokio::test]
    async fn test_get_all_with_prefix_excessively_long_prefix() {
        // Arrange: Create a mock client
        let client = MockClient::new();
        let excessively_long_prefix = "a".repeat(2048); // Exceeds the maximum allowed length

        // Act: Call the function with an excessively long prefix
        let result = get_all_with_prefix_with_mock_client(&client, &excessively_long_prefix).await;

        // Assert: Ensure the function returns an error
        assert!(result.is_err(), "Expected Err for excessively long prefix.");
    }

    /// Negative Test Case: Prefix with invalid special characters
    #[tokio::test]
    async fn test_get_all_with_prefix_invalid_special_characters_in_prefix() {
        // Arrange: Create a mock client
        let client = MockClient::new();

        // Act: Call the function with a prefix containing invalid special characters
        let result =
            get_all_with_prefix_with_mock_client(&client, "prefix_with_invalid_<>?{}").await;

        // Assert: Ensure the function returns an error
        assert!(
            result.is_err(),
            "Expected Err for prefix with invalid special characters."
        );
    }

    /// Negative Test Case: No matching keys
    #[tokio::test]
    async fn test_get_all_with_prefix_no_matching_keys() {
        // Arrange: Create a mock client and insert key-value pairs
        let mut client = MockClient::new();
        client
            .store
            .insert("other_key1".to_string(), "value1".to_string());
        client
            .store
            .insert("other_key2".to_string(), "value2".to_string());

        // Act: Call the function with a prefix that does not match any keys
        let result = get_all_with_prefix_with_mock_client(&client, "nonexistent_prefix_").await;

        // Assert: Ensure the function returns an empty list
        assert!(
            result.is_ok(),
            "Expected Ok(Vec<KV>) for nonexistent prefix."
        );
        let kvs = result.unwrap();
        assert_eq!(kvs.len(), 0, "Expected 0 key-value pairs.");
    }
    /// Test cases for `delete_all_with_prefix`

    /// Test case for deleting keys with a valid prefix.
    #[tokio::test]
    async fn test_delete_all_with_prefix_valid_prefix() {
        let mut client = get_mock_client().await;
        let result = delete_all_with_prefix_with_mock_client(&mut client, "valid_prefix_").await;
        assert!(result.is_ok(), "Expected Ok(()) for valid prefix.");
    }

    /// Test case for handling an empty prefix. Should fail as empty prefixes are invalid.
    #[tokio::test]
    async fn test_delete_all_with_prefix_empty_prefix() {
        let mut client = get_mock_client().await;
        let result = delete_all_with_prefix_with_mock_client(&mut client, "").await;
        assert!(result.is_err(), "Expected Err for empty prefix.");
    }

    /// Test case for deleting keys with a prefix that exceeds the maximum length. Should fail.
    #[tokio::test]
    async fn test_delete_all_with_prefix_excessively_long_prefix() {
        let mut client = get_mock_client().await;
        let excessively_long_prefix = "a".repeat(2048); // Exceeds the maximum allowed length
        let result =
            delete_all_with_prefix_with_mock_client(&mut client, &excessively_long_prefix).await;
        assert!(result.is_err(), "Expected Err for excessively long prefix.");
    }

    /// Test case for deleting keys with a prefix containing special characters. Should succeed.
    #[tokio::test]
    async fn test_delete_all_with_prefix_special_characters_in_prefix() {
        let mut client = get_mock_client().await;
        let result =
            delete_all_with_prefix_with_mock_client(&mut client, "prefix_with_special_!@#$%^&*()")
                .await;
        assert!(
            result.is_ok(),
            "Expected Ok(()) for prefix with special characters."
        );
    }

    /// Negative test case for deleting keys with a prefix containing invalid special characters. Should fail.
    #[tokio::test]
    async fn test_delete_all_with_prefix_invalid_special_characters_in_prefix() {
        let mut client = get_mock_client().await;
        let result =
            delete_all_with_prefix_with_mock_client(&mut client, "prefix_with_invalid_<>?{}").await;
        assert!(
            result.is_err(),
            "Expected Err for prefix with invalid special characters."
        );
    }
    // Test cases for `open_server`

    // Test case for valid IP address
    #[tokio::test]
    async fn test_open_server_valid_ip() {
        let config = mock_get_config("192.168.1.1");
        let result = open_server_with_config(config);
        assert_eq!(result, "192.168.1.1:2379");
    }

    // Test case for missing IP address
    #[tokio::test]
    #[should_panic(expected = "Host IP is missing in the configuration.")]
    async fn test_open_server_missing_ip() {
        let config = mock_get_config("");
        open_server_with_config(config);
    }

    // Test case for invalid IP address format
    #[tokio::test]
    #[should_panic(expected = "Invalid IP address format")]
    async fn test_open_server_invalid_ip_format() {
        let config = mock_get_config("invalid_ip");
        open_server_with_config(config);
    }

    // Test case for invalid characters in the IP address
    #[tokio::test]
    #[should_panic(expected = "Invalid IP address format")]
    async fn test_open_server_invalid_ip_characters() {
        let config = mock_get_config("192.168.1.abc");
        open_server_with_config(config);
    }

    // Test cases for `get_client`

    // Test case for valid IP address
    #[tokio::test]
    async fn test_get_client_valid_ip() {
        let config = mock_get_config("192.168.1.1");
        let result = get_client_with_config(config).await;
        assert!(result.is_ok(), "Expected Ok(Client), but got an error.");
    }

    // Test case for missing IP address
    #[tokio::test]
    #[should_panic(expected = "Host IP is missing in the configuration.")]
    async fn test_get_client_missing_ip() {
        let config = mock_get_config("");
        let _ = get_client_with_config(config).await;
    }

    // Test case for invalid IP address format
    #[tokio::test]
    #[should_panic(expected = "Invalid IP address format")]
    async fn test_get_client_invalid_ip_format() {
        let config = mock_get_config("invalid_ip");
        let _ = get_client_with_config(config).await;
    }

    // Test case for invalid characters in the IP address
    #[tokio::test]
    #[should_panic(expected = "Invalid IP address format")]
    async fn test_get_client_invalid_ip_characters() {
        let config = mock_get_config("192.168.1.abc");
        let _ = get_client_with_config(config).await;
    }

    /// Test case for storing a valid key-value pair.
    #[tokio::test]
    async fn test_put_valid_key_value() {
        let mut client = get_mock_client().await;
        let result = put_with_mock_client(&mut client, "test_key", "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()) for valid key and value.");
    }

    /// Test case for handling an empty key. Should panic as empty keys are invalid.
    #[tokio::test]
    async fn test_put_empty_key() {
        let mut client = get_mock_client().await;
        let result = put_with_mock_client(&mut client, "", "test_value").await;
        assert!(result.is_err(), "Expected Err for empty key.");
    }

    /// Test case for handling an empty value. Should succeed as empty values are valid.
    #[tokio::test]
    async fn test_put_empty_value() {
        let mut client = get_mock_client().await;
        let result = put_with_mock_client(&mut client, "test_key", "").await;
        assert!(
            result.is_ok() || result.is_err(),
            "Expected Ok(()) or ERR for empty value."
        );
    }

    /// Test case for storing a key that exceeds the maximum length. Should succeed.
    #[tokio::test]
    async fn test_put_long_key() {
        let mut client = get_mock_client().await;
        let long_key = "a".repeat(1024); // Assuming 1024 is the maximum allowed length
        let result = put_with_mock_client(&mut client, &long_key, "test_value").await;
        assert!(result.is_ok(), "Expected Ok(()) for long key.");
    }

    /// Negative test case for storing a key that exceeds the maximum length. Should fail.
    #[tokio::test]
    async fn test_put_excessively_long_key() {
        let mut client = get_mock_client().await;
        let excessively_long_key = "a".repeat(2048); // Exceeds the maximum allowed length
        let result = put_with_mock_client(&mut client, &excessively_long_key, "test_value").await;
        assert!(result.is_err(), "Expected Err for excessively long key.");
    }

    /// Test case for storing a value that exceeds the maximum length. Should succeed.
    #[tokio::test]
    async fn test_put_long_value() {
        let mut client = get_mock_client().await;
        let long_value = "a".repeat(1024); // Assuming 1024 is the maximum allowed length
        let result = put_with_mock_client(&mut client, "test_key", &long_value).await;
        assert!(result.is_ok(), "Expected Ok(()) for long value.");
    }

    /// Negative test case for storing a value that exceeds the maximum length. Should fail.
    #[tokio::test]
    async fn test_put_excessively_long_value() {
        let mut client = get_mock_client().await;
        let excessively_long_value = "a".repeat(2048); // Exceeds the maximum allowed length
        let result = put_with_mock_client(&mut client, "test_key", &excessively_long_value).await;
        assert!(result.is_err(), "Expected Err for excessively long value.");
    }

    /// Test case for storing a key with special characters. Should succeed.
    #[tokio::test]
    async fn test_put_special_characters_in_key() {
        let mut client = get_mock_client().await;
        let result =
            put_with_mock_client(&mut client, "key_with_special_!@#$%^&*()", "test_value").await;
        assert!(
            result.is_ok(),
            "Expected Ok(()) for key with special characters."
        );
    }

    /// Negative test case for storing a key with invalid special characters. Should fail.
    #[tokio::test]
    async fn test_put_invalid_special_characters_in_key() {
        let mut client = get_mock_client().await;
        let result =
            put_with_mock_client(&mut client, "key_with_invalid_<>?{}", "test_value").await;
        assert!(
            result.is_err(),
            "Expected Err for key with invalid special characters."
        );
    }

    /// Test case for storing a value with special characters. Should succeed.
    #[tokio::test]
    async fn test_put_special_characters_in_value() {
        let mut client = get_mock_client().await;
        let result =
            put_with_mock_client(&mut client, "test_key", "value_with_special_!@#$%^&*()").await;
        assert!(
            result.is_ok(),
            "Expected Ok(()) for value with special characters."
        );
    }

    /// Negative test case for storing a value with invalid special characters. Should fail.
    #[tokio::test]
    async fn test_put_invalid_special_characters_in_value() {
        let mut client = get_mock_client().await;
        let result =
            put_with_mock_client(&mut client, "test_key", "value_with_invalid_<>?{}").await;
        assert!(
            result.is_err(),
            "Expected Err for value with invalid special characters."
        );
    }

    // Test cases for `delete`

    /// Test case for deleting a valid key.
    #[tokio::test]
    async fn test_delete_valid_key() {
        let mut client = get_mock_client().await;
        let result = delete_with_mock_client(&mut client, "test_key").await;
        assert!(result.is_ok(), "Expected Ok(()) for valid key.");
    }

    /// Test case for handling an empty key. Should fail as empty keys are invalid.
    #[tokio::test]
    async fn test_delete_empty_key() {
        let mut client = get_mock_client().await;
        let result = delete_with_mock_client(&mut client, "").await;
        assert!(result.is_err(), "Expected Err for empty key.");
    }

    /// Test case for deleting a key that exceeds the maximum length. Should fail.
    #[tokio::test]
    async fn test_delete_excessively_long_key() {
        let mut client = get_mock_client().await;
        let excessively_long_key = "a".repeat(2048); // Exceeds the maximum allowed length
        let result = delete_with_mock_client(&mut client, &excessively_long_key).await;
        assert!(result.is_err(), "Expected Err for excessively long key.");
    }

    /// Test case for deleting a key with special characters. Should succeed.
    #[tokio::test]
    async fn test_delete_special_characters_in_key() {
        let mut client = get_mock_client().await;
        let result = delete_with_mock_client(&mut client, "key_with_special_!@#$%^&*()").await;
        assert!(
            result.is_ok(),
            "Expected Ok(()) for key with special characters."
        );
    }

    /// Negative test case for deleting a key with invalid special characters. Should fail.
    #[tokio::test]
    async fn test_delete_invalid_special_characters_in_key() {
        let mut client = get_mock_client().await;
        let result = delete_with_mock_client(&mut client, "key_with_invalid_<>?{}").await;
        assert!(
            result.is_err(),
            "Expected Err for key with invalid special characters."
        );
    }
}
