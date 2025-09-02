pub mod container;
pub mod nodeinfo;

use hyper::{Body, Client, Uri};
use hyperlocal::{UnixConnector, Uri as UnixUri};
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

async fn get(path: &str) -> Result<hyper::body::Bytes, hyper::Error> {
    let connector = UnixConnector;
    let client = Client::builder().build::<_, Body>(connector);

    // Modify this if you want to run without root authorization
    // or if you have a different socket path.
    // For example, if you run Podman as root, you might use:
    // let socket = "/var/run/podman/podman.sock";
    // Or if you run it as a user, you might use:
    // let socket = "/run/user/1000/podman/podman.sock
    let socket = "/var/run/podman/podman.sock";
    // let socket = "/var/run/podman/podman.sock";
    let uri: Uri = UnixUri::new(socket, path).into();

    let res = client.get(uri).await?;
    hyper::body::to_bytes(res).await
}

/// Node information matching the requested DataCache structure.
#[derive(Deserialize, Debug)]
pub struct NodeInfo {
    // 1. CPU
    pub cpu_count: usize, // NodeInfo['cpu']['cpu_count']
    pub cpu_usage: f32,   // NodeInfo['cpu']['cpu_usage']

    // 2. GPU
    pub gpu_count: usize, // NodeInfo['gpu']['gpu_count']

    // 3. Memory
    pub total_memory: u64, // NodeInfo['mem']['total_memory']
    pub used_memory: u64,  // NodeInfo['mem']['used_memory']
    pub mem_usage: f32,    // NodeInfo['mem']['mem_usage']

    // 4. Network
    pub rx_bytes: u64, // NodeInfo['net']['rx_bytes']
    pub tx_bytes: u64, // NodeInfo['net']['tx_bytes']

    // 5. Storage
    pub read_bytes: u64,  // NodeInfo['storage']['read_bytes']
    pub write_bytes: u64, // NodeInfo['storage']['write_bytes']

    // 6. System
    pub os: String,   // NodeInfo['system']['os']
    pub arch: String, // NodeInfo['system']['arch']
    pub ip: String,   // NodeInfo['system']['ip']
}

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Podman API error: {0}")]
    PodmanApi(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Env error: {0}")]
    Env(#[from] std::env::VarError),
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct Container {
    pub Id: String,
    pub Names: Vec<String>,
    pub Image: String,
    pub State: String,
    pub Status: String,
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerInspect {
    pub Id: String,
    pub Name: String,
    pub State: ContainerState,
    pub Config: ContainerConfig,
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerState {
    pub Status: String,
    pub Running: bool,
    pub Paused: bool,
    pub Restarting: bool,
    pub OOMKilled: bool,
    pub Dead: bool,
    pub Pid: i32,
    pub ExitCode: i32,
    pub Error: String,
    pub StartedAt: String,
    pub FinishedAt: String,
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerConfig {
    pub Hostname: String,
    pub Domainname: String,
    pub User: String,
    pub AttachStdin: bool,
    pub AttachStdout: bool,
    pub AttachStderr: bool,
    pub ExposedPorts: Option<HashMap<String, serde_json::Value>>,
    pub Tty: bool,
    pub OpenStdin: bool,
    pub StdinOnce: bool,
    pub Env: Option<Vec<String>>,
    pub Cmd: Option<Vec<String>>,
    pub Image: String,
    pub Volumes: Option<HashMap<String, serde_json::Value>>,
    pub WorkingDir: String,
    pub Entrypoint: String,
    pub OnBuild: Option<Vec<String>>,
    pub Labels: Option<HashMap<String, String>>,
    pub Annotations: Option<HashMap<String, String>>,
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerStats {
    pub Id: String,
    pub name: String,
    pub cpu_stats: ContainerCpuStats,
    pub memory_stats: ContainerMemoryStats,
    pub networks: Option<HashMap<String, ContainerNetworkStats>>,
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerCpuStats {
    pub cpu_usage: ContainerCpuUsage,
    pub online_cpus: u64,
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerCpuUsage {
    pub total_usage: u64,
    pub usage_in_kernelmode: u64,
    pub usage_in_usermode: u64,
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerMemoryStats {
    pub usage: u64,
    pub limit: u64,
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerNetworkStats {
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errors: u64,
    pub rx_dropped: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errors: u64,
    pub tx_dropped: u64,
}

use std::fmt;

impl fmt::Display for ContainerNetworkStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "rx_bytes: {}, rx_packets: {}, rx_errors: {}, rx_dropped: {}, tx_bytes: {}, tx_packets: {}, tx_errors: {}, tx_dropped: {}",
            self.rx_bytes,
            self.rx_packets,
            self.rx_errors,
            self.rx_dropped,
            self.tx_bytes,
            self.tx_packets,
            self.tx_errors,
            self.tx_dropped
        )
    }
}

//Unit tets cases
#[cfg(test)]
mod tests {
    use super::get;
    use hyper::body::Bytes;
    use hyper::Error;
    use tokio;

    #[tokio::test]
    async fn test_get_with_valid_path() {
        let result: Result<Bytes, Error> = get("/v1.0/version").await;
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }
}
