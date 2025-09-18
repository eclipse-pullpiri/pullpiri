use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;
use thiserror::Error;

// Global config instance
static NODEAGENT_CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse YAML: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MetricsConfig {
    pub collection_interval: u64,
    pub batch_size: u32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct EtcdConfig {
    pub endpoint: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SystemConfig {
    pub hostname: String,
    pub platform: String,
    pub architecture: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct NodeAgentConfig {
    pub node_type: String,
    pub master_ip: String,
    pub grpc_port: u16,
    pub log_level: String,
    pub metrics: MetricsConfig,
    pub etcd: EtcdConfig,
    pub system: SystemConfig,
    #[serde(default = "default_yaml_storage")]
    pub yaml_storage: String,
}

fn default_yaml_storage() -> String {
    "/etc/piccolo/yaml".to_string()
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    pub nodeagent: NodeAgentConfig,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    pub fn get_host_ip(&self) -> String {
        self.nodeagent.master_ip.clone()
    }

    pub fn get_hostname(&self) -> String {
        self.nodeagent.system.hostname.clone()
    }

    pub fn get_yaml_storage(&self) -> String {
        self.nodeagent.yaml_storage.clone()
    }

    // Get or initialize the global config
    pub fn get() -> &'static Config {
        NODEAGENT_CONFIG.get().unwrap_or_else(|| {
            let default_config = Config::default();
            NODEAGENT_CONFIG.set(default_config.clone()).unwrap_or(());
            NODEAGENT_CONFIG.get().unwrap()
        })
    }

    // Set the global config
    pub fn set_global(config: Config) {
        let _ = NODEAGENT_CONFIG.set(config);
    }
}
