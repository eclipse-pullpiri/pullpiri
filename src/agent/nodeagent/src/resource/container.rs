use common::monitoringserver::ContainerInfo;
use futures::future::try_join_all;
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Podman API error: {0}")]
    PodmanApi(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("Serde error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Env error: {0}")]
    Env(#[from] std::env::VarError),
}

pub async fn inspect(hostname: String) -> std::result::Result<Vec<ContainerInfo>, ContainerError> {
    let list = get_list().await?;
    let infos: Vec<ContainerInfo> = try_join_all(list.iter().map(|container| {
        let id = container.Id.clone();
        let host_name = hostname.clone();
        async move {
            let inspect = get_inspect(&id).await?;
            let stats = get_stats(&id).await?;

            let mut state_map = HashMap::new();
            state_map.insert("Status".to_string(), inspect.State.Status);
            state_map.insert("Running".to_string(), inspect.State.Running.to_string());
            state_map.insert("Paused".to_string(), inspect.State.Paused.to_string());
            state_map.insert(
                "Restarting".to_string(),
                inspect.State.Restarting.to_string(),
            );
            state_map.insert("OOMKilled".to_string(), inspect.State.OOMKilled.to_string());
            state_map.insert("Dead".to_string(), inspect.State.Dead.to_string());
            state_map.insert("Pid".to_string(), inspect.State.Pid.to_string());
            state_map.insert("ExitCode".to_string(), inspect.State.ExitCode.to_string());
            state_map.insert("Error".to_string(), inspect.State.Error);
            state_map.insert("StartedAt".to_string(), inspect.State.StartedAt);
            state_map.insert("FinishedAt".to_string(), inspect.State.FinishedAt);

            let mut config_map = HashMap::new();
            config_map.insert("Hostname".to_string(), host_name);
            config_map.insert("Domainname".to_string(), inspect.Config.Domainname);
            config_map.insert("User".to_string(), inspect.Config.User);
            config_map.insert(
                "AttachStdin".to_string(),
                inspect.Config.AttachStdin.to_string(),
            );
            config_map.insert(
                "AttachStdout".to_string(),
                inspect.Config.AttachStdout.to_string(),
            );
            config_map.insert(
                "AttachStderr".to_string(),
                inspect.Config.AttachStderr.to_string(),
            );
            config_map.insert("Tty".to_string(), inspect.Config.Tty.to_string());
            config_map.insert(
                "OpenStdin".to_string(),
                inspect.Config.OpenStdin.to_string(),
            );
            config_map.insert(
                "StdinOnce".to_string(),
                inspect.Config.StdinOnce.to_string(),
            );
            config_map.insert("Image".to_string(), inspect.Config.Image.clone());
            config_map.insert("WorkingDir".to_string(), inspect.Config.WorkingDir);

            let annotation_map = if let Some(ann_map) = inspect.Config.Annotations {
                ann_map.clone()
            } else {
                HashMap::new()
            };

            let mut stats_map = HashMap::new();
            stats_map.insert(
                "CpuTotalUsage".to_string(),
                stats.cpu_stats.cpu_usage.total_usage.to_string(),
            );
            stats_map.insert(
                "CpuUsageInKernelMode".to_string(),
                stats.cpu_stats.cpu_usage.usage_in_kernelmode.to_string(),
            );
            stats_map.insert(
                "CpuUsageInUserMode".to_string(),
                stats.cpu_stats.cpu_usage.usage_in_usermode.to_string(),
            );
            stats_map.insert(
                "MemoryUsage".to_string(),
                stats.memory_stats.usage.to_string(),
            );
            stats_map.insert(
                "MemoryLimit".to_string(),
                stats.memory_stats.limit.to_string(),
            );

            // calculate total network inbound
            let total_rx_bytes: u64 = stats
                .networks
                .as_ref()
                .map(|nets| nets.values().map(|net| net.rx_bytes).sum())
                .unwrap_or(0);
            stats_map.insert(
                "TotalNetworkRxBytes".to_string(),
                total_rx_bytes.to_string(),
            );

            // calculate total network outbound
            let total_tx_bytes: u64 = stats
                .networks
                .as_ref()
                .map(|nets| nets.values().map(|net| net.tx_bytes).sum())
                .unwrap_or(0);
            stats_map.insert(
                "TotalNetworkTxBytes".to_string(),
                total_tx_bytes.to_string(),
            );

            // calculate total blkio read bytes
            let total_blkio_read: u64 = stats
                .blkio_stats
                .io_service_bytes_recursive
                .as_ref()
                .map(|vec| {
                    vec.iter()
                        .filter(|blkio| blkio.op == "Read")
                        .map(|blkio| blkio.value)
                        .sum()
                })
                .unwrap_or(0);
            stats_map.insert(
                "TotalBlkioReadBytes".to_string(),
                total_blkio_read.to_string(),
            );

            // calculate total blkio write bytes
            let total_blkio_write: u64 = stats
                .blkio_stats
                .io_service_bytes_recursive
                .as_ref()
                .map(|vec| {
                    vec.iter()
                        .filter(|blkio| blkio.op == "Write")
                        .map(|blkio| blkio.value)
                        .sum()
                })
                .unwrap_or(0);
            stats_map.insert(
                "TotalBlkioWriteBytes".to_string(),
                total_blkio_write.to_string(),
            );

            Ok::<ContainerInfo, ContainerError>(ContainerInfo {
                id: inspect.Id,
                names: vec![inspect.Name],
                image: inspect.Config.Image.clone(),
                state: state_map,
                config: config_map,
                annotation: annotation_map,
                stats: stats_map,
            })
        }
    }))
    .await
    .map_err(|e| ContainerError::PodmanApi(Box::new(e)))?
    .into_iter()
    .collect();

    Ok(infos)
}

pub async fn get_list() -> Result<Vec<Container>> {
    let body = super::get("/v1.0.0/libpod/containers/json").await?;

    let containers: Vec<Container> = serde_json::from_slice(&body)?;
    //println!("{:#?}", containers);

    Ok(containers)
}

pub async fn get_inspect(
    id: &str,
) -> std::result::Result<ContainerInspect, Box<dyn std::error::Error + Send + Sync>> {
    let path = &format!("/v1.0.0/libpod/containers/{}/json", id);
    let body = super::get(path).await?;

    let inspect: ContainerInspect = serde_json::from_slice(&body)?;
    //println!("{:#?}", container_inspect);

    Ok(inspect)
}

pub async fn get_stats(
    id: &str,
) -> std::result::Result<ContainerStats, Box<dyn std::error::Error + Send + Sync>> {
    let path = &format!("/v1.0.0/libpod/containers/{}/stats?stream=false", id);
    let body = super::get(path).await?;

    let stats: ContainerStats = serde_json::from_slice(&body)?;
    // println!("{:#?}", stats);

    Ok(stats)
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
    pub blkio_stats: ContainerBlkioStats,
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

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerBlkioStats {
    pub io_service_bytes_recursive: Option<Vec<ContainerBlkioServiceBytesRecursive>>,
}

#[allow(non_snake_case, unused)]
#[derive(Deserialize, Debug)]
pub struct ContainerBlkioServiceBytesRecursive {
    pub major: u64,
    pub minor: u64,
    pub op: String,
    pub value: u64,
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::{get_inspect, get_list, inspect, Container, ContainerError, ContainerInspect};
    use common::monitoringserver::ContainerInfo;
    use std::collections::HashMap;
    use tokio;

    #[tokio::test]
    async fn test_get_list_success() {
        let result = get_list().await;
        assert!(result.is_ok());
        let containers = result.unwrap();
        for container in containers {
            assert!(!container.Id.is_empty());
            assert!(!container.Image.is_empty());
            assert!(!container.State.is_empty());
            // There's a case that the Status is empty.
            // assert!(!container.Status.is_empty());
        }
    }

    #[tokio::test]
    async fn test_get_inspect_success() {
        let list = get_list().await.unwrap();
        if let Some(container) = list.first() {
            let result = get_inspect(&container.Id).await;
            assert!(result.is_ok());
            let inspect = result.unwrap();
            assert_eq!(inspect.Id, container.Id);
            assert!(!inspect.Name.is_empty());
            assert!(!inspect.Config.Image.is_empty());
            assert!(!inspect.State.Status.is_empty());
        }
    }

    #[tokio::test]
    async fn test_get_inspect_invalid_id() {
        let invalid_id = "nonexistent_container_id_12345";
        let result = get_inspect(invalid_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_inspect_contains_expected_keys() {
        let hostname: String = String::from_utf8_lossy(
            &std::process::Command::new("hostname")
                .output()
                .expect("Failed to get hostname")
                .stdout,
        )
        .trim()
        .to_string();

        let result = inspect(hostname).await;
        assert!(result.is_ok());
        let infos = result.unwrap();
        for info in infos {
            assert!(!info.id.is_empty());
            assert!(!info.names.is_empty());
            assert!(!info.image.is_empty());
            assert!(info.state.contains_key("Status"));
            assert!(info.state.contains_key("Running"));
            assert!(info.config.contains_key("Hostname"));
        }
    }
}
