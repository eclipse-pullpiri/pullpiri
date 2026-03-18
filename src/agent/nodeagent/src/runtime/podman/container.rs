/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

use super::{get, post};
use hyper::Body;
use serde_json::json;

//const PODMAN_API_VERSION: &str = "/v4.0.0/libpod";
const PODMAN_API_VERSION: &str = "/v4.0.0";

/// Parse Pod YAML and extract pod name and spec
fn parse_pod(pod_yaml: &str) -> Result<(String, serde_json::Value), Box<dyn std::error::Error>> {
    let pod = serde_yaml::from_str::<common::spec::k8s::Pod>(pod_yaml)?;
    let pod_name = pod.get_name();
    let pod_json = serde_json::to_value(&pod)?;
    let spec = pod_json["spec"].clone();
    Ok((pod_name, spec))
}

/// Get container names from pod spec
fn get_container_names(
    pod_name: &str,
    spec: &serde_json::Value,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let containers = spec["containers"]
        .as_array()
        .ok_or("No containers found in spec")?;

    containers
        .iter()
        .map(|container| {
            let container_name = container["name"]
                .as_str()
                .ok_or("Container name field not found")?;
            Ok(format!("{}_{}", pod_name, container_name))
        })
        .collect()
}

/// Build HostConfig for container creation
fn build_host_config(
    container: &serde_json::Value,
    spec: &serde_json::Value,
    host_network: bool,
) -> serde_json::Value {
    let mut host_config = serde_json::Map::new();

    // Add hostNetwork setting
    if host_network {
        host_config.insert("NetworkMode".to_string(), json!("host"));
    }

    // CapAdd
    if let Some(cap_add) = container["securityContext"]
        .get("capabilities")
        .and_then(|c| c.get("add"))
        .and_then(|a| a.as_array())
    {
        let caps: Vec<String> = cap_add
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !caps.is_empty() {
            host_config.insert("CapAdd".to_string(), json!(caps));
        }
    }

    // Privileged
    if let Some(privileged) = container["securityContext"]
        .get("privileged")
        .and_then(|p| p.as_bool())
    {
        host_config.insert("Privileged".to_string(), json!(privileged));
    }

    // NanoCpus (CPU 제한)
    if let Some(cpu) = container["resources"]
        .get("limits")
        .and_then(|l| l.get("cpu"))
        .and_then(|c| c.as_str())
    {
        if let Ok(cpu_num) = cpu.parse::<f64>() {
            let nano_cpus = (cpu_num * 1_000_000_000.0) as i64;
            host_config.insert("NanoCpus".to_string(), json!(nano_cpus));
        }
    }

    // Mounts (volumeMounts → Mounts array for Podman API v4.0 compat)
    if let Some(volume_mounts) = container["volumeMounts"].as_array() {
        if let Some(volumes) = spec["volumes"].as_array() {
            let mut mounts: Vec<serde_json::Value> = Vec::new();
            for mount in volume_mounts {
                let mount_name = mount["name"].as_str().unwrap_or("");
                let mount_path = mount["mountPath"].as_str().unwrap_or("");
                for volume in volumes {
                    if volume["name"].as_str() == Some(mount_name) {
                        // hostPath가 있으면 hostPath를 Source로, 없으면 mountPath를 Source로 사용
                        let source = if let Some(host_path) = volume["hostPath"]["path"].as_str() {
                            host_path
                        } else {
                            // hostPath 정보가 없으면 mountPath를 그대로 사용
                            mount_path
                        };
                        mounts.push(json!({
                            "Type": "bind",
                            "Source": source,
                            "Target": mount_path
                        }));
                        break;
                    }
                }
            }
            if !mounts.is_empty() {
                host_config.insert("Mounts".to_string(), json!(mounts));
            }
        }
    }

    json!(host_config)
}

/// Get network mode for container creation (Docker-compatible API)
/// Returns the first network name from spec, or None if not specified
fn get_network_mode(spec: &serde_json::Value) -> Option<String> {
    spec["networks"]
        .as_array()
        .and_then(|networks| networks.first())
        .and_then(|network| network["name"].as_str())
        .map(|s| s.to_string())
}

/// Build environment variables array
fn build_env_vars(container: &serde_json::Value) -> Vec<String> {
    container["env"]
        .as_array()
        .map(|env| {
            env.iter()
                .filter_map(|e| {
                    let name = e["name"].as_str()?;
                    let value = e["value"].as_str()?;
                    Some(format!("{}={}", name, value))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Build command array
fn build_command(container: &serde_json::Value) -> Vec<String> {
    container["command"]
        .as_array()
        .map(|command| {
            command
                .iter()
                .filter_map(|c| c.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Create container from spec
async fn create_container(
    pod_name: &str,
    container: &serde_json::Value,
    spec: &serde_json::Value,
    host_network: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let image = container["image"]
        .as_str()
        .ok_or("Container image field not found")?;
    let container_name = container["name"]
        .as_str()
        .ok_or("Container name field not found")?;

    // Check if image exists, pull if not
    if !image_exists(image).await? {
        println!("Image {} not found locally, pulling...", image);
        pull_image(image).await?;
        println!("Image {} pulled successfully", image);
    }

    let name = format!("{}_{}", pod_name, container_name);

    // Build container creation request
    let mut create_body = json!({
        "Image": image,
        "Name": name,
        "Tty": true,
        "OpenStdin": true
    });

    // Add HostConfig
    let mut host_config = build_host_config(container, spec, host_network);
    
    // Add NetworkMode to HostConfig (Docker-compatible API)
    if !host_network {
        if let Some(network_mode) = get_network_mode(spec) {
            host_config
                .as_object_mut()
                .unwrap()
                .insert("NetworkMode".to_string(), json!(network_mode));
        }
    }
    
    if !host_config.as_object().unwrap().is_empty() {
        create_body["HostConfig"] = host_config;
    }

    // Add environment variables
    let env_vars = build_env_vars(container);
    if !env_vars.is_empty() {
        create_body["Env"] = json!(env_vars);
    }

    // Add command
    let cmd = build_command(container);
    if !cmd.is_empty() {
        create_body["Cmd"] = json!(cmd);
    }

    println!("{}", create_body.to_string());
    // Create the container
    println!("Creating container from image: {}", image);
    let create_path = format!("{}/containers/create?name={}", PODMAN_API_VERSION, name);
    let create_response = post(&create_path, Body::from(create_body.to_string())).await?;

    let create_result: serde_json::Value = serde_json::from_slice(&create_response)?;
    let container_id = create_result["Id"]
        .as_str()
        .ok_or("Failed to get container ID")?
        .to_string();

    Ok(container_id)
}

pub async fn start(pod_yaml: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let (pod_name, spec) = parse_pod(pod_yaml)?;
    let host_network = spec["hostNetwork"].as_bool().unwrap_or(false);

    let mut container_ids = Vec::new();

    if let Some(containers) = spec["containers"].as_array() {
        for container in containers.iter() {
            let container_id = create_container(&pod_name, container, &spec, host_network).await?;

            // Start the container
            println!("Starting container: {}", container_id);
            let start_path = format!("{}/containers/{}/start", PODMAN_API_VERSION, container_id);
            post(&start_path, Body::empty()).await?;

            println!("Container {} started successfully", container_id);
            container_ids.push(container_id);
        }
    }

    Ok(container_ids)
}

pub async fn stop(pod_yaml: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (pod_name, spec) = parse_pod(pod_yaml)?;
    let container_names = get_container_names(&pod_name, &spec)?;

    for full_container_name in container_names {
        // Stop the container
        println!("Stopping container: {}", full_container_name);
        let stop_path = format!(
            "{}/containers/{}/stop",
            PODMAN_API_VERSION, full_container_name
        );
        match post(&stop_path, Body::empty()).await {
            Ok(_) => println!("Container {} stopped successfully", full_container_name),
            Err(e) => println!(
                "Warning: Failed to stop container {}: {}",
                full_container_name, e
            ),
        }

        // Remove the container
        println!("Removing container: {}", full_container_name);
        let remove_path = format!(
            "{}/containers/{}?force=true",
            PODMAN_API_VERSION, full_container_name
        );
        match super::delete(&remove_path).await {
            Ok(_) => println!("Container {} removed successfully", full_container_name),
            Err(e) => println!(
                "Warning: Failed to remove container {}: {}",
                full_container_name, e
            ),
        }
    }

    Ok(())
}

pub async fn restart(pod_yaml: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (pod_name, spec) = parse_pod(pod_yaml)?;
    let container_names = get_container_names(&pod_name, &spec)?;

    for full_container_name in container_names {
        // Use Podman's restart API endpoint
        println!("Restarting container: {}", full_container_name);
        let restart_path = format!(
            "{}/containers/{}/restart",
            PODMAN_API_VERSION, full_container_name
        );
        match post(&restart_path, Body::empty()).await {
            Ok(_) => println!("Container {} restarted successfully", full_container_name),
            Err(e) => {
                println!(
                    "Warning: Failed to restart container {}: {}",
                    full_container_name, e
                );
                println!("Attempting full stop/start cycle...");
                // Fallback: if restart fails, try stop and start
                stop(pod_yaml).await?;
                start(pod_yaml).await?;
                return Ok(());
            }
        }
    }

    Ok(())
}

/// Check if an image exists locally
pub async fn image_exists(image_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let path = "/v4.0.0/libpod/images/json";

    let result = get(path).await?;
    let images: Vec<serde_json::Value> = serde_json::from_slice(&result)?;
    for image in images {
        if let Some(repo_tags) = image["RepoTags"].as_array() {
            for tag in repo_tags {
                if tag.as_str() == Some(image_name) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// Pull an image from a registry
pub async fn pull_image(image_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = format!("/v4.0.0/libpod/images/pull?reference={}", image_name);
    post(&path, Body::empty()).await?;
    Ok(())
}
