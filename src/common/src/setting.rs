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

#[derive(Deserialize)]
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
        /*guest: Some(vec![GuestSettings {
            name: String::from("ZONE"),
            ip: String::from("192.168.0.1"),
            r#type: String::from("nodeagent"),
        }]),*/
        guest: None,
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
