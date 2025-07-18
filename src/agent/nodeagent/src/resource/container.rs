use common::monitoringserver::ContainerInfo;
use futures::future::try_join_all;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
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

pub async fn inspect() -> std::result::Result<Vec<ContainerInfo>, ContainerError> {
    let list = get_list().await?;
    let infos: Vec<ContainerInfo> = try_join_all(list.iter().map(|container| {
        let id = container.Id.clone();
        async move {
            let inspect = get_inspect(&id).await?;
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
            let host_name = env::var("HOST_NAME").unwrap_or("Unknown".to_string());
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

            Ok::<ContainerInfo, ContainerError>(ContainerInfo {
                id: inspect.Id,
                names: vec![inspect.Name],
                image: inspect.Config.Image.clone(),
                state: state_map,
                config: config_map,
                annotation: annotation_map,
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
