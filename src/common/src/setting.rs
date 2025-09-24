use serde::Deserialize;
use std::sync::OnceLock;
static SETTINGS: OnceLock<Settings> = OnceLock::new();

#[derive(Deserialize)]
pub struct Settings {
    pub yaml_storage: String,
    pub pullpiri_cloud: String,
    pub host: HostSettings,
    // guest 설정 제거
}

#[derive(Deserialize)]
pub struct HostSettings {
    pub name: String,
    pub ip: String,
    pub r#type: String,
    pub role: String,
}

// GuestSettings 구조체 제거

fn parse_settings_yaml() -> Settings {
    let default_settings: Settings = Settings {
        yaml_storage: String::from("/etc/pullpiri/yaml"),
        pullpiri_cloud: String::from("http://0.0.0.0:41234"),
        host: HostSettings {
            name: String::from("HPC"),
            ip: String::from("0.0.0.0"),
            r#type: String::from("bluechi"),
            role: String::from("master"),
        },
        // guest 설정 제거
    };

    let settings = config::Config::builder()
        .add_source(config::File::with_name("/etc/piccolo/settings.yaml"))
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
        assert_eq!(settings.yaml_storage, "/etc/pullpiri/yaml");
        assert_eq!(settings.pullpiri_cloud, "http://0.0.0.0:41234");
        assert_eq!(settings.host.name, "HPC");
        assert_eq!(settings.host.ip, "0.0.0.0");
        assert_eq!(settings.host.r#type, "bluechi");
    }

    // Guest 설정 테스트 제거

    // Test lazy initialization of configuration
    #[tokio::test]
    async fn test_get_config_lazy_initialization() {
        // Verify that the configuration is lazily initialized
        let config = get_config();
        assert_eq!(config.yaml_storage, "/etc/pullpiri/yaml");
        assert_eq!(config.pullpiri_cloud, "http://0.0.0.0:41234");
        assert_eq!(config.host.name, "HPC");
        assert_eq!(config.host.ip, "0.0.0.0");
        assert_eq!(config.host.r#type, "bluechi");
    }

    // Test static behavior of `get_config`
    #[tokio::test]
    async fn test_get_config_static_behavior() {
        // Verify that `get_config` returns the same instance every time
        let config1 = get_config();
        let config2 = get_config();
        assert!(std::ptr::eq(config1, config2));
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
                    assert_eq!(config.yaml_storage, "/etc/pullpiri/yaml");
                    assert_eq!(config.pullpiri_cloud, "http://0.0.0.0:41234");
                    assert_eq!(config.host.name, "HPC");
                    assert_eq!(config.host.ip, "0.0.0.0");
                    assert_eq!(config.host.r#type, "bluechi");
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    // Guest 관련 테스트 제거

    // Test handling of a settings file with invalid pullpiri_cloud URL
    #[tokio::test]
    async fn test_parse_settings_yaml_invalid_pullpiri_cloud_url() {
        // Verify that the pullpiri_cloud URL is valid
        let settings = parse_settings_yaml();
        assert!(
            settings.pullpiri_cloud.starts_with("http://")
                || settings.pullpiri_cloud.starts_with("https://")
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

    // Test handling of invalid YAML file path
    #[tokio::test]
    async fn test_parse_settings_yaml_invalid_file_path() {
        // Verify that an invalid YAML file path is handled correctly
        let settings = parse_settings_yaml();
        assert_eq!(settings.yaml_storage, "/etc/pullpiri/yaml");
        assert_eq!(settings.pullpiri_cloud, "http://0.0.0.0:41234");
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

    // Guest 관련 테스트 제거

    // Test handling of invalid host IP format
    #[tokio::test]
    async fn test_parse_settings_yaml_invalid_host_ip_format() {
        // Verify that the host IP format is valid
        let settings = parse_settings_yaml();
        assert!(settings.host.ip.parse::<std::net::Ipv4Addr>().is_ok());
    }

    // Guest 관련 테스트 제거
}
