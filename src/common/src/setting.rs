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
}

#[derive(Deserialize)]
pub struct GuestSettings {
    pub name: String,
    pub ip: String,
}

fn parse_settings_yaml() -> Settings {
    let default_settings: Settings = Settings {
        yaml_storage: String::from("/etc/piccolo"),
        piccolo_cloud: String::from("http://0.0.0.0:41234"),
        host: HostSettings {
            name: String::from("HPC"),
            ip: String::from("0.0.0.0"),
        },
        /*guest: Some(vec![GuestSettings {
            name: String::from("ZONE"),
            ip: String::from("192.168.0.1"),
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
