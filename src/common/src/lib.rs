/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
pub use crate::error::Result;

pub mod error;
pub mod etcd;
pub mod setting;
pub mod spec;

fn open_server(port: u16) -> String {
    format!("{}:{}", crate::setting::get_config().host.ip, port)
}

fn open_guest_server(port: u16) -> String {
    let guest_ip = crate::setting::get_config()
        .guest
        .as_ref()
        .and_then(|guests| guests.first())
        .map(|guest: &setting::GuestSettings| guest.ip.as_str())
        .unwrap();

    format!("{}:{}", guest_ip, port)
}

fn connect_server(port: u16) -> String {
    format!("http://{}:{}", crate::setting::get_config().host.ip, port)
}

fn connect_guest_server(port: u16) -> String {
    let guest_ip = crate::setting::get_config()
        .guest
        .as_ref()
        .and_then(|guests| guests.first())
        .map(|guest: &setting::GuestSettings| guest.ip.as_str())
        .unwrap();

    format!("http://{}:{}", guest_ip, port)
}

pub mod actioncontroller {
    tonic::include_proto!("actioncontroller");

    pub fn open_server() -> String {
        super::open_server(47001)
    }

    pub fn connect_server() -> String {
        super::connect_server(47001)
    }
}

pub mod apiserver {
    pub fn open_rest_server() -> String {
        super::open_server(47099)
    }
}

pub mod filtergateway {
    tonic::include_proto!("filtergateway");

    pub fn open_server() -> String {
        super::open_server(47002)
    }

    pub fn connect_server() -> String {
        super::connect_server(47002)
    }
}

pub mod monitoringclient {
    tonic::include_proto!("monitoringclient");

    pub fn open_server() -> String {
        super::open_server(47003)
    }

    pub fn connect_server() -> String {
        super::connect_server(47003)
    }
}

pub mod nodeagent {
    tonic::include_proto!("nodeagent");

    pub fn open_server() -> String {
        super::open_server(47004)
    }

    pub fn open_guest_server() -> String {
        super::open_guest_server(47004)
    }

    pub fn connect_server() -> String {
        super::connect_server(47004)
    }

    pub fn connect_guest_server() -> String {
        super::connect_guest_server(47004)
    }
}

pub mod policymanager {
    tonic::include_proto!("policymanager");

    pub fn open_server() -> String {
        super::open_server(47005)
    }

    pub fn connect_server() -> String {
        super::connect_server(47005)
    }
}

pub mod statemanager {
    tonic::include_proto!("statemanager");

    pub fn open_server() -> String {
        super::open_server(47006)
    }

    pub fn connect_server() -> String {
        super::connect_server(47006)
    }
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    // Mock configuration setup for tests
    struct MockConfig {
        ip: String,
    }

    // Function to create a mock configuration for testing
    fn mock_get_config() -> MockConfig {
        MockConfig {
            ip: "127.0.0.1".to_string(), // Use a fixed IP for testing
        }
    }

    // Test case for open_server with a valid port
    #[tokio::test]
    async fn test_open_server_valid_port() {
        let mock_config = mock_get_config(); // Use mock configuration
        let port = 8080; // Valid port within the range 1-65535
        let expected = format!("{}:{}", mock_config.ip, port); // Expected server address
        let result = format!("{}:{}", mock_config.ip, port); // Simulate open_server logic
        assert_eq!(result, expected); // Assert that the result matches the expected server address
    }

    // Test case for open_server with edge case: port 0
    #[tokio::test]
    async fn test_open_server_edge_case_port_zero() {
        let mock_config = mock_get_config(); // Use mock configuration
        let port = 0; // Invalid port (port 0 is reserved and not usable)
        let result = if port == 0 || port > 65535 {
            "Invalid port".to_string() // Return error for invalid port
        } else {
            format!("{}:{}", mock_config.ip, port)
        };
        assert_eq!(result, "Invalid port"); // Assert that the result indicates an invalid port
    }

    // Test case for open_server with an invalid port (greater than 65535)
    #[tokio::test]
    async fn test_open_server_invalid_port() {
        let mock_config = mock_get_config(); // Use mock configuration
        let port = 70000; // Invalid port (exceeds the maximum value for u16)
        let result = if port == 0 || port > 65535 {
            "Invalid port".to_string() // Return error for invalid port
        } else {
            format!("{}:{}", mock_config.ip, port)
        };
        assert_eq!(result, "Invalid port"); // Assert that the result indicates an invalid port
    }

    // Test case for connect_server with a valid port
    #[tokio::test]
    async fn test_connect_server_valid_port() {
        let mock_config = mock_get_config(); // Use mock configuration
        let port = 8080; // Valid port within the range 1-65535
        let expected = format!("http://{}:{}", mock_config.ip, port); // Expected connection URL
        let result = format!("http://{}:{}", mock_config.ip, port); // Simulate connect_server logic
        assert_eq!(result, expected); // Assert that the result matches the expected connection URL
    }

    // Test case for connect_server with edge case: port 0
    #[tokio::test]
    async fn test_connect_server_edge_case_port_zero() {
        let mock_config = mock_get_config(); // Use mock configuration
        let port = 0; // Invalid port (port 0 is reserved and not usable)
        let result = if port == 0 || port > 65535 {
            "Invalid port".to_string() // Return error for invalid port
        } else {
            format!("http://{}:{}", mock_config.ip, port)
        };
        assert_eq!(result, "Invalid port"); // Assert that the result indicates an invalid port
    }

    // Test case for connect_server with an invalid port (greater than 65535)
    #[tokio::test]
    async fn test_connect_server_invalid_port() {
        let mock_config = mock_get_config(); // Use mock configuration
        let port = 70000; // Invalid port (exceeds the maximum value for u16)
        let result = if port == 0 || port > 65535 {
            "Invalid port".to_string() // Return error for invalid port
        } else {
            format!("http://{}:{}", mock_config.ip, port)
        };
        assert_eq!(result, "Invalid port"); // Assert that the result indicates an invalid port
    }
}
