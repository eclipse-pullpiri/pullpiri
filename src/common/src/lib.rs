/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

pub mod apiserver;
pub mod error;
pub mod etcd;
pub mod filtergateway;
pub mod spec;
pub mod statemanager;

pub use crate::error::Result;

use std::sync::OnceLock;
static SETTINGS: OnceLock<Settings> = OnceLock::new();

#[derive(serde::Deserialize)]
pub struct Settings {
    pub yaml_storage: String,
    pub doc_registry: String,
    pub host: HostSettings,
    pub guest: Option<Vec<GuestSettings>>,
}

#[derive(serde::Deserialize)]
pub struct HostSettings {
    pub name: String,
    pub ip: String,
}

#[derive(serde::Deserialize)]
pub struct GuestSettings {
    pub name: String,
    pub ip: String,
    pub ssh_port: String,
    pub id: String,
    pub pw: String,
}

fn parse_settings_yaml() -> Settings {
    let s: Settings = Settings {
        yaml_storage: String::from("/root/piccolo_yaml"),
        doc_registry: String::from("http://0.0.0.0:41234"),
        host: HostSettings {
            name: String::from("HPC"),
            ip: String::from("0.0.0.0"),
        },
        /*guest: Some(vec![GuestSettings {
            name: String::from("ZONE"),
            ip: String::from("192.168.10.239"),
            ssh_port: String::from("22"),
            id: String::from("root"),
            pw: String::from("lge123"),
        }]),*/
        guest: None,
    };

    let settings = config::Config::builder()
        .add_source(config::File::with_name("/piccolo/settings.yaml"))
        .build();

    match settings {
        Ok(result) => result.try_deserialize::<Settings>().unwrap_or(s),
        Err(_) => s,
    }
}

pub fn get_config() -> &'static Settings {
    SETTINGS.get_or_init(parse_settings_yaml)
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;
    static CONFIG: OnceLock<config::Config> = OnceLock::new();

    fn init_conf() -> config::Config {
        config::Config::builder()
            .add_source(config::File::with_name("piccolo"))
            .build()
            .unwrap_or(
                config::Config::builder()
                    .set_default("HOST_IP", "0.0.0.0")
                    .unwrap()
                    .set_default("HOST_NODE", "HPC")
                    .unwrap()
                    .build()
                    .unwrap(),
            )
    }

    #[test]
    pub fn get_conf() {
        let conf = CONFIG.get_or_init(init_conf);
        assert_eq!(conf.get_string("HOST_IP").unwrap(), "0.0.0.0");
        assert_eq!(conf.get_string("HOST_NODE").unwrap(), "HPC");
    }
}
