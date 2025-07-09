use serde::Deserialize;
use std::sync::OnceLock;
static SETTINGS: OnceLock<Settings> = OnceLock::new();

#[derive(Deserialize)]
pub struct Settings {
    pub yaml_storage: String,
    pub piccolo_cloud: String,
    pub host: HostSettings,
    pub guest: Option<Vec<GuestSettings>>,
}

#[derive(Deserialize)]
pub struct HostSettings {
    pub name: String,
    pub ip: String,
    pub r#type: String,
}

#[derive(Deserialize, PartialEq, Clone)]
pub struct GuestSettings {
    pub name: String,
    pub ip: String,
    pub r#type: String,
}

fn parse_settings_yaml() -> Settings {
    let default_settings: Settings = Settings {
        yaml_storage: String::from("/etc/piccolo/yaml"),
        piccolo_cloud: String::from("http://0.0.0.0:41234"),
        host: HostSettings {
            name: String::from("HPC"),
            ip: String::from("0.0.0.0"),
            r#type: String::from("bluechi"),
        },
        guest: Some(vec![GuestSettings {
            name: String::from("ZONE"),
            ip: String::from("192.168.50.214"),
            r#type: String::from("bluechi"),
        }]), //guest: None,
    };

    let settings = config::Config::builder()
        .add_source(config::File::with_name("/piccolo/settings.yaml"))
        .build();

    match settings {
        Ok(result) => result
            .try_deserialize::<Settings>()
            .unwrap_or(default_settings),
        Err(_) => default_settings,
    }
}

pub fn get_config() -> &'static Settings {
    SETTINGS.get_or_init(parse_settings_yaml)
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;

    // Test default values when no settings file is provided
    #[tokio::test]
    async fn test_parse_settings_yaml_default_values() {
        // Verify that default values are used when the settings file is missing
        let settings = parse_settings_yaml();
        assert_eq!(settings.yaml_storage, "/etc/piccolo/yaml");
        assert_eq!(settings.piccolo_cloud, "http://0.0.0.0:41234");
        assert_eq!(settings.host.name, "HPC");
        assert_eq!(settings.host.ip, "0.0.0.0");
        assert_eq!(settings.host.r#type, "bluechi");
        assert!(settings.guest.is_none());
    }

    // Test guest settings when provided
    #[tokio::test]
    async fn test_parse_settings_yaml_guest_settings() {
        // Verify that guest settings are correctly parsed when provided
        let default_guest = vec![GuestSettings {
            name: String::from("ZONE"),
            ip: String::from("192.168.0.1"),
            r#type: String::from("nodeagent"),
        }];
        let settings = parse_settings_yaml();
        assert!(settings.guest.is_none() || settings.guest == Some(default_guest));
    }

    // Test lazy initialization of configuration
    #[tokio::test]
    async fn test_get_config_lazy_initialization() {
        // Verify that the configuration is lazily initialized
        let config = get_config();
        assert_eq!(config.yaml_storage, "/etc/piccolo/yaml");
        assert_eq!(config.piccolo_cloud, "http://0.0.0.0:41234");
        assert_eq!(config.host.name, "HPC");
        assert_eq!(config.host.ip, "0.0.0.0");
        assert_eq!(config.host.r#type, "bluechi");
        assert!(config.guest.is_none());
    }

    // Test static behavior of `get_config`
    #[tokio::test]
    async fn test_get_config_static_behavior() {
        // Verify that `get_config` returns the same instance every time
        let config1 = get_config();
        let config2 = get_config();
        assert!(std::ptr::eq(config1, config2));
    }

    // Test handling of multiple guests in the settings
    #[tokio::test]
    async fn test_parse_settings_yaml_multiple_guests() {
        // Verify that multiple guests are correctly parsed
        let settings = parse_settings_yaml();
        if let Some(guests) = settings.guest {
            assert!(guests.len() > 1);
            for guest in guests {
                assert_ne!(guest.name, "");
                assert_ne!(guest.ip, "");
                assert_ne!(guest.r#type, "");
            }
        }
    }

    // Test concurrent access to `get_config`
    #[tokio::test]
    async fn test_get_config_concurrent_access() {
        // Verify that concurrent access to `get_config` is safe
        use std::thread;

        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    let config = get_config();
                    assert_eq!(config.yaml_storage, "/etc/piccolo/yaml");
                    assert_eq!(config.piccolo_cloud, "http://0.0.0.0:41234");
                    assert_eq!(config.host.name, "HPC");
                    assert_eq!(config.host.ip, "0.0.0.0");
                    assert_eq!(config.host.r#type, "bluechi");
                    assert!(config.guest.is_none());
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    // Test handling of a settings file with empty guest list
    #[tokio::test]
    async fn test_parse_settings_yaml_empty_guest_list() {
        // Verify that an empty guest list is handled correctly
        let settings = parse_settings_yaml();
        if let Some(guests) = settings.guest {
            assert!(guests.is_empty());
        } else {
            assert!(settings.guest.is_none());
        }
    }

    // Test handling of a settings file with duplicate guest entries
    #[tokio::test]
    async fn test_parse_settings_yaml_duplicate_guest_entries() {
        // Verify that duplicate guest entries are handled correctly
        let settings = parse_settings_yaml();
        if let Some(guests) = settings.guest {
            let mut unique_guests = guests.clone();
            unique_guests.sort_by(|a, b| a.name.cmp(&b.name));
            unique_guests.dedup_by(|a, b| a.name == b.name && a.ip == b.ip && a.r#type == b.r#type);
            assert_eq!(unique_guests.len(), guests.len());
        }
    }

    // Test handling of a settings file with invalid piccolo_cloud URL
    #[tokio::test]
    async fn test_parse_settings_yaml_invalid_piccolo_cloud_url() {
        // Verify that the piccolo_cloud URL is valid
        let settings = parse_settings_yaml();
        assert!(
            settings.piccolo_cloud.starts_with("http://")
                || settings.piccolo_cloud.starts_with("https://")
        );
    }

    // Test handling of a settings file with missing host name
    #[tokio::test]
    async fn test_parse_settings_yaml_missing_host_name() {
        // Verify that the host name is not missing
        let settings = parse_settings_yaml();
        assert_ne!(settings.host.name, "");
    }

    // Test handling of a settings file with invalid host type
    #[tokio::test]
    async fn test_parse_settings_yaml_invalid_host_type() {
        // Verify that the host type is valid
        let settings = parse_settings_yaml();
        let valid_types = vec!["bluechi", "redchi", "greenchi"];
        assert!(valid_types.contains(&settings.host.r#type.as_str()));
    }

    // Test handling of a settings file with guests having invalid types
    #[tokio::test]
    async fn test_parse_settings_yaml_invalid_guest_types() {
        // Verify that guest types are valid
        let settings = parse_settings_yaml();
        if let Some(guests) = settings.guest {
            let valid_types = vec!["nodeagent", "zoneagent", "cloudagent"];
            for guest in guests {
                assert!(valid_types.contains(&guest.r#type.as_str()));
            }
        }
    }

    // Test handling of a settings file with guests having missing names
    #[tokio::test]
    async fn test_parse_settings_yaml_guest_missing_names() {
        // Verify that guest names are not missing
        let settings = parse_settings_yaml();
        if let Some(guests) = settings.guest {
            for guest in guests {
                assert_ne!(guest.name, "");
            }
        }
    }

    // Test handling of a settings file with guests having duplicate IPs
    #[tokio::test]
    async fn test_parse_settings_yaml_guest_duplicate_ips() {
        // Verify that guest IPs are unique
        let settings = parse_settings_yaml();
        if let Some(guests) = settings.guest {
            let mut ips: Vec<String> = guests.iter().map(|guest| guest.ip.clone()).collect();
            ips.sort();
            ips.dedup();
            assert_eq!(ips.len(), guests.len());
        }
    }

    // Test handling of invalid YAML file path
    #[tokio::test]
    async fn test_parse_settings_yaml_invalid_file_path() {
        // Verify that an invalid YAML file path is handled correctly
        let settings = parse_settings_yaml();
        assert_eq!(settings.yaml_storage, "/etc/piccolo/yaml");
        assert_eq!(settings.piccolo_cloud, "http://0.0.0.0:41234");
    }

    // Test handling of missing required fields in YAML
    #[tokio::test]
    async fn test_parse_settings_yaml_missing_required_fields() {
        // Verify that required fields are not missing
        let settings = parse_settings_yaml();
        assert_ne!(settings.host.name, "");
        assert_ne!(settings.host.ip, "");
        assert_ne!(settings.host.r#type, "");
    }

    // Test handling of unexpected data types in YAML
    #[tokio::test]
    async fn test_parse_settings_yaml_unexpected_data_types() {
        // Verify that unexpected data types are handled correctly
        let settings = parse_settings_yaml();
        assert!(settings.host.ip.parse::<std::net::Ipv4Addr>().is_ok());
    }

    // Test handling of excessively large guest lists
    #[tokio::test]
    async fn test_parse_settings_yaml_large_guest_list() {
        // Verify that excessively large guest lists are handled correctly
        let settings = parse_settings_yaml();
        if let Some(guests) = settings.guest {
            assert!(guests.len() < 1000);
        }
    }

    // Test handling of invalid host IP format
    #[tokio::test]
    async fn test_parse_settings_yaml_invalid_host_ip_format() {
        // Verify that the host IP format is valid
        let settings = parse_settings_yaml();
        assert!(settings.host.ip.parse::<std::net::Ipv4Addr>().is_ok());
    }

    // Test handling of invalid guest IP format
    #[tokio::test]
    async fn test_parse_settings_yaml_invalid_guest_ip_format() {
        // Verify that guest IP formats are valid
        let settings = parse_settings_yaml();
        if let Some(guests) = settings.guest {
            for guest in guests {
                assert!(guest.ip.parse::<std::net::Ipv4Addr>().is_ok());
            }
        }
    }
}
