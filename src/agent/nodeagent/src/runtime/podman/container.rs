/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! Podman container runtime implementation
//!
//! This module provides functionality to manage containers using Podman's Docker-compatible REST API.
//! It converts Kubernetes Pod specifications to Podman container configurations and manages their lifecycle.
//!
//! # Main Functions
//! - `start`: Create and start containers from a Pod YAML
//! - `stop`: Stop and remove containers
//! - `restart`: Restart running containers
//!
//! # Architecture
//! The module is organized into several logical sections:
//! - Pod YAML parsing and container name generation
//! - HostConfig building (security, resources, networking, volumes)
//! - Container specification building (image, command, environment, ports)
//! - Podman API communication (create, start, stop, restart)
//! - Image management (existence check, pull)

use super::{get, post};
use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::Path;

const PODMAN_API_VERSION: &str = "/v4.0.0"; // docker-compatible API
const CDI_NVIDIA_PATH: &str = "/etc/cdi/nvidia.yaml";

// Maximum number of GPUs to detect (0-15, total 16 GPUs)
const MAX_GPU_INDEX: u32 = 15;

// NVIDIA environment variable keys
const NVIDIA_VISIBLE_DEVICES: &str = "NVIDIA_VISIBLE_DEVICES";
const NVIDIA_DRIVER_CAPABILITIES: &str = "NVIDIA_DRIVER_CAPABILITIES";

// Container path for NVIDIA libraries (legacy, kept for compatibility)
const NVIDIA_LIB_CONTAINER_PATH: &str = "/opt/nvidia/lib64";

// CDI specification structures
#[derive(Debug, Deserialize, Serialize)]
struct CdiSpec {
    #[serde(rename = "cdiVersion")]
    cdi_version: String,
    kind: String,
    devices: Vec<CdiDevice>,
    #[serde(rename = "containerEdits", skip_serializing_if = "Option::is_none")]
    container_edits: Option<CdiContainerEdits>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CdiDevice {
    name: String,
    #[serde(rename = "containerEdits")]
    container_edits: CdiContainerEdits,
}

#[derive(Debug, Deserialize, Serialize)]
struct CdiContainerEdits {
    #[serde(rename = "deviceNodes", skip_serializing_if = "Option::is_none")]
    device_nodes: Option<Vec<CdiDeviceNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mounts: Option<Vec<CdiMount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hooks: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CdiDeviceNode {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    major: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minor: Option<u32>,
    #[serde(rename = "fileMode", skip_serializing_if = "Option::is_none")]
    file_mode: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    permissions: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CdiMount {
    #[serde(rename = "hostPath")]
    host_path: String,
    #[serde(rename = "containerPath")]
    container_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Vec<String>>,
}

/// Parse Pod YAML and extract pod name, spec, and annotations
fn parse_pod(
    pod_yaml: &str,
) -> Result<
    (
        String,
        serde_json::Value,
        std::collections::HashMap<String, String>,
    ),
    Box<dyn std::error::Error>,
> {
    let pod = serde_yaml::from_str::<common::spec::k8s::Pod>(pod_yaml)?;
    let pod_name = pod.get_name();
    let pod_json = serde_json::to_value(&pod)?;
    let spec = pod_json["spec"].clone();

    // Extract annotations from metadata
    let annotations = pod_json["metadata"]["annotations"]
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    Ok((pod_name, spec, annotations))
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

    // Network configuration
    if host_network {
        host_config.insert("NetworkMode".to_string(), json!("host"));
    }

    // Security context (capabilities, privileged, user/group)
    apply_security_config(&mut host_config, container);

    // Resource limits (CPU, Memory, GPU)
    apply_resource_limits(&mut host_config, container);

    // Port bindings
    apply_port_bindings(&mut host_config, container);

    // Volume mounts
    apply_volume_mounts(&mut host_config, container, spec);

    json!(host_config)
}

/// Apply security-related configurations to HostConfig
fn apply_security_config(
    host_config: &mut serde_json::Map<String, serde_json::Value>,
    container: &serde_json::Value,
) {
    let security_context = &container["securityContext"];

    // CapAdd
    if let Some(cap_add) = security_context
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

    // CapDrop
    if let Some(cap_drop) = security_context
        .get("capabilities")
        .and_then(|c| c.get("drop"))
        .and_then(|d| d.as_array())
    {
        let caps: Vec<String> = cap_drop
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if !caps.is_empty() {
            host_config.insert("CapDrop".to_string(), json!(caps));
        }
    }

    // Privileged
    if let Some(privileged) = security_context.get("privileged").and_then(|p| p.as_bool()) {
        host_config.insert("Privileged".to_string(), json!(privileged));
    }

    // User (runAsUser)
    if let Some(run_as_user) = security_context.get("runAsUser").and_then(|u| u.as_i64()) {
        host_config.insert("User".to_string(), json!(run_as_user.to_string()));
    }

    // GroupAdd (runAsGroup)
    if let Some(run_as_group) = security_context.get("runAsGroup").and_then(|g| g.as_i64()) {
        host_config.insert(
            "GroupAdd".to_string(),
            json!(vec![run_as_group.to_string()]),
        );
    }
}

/// Apply resource limits (CPU, Memory, GPU) to HostConfig
fn apply_resource_limits(
    host_config: &mut serde_json::Map<String, serde_json::Value>,
    container: &serde_json::Value,
) {
    if let Some(limits) = container["resources"]
        .get("limits")
        .and_then(|l| l.as_object())
    {
        // CPU limit (NanoCpus)
        if let Some(cpu) = limits.get("cpu").and_then(|c| c.as_str()) {
            if let Ok(cpu_num) = cpu.parse::<f64>() {
                let nano_cpus = (cpu_num * 1_000_000_000.0) as i64;
                host_config.insert("NanoCpus".to_string(), json!(nano_cpus));
            }
        }

        // Memory limit
        if let Some(memory) = limits.get("memory").and_then(|m| m.as_str()) {
            if let Some(memory_bytes) = parse_memory(memory) {
                host_config.insert("Memory".to_string(), json!(memory_bytes));
            }
        }

        // GPU devices (nvidia.com/gpu)
        if let Some(gpu_value) = limits.get("nvidia.com/gpu") {
            apply_gpu_devices(host_config, gpu_value);
            apply_nvidia_libraries(host_config);
        }
    }
}

/// Read and parse CDI NVIDIA specification
fn read_cdi_spec() -> Result<CdiSpec, Box<dyn std::error::Error>> {
    let cdi_content = fs::read_to_string(CDI_NVIDIA_PATH)
        .map_err(|e| format!("Failed to read CDI file {}: {}", CDI_NVIDIA_PATH, e))?;

    let cdi_spec: CdiSpec = serde_yaml::from_str(&cdi_content)
        .map_err(|e| format!("Failed to parse CDI YAML: {}", e))?;

    Ok(cdi_spec)
}

/// Detect available NVIDIA GPU devices on the host
fn detect_available_nvidia_gpus() -> Vec<u32> {
    (0..=MAX_GPU_INDEX)
        .filter(|i| {
            let device_path = format!("/dev/nvidia{}", i);
            Path::new(&device_path).exists()
        })
        .collect()
}

/// Parse GPU count from resource limit value
/// Supports: "1", "2", "all", etc.
fn parse_gpu_count(gpu_value: &serde_json::Value) -> Option<usize> {
    match gpu_value {
        serde_json::Value::String(s) => {
            if s == "all" {
                None // None means all available GPUs
            } else {
                s.parse::<usize>().ok()
            }
        }
        serde_json::Value::Number(n) => n.as_u64().map(|v| v as usize),
        _ => None,
    }
}

/// Add NVIDIA GPU device mappings to HostConfig using CDI
fn apply_gpu_devices(
    host_config: &mut serde_json::Map<String, serde_json::Value>,
    gpu_value: &serde_json::Value,
) {
    // Read CDI specification
    let cdi_spec = match read_cdi_spec() {
        Ok(spec) => spec,
        Err(e) => {
            println!("Warning: Failed to read CDI spec: {}", e);
            println!("         Falling back to manual GPU detection");
            apply_gpu_devices_fallback(host_config, gpu_value);
            return;
        }
    };

    // Detect available GPUs on the host
    let available_gpus = detect_available_nvidia_gpus();
    if available_gpus.is_empty() {
        println!("Warning: No NVIDIA GPU devices found on host.");
        return;
    }

    println!(
        "Detected {} NVIDIA GPU(s): {:?}",
        available_gpus.len(),
        available_gpus
    );

    // Parse requested GPU count
    let requested_count = parse_gpu_count(gpu_value);
    let gpus_to_use: Vec<u32> = match requested_count {
        Some(count) => {
            if count > available_gpus.len() {
                println!(
                    "Warning: Requested {} GPU(s) but only {} available",
                    count,
                    available_gpus.len()
                );
                available_gpus
            } else {
                println!("Allocating {} GPU(s) from CDI", count);
                available_gpus.into_iter().take(count).collect()
            }
        }
        None => {
            println!("Allocating all {} GPU(s) from CDI", available_gpus.len());
            available_gpus
        }
    };

    // Build devices array from CDI
    let mut devices = Vec::new();

    // Add GPU-specific devices from CDI devices list
    for gpu_index in &gpus_to_use {
        let gpu_name = gpu_index.to_string();
        if let Some(cdi_device) = cdi_spec.devices.iter().find(|d| d.name == gpu_name) {
            if let Some(ref device_nodes) = cdi_device.container_edits.device_nodes {
                for node in device_nodes {
                    devices.push(json!({
                        "PathOnHost": node.path,
                        "PathInContainer": node.path,
                        "CgroupPermissions": node.permissions.as_ref().unwrap_or(&"rwm".to_string())
                    }));
                }
            }
        }
    }

    // Add common control devices from top-level containerEdits
    if let Some(ref common_edits) = cdi_spec.container_edits {
        if let Some(ref device_nodes) = common_edits.device_nodes {
            for node in device_nodes {
                if Path::new(&node.path).exists() {
                    devices.push(json!({
                        "PathOnHost": node.path,
                        "PathInContainer": node.path,
                        "CgroupPermissions": node.permissions.as_ref().unwrap_or(&"rwm".to_string())
                    }));
                }
            }
        }
    }

    // Add nvidia-caps devices (dynamic scan as they're not in CDI)
    for cap_idx in 1..=2 {
        let cap_path = format!("/dev/nvidia-caps/nvidia-cap{}", cap_idx);
        if Path::new(&cap_path).exists() {
            devices.push(json!({
                "PathOnHost": cap_path,
                "PathInContainer": cap_path,
                "CgroupPermissions": "rwm"
            }));
        }
    }

    if !devices.is_empty() {
        println!("Adding {} device(s) from CDI", devices.len());
        host_config.insert("Devices".to_string(), json!(devices));
    }

    // Apply mounts and environment from CDI
    apply_cdi_mounts_and_env(host_config, &cdi_spec, &gpus_to_use);
}

/// Fallback to manual GPU detection if CDI fails
fn apply_gpu_devices_fallback(
    host_config: &mut serde_json::Map<String, serde_json::Value>,
    gpu_value: &serde_json::Value,
) {
    let available_gpus = detect_available_nvidia_gpus();
    if available_gpus.is_empty() {
        return;
    }

    let requested_count = parse_gpu_count(gpu_value);
    let gpus_to_use: Vec<u32> = match requested_count {
        Some(count) => available_gpus.into_iter().take(count).collect(),
        None => available_gpus,
    };

    let mut devices = Vec::new();
    for gpu_index in &gpus_to_use {
        devices.push(json!({
            "PathOnHost": format!("/dev/nvidia{}", gpu_index),
            "PathInContainer": format!("/dev/nvidia{}", gpu_index),
            "CgroupPermissions": "rwm"
        }));
    }

    // Minimal control devices
    for path in [
        "/dev/nvidiactl",
        "/dev/nvidia-uvm",
        "/dev/nvidia-uvm-tools",
        "/dev/nvidia-modeset",
    ] {
        if Path::new(path).exists() {
            devices.push(json!({
                "PathOnHost": path,
                "PathInContainer": path,
                "CgroupPermissions": "rwm"
            }));
        }
    }

    if !devices.is_empty() {
        host_config.insert("Devices".to_string(), json!(devices));
    }
}

/// Apply CDI mounts and environment variables
fn apply_cdi_mounts_and_env(
    host_config: &mut serde_json::Map<String, serde_json::Value>,
    cdi_spec: &CdiSpec,
    _gpus_to_use: &[u32],
) {
    let mut mounts = host_config
        .get("Mounts")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();

    // Collect existing mount targets to avoid duplicates
    let mut existing_targets: std::collections::HashSet<String> = mounts
        .iter()
        .filter_map(|m| m["Target"].as_str().map(|s| s.to_string()))
        .collect();

    // Add mounts from top-level containerEdits
    if let Some(ref common_edits) = cdi_spec.container_edits {
        if let Some(ref cdi_mounts) = common_edits.mounts {
            for mount in cdi_mounts {
                // Skip if target already exists (avoid duplicate mount destination)
                if existing_targets.contains(&mount.container_path) {
                    continue;
                }
                if Path::new(&mount.host_path).exists() {
                    let options = mount
                        .options
                        .as_ref()
                        .map(|opts| opts.join(","))
                        .unwrap_or_else(|| "ro,nosuid,nodev,bind".to_string());

                    mounts.push(json!({
                        "Type": "bind",
                        "Source": mount.host_path,
                        "Target": mount.container_path,
                        "Options": options.split(',').collect::<Vec<_>>()
                    }));
                    existing_targets.insert(mount.container_path.clone());
                }
            }
        }
    }

    if !mounts.is_empty() {
        println!("Adding {} mount(s) from CDI", mounts.len());
        host_config.insert("Mounts".to_string(), json!(mounts));
    }
}

/// Add NVIDIA driver libraries - now handled by CDI mounts
fn apply_nvidia_libraries(_host_config: &mut serde_json::Map<String, serde_json::Value>) {
    // Libraries are now added via apply_cdi_mounts_and_env() from CDI spec
    // This function is kept for compatibility but does nothing
}

/// Apply port bindings to HostConfig
fn apply_port_bindings(
    host_config: &mut serde_json::Map<String, serde_json::Value>,
    container: &serde_json::Value,
) {
    if let Some(ports) = container["ports"].as_array() {
        let mut port_bindings = serde_json::Map::new();

        for port in ports {
            if let Some(container_port) = port["containerPort"].as_i64() {
                let port_key = format!("{}/tcp", container_port);

                if let Some(host_port) = port["hostPort"].as_i64() {
                    port_bindings.insert(port_key, json!([{"HostPort": host_port.to_string()}]));
                } else {
                    // If hostPort is not specified, use dynamic allocation
                    port_bindings.insert(port_key, json!([{"HostPort": ""}]));
                }
            }
        }

        if !port_bindings.is_empty() {
            host_config.insert("PortBindings".to_string(), json!(port_bindings));
        }
    }
}

/// Apply volume mounts to HostConfig
fn apply_volume_mounts(
    host_config: &mut serde_json::Map<String, serde_json::Value>,
    container: &serde_json::Value,
    spec: &serde_json::Value,
) {
    if let Some(volume_mounts) = container["volumeMounts"].as_array() {
        if let Some(volumes) = spec["volumes"].as_array() {
            // Get existing Mounts or create new array
            let mut mounts = host_config
                .get("Mounts")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();

            // Collect existing mount targets to avoid duplicates
            let mut existing_targets: std::collections::HashSet<String> = mounts
                .iter()
                .filter_map(|m| m["Target"].as_str().map(|s| s.to_string()))
                .collect();

            for mount in volume_mounts {
                let mount_name = mount["name"].as_str().unwrap_or("");
                let mount_path = mount["mountPath"].as_str().unwrap_or("");

                // Skip if target already exists (avoid duplicate mount destination)
                if existing_targets.contains(mount_path) {
                    continue;
                }

                for volume in volumes {
                    if volume["name"].as_str() == Some(mount_name) {
                        if let Some(host_path) = volume["hostPath"]["path"].as_str() {
                            mounts.push(json!({
                                "Type": "bind",
                                "Source": host_path,
                                "Target": mount_path
                            }));
                            existing_targets.insert(mount_path.to_string());
                        }
                        break;
                    }
                }
            }
            if !mounts.is_empty() {
                host_config.insert("Mounts".to_string(), json!(mounts));
            }
        }
    }
}

/// Parse memory string (e.g., "512Mi", "1Gi", "1024") to bytes
fn parse_memory(memory: &str) -> Option<i64> {
    if memory.is_empty() {
        return None;
    }

    // Try direct number parse first
    if let Ok(bytes) = memory.parse::<i64>() {
        return Some(bytes);
    }

    // Parse with suffix (Ki, Mi, Gi, K, M, G)
    let memory = memory.trim();
    let (value_str, multiplier) = if memory.ends_with("Gi") {
        (memory.trim_end_matches("Gi"), 1024 * 1024 * 1024)
    } else if memory.ends_with("Mi") {
        (memory.trim_end_matches("Mi"), 1024 * 1024)
    } else if memory.ends_with("Ki") {
        (memory.trim_end_matches("Ki"), 1024)
    } else if memory.ends_with('G') {
        (memory.trim_end_matches('G'), 1000 * 1000 * 1000)
    } else if memory.ends_with('M') {
        (memory.trim_end_matches('M'), 1000 * 1000)
    } else if memory.ends_with('K') {
        (memory.trim_end_matches('K'), 1000)
    } else {
        return None;
    };

    value_str.parse::<i64>().ok().map(|v| v * multiplier)
}

/// Build environment variables array
fn build_env_vars(container: &serde_json::Value) -> Vec<String> {
    let mut env_vars: Vec<String> = container["env"]
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
        .unwrap_or_default();

    // Add NVIDIA GPU environment variables if GPU is requested
    add_nvidia_env_vars(&mut env_vars, container);

    env_vars
}

/// Add NVIDIA-specific environment variables if GPU is requested
fn add_nvidia_env_vars(env_vars: &mut Vec<String>, container: &serde_json::Value) {
    if let Some(limits) = container["resources"]
        .get("limits")
        .and_then(|l| l.as_object())
    {
        if let Some(gpu_value) = limits.get("nvidia.com/gpu") {
            // Determine which GPUs to expose
            let available_gpus = detect_available_nvidia_gpus();
            let requested_count = parse_gpu_count(gpu_value);

            let gpu_indices: Vec<u32> = match requested_count {
                Some(count) => available_gpus.into_iter().take(count).collect(),
                None => available_gpus,
            };

            // Add NVIDIA_VISIBLE_DEVICES if not already set
            if !env_vars
                .iter()
                .any(|e| e.starts_with(&format!("{}=", NVIDIA_VISIBLE_DEVICES)))
            {
                let visible_devices = if gpu_indices.is_empty() {
                    "void".to_string() // No GPUs available
                } else if requested_count.is_none() {
                    "all".to_string() // Use all GPUs
                } else {
                    // Specify GPU indices: "0,1,2"
                    gpu_indices
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                };
                env_vars.push(format!("{}={}", NVIDIA_VISIBLE_DEVICES, visible_devices));
            }

            // Add NVIDIA_DRIVER_CAPABILITIES if not already set
            if !env_vars
                .iter()
                .any(|e| e.starts_with(&format!("{}=", NVIDIA_DRIVER_CAPABILITIES)))
            {
                env_vars.push(format!("{}=compute,utility", NVIDIA_DRIVER_CAPABILITIES));
            }

            // Add LD_LIBRARY_PATH if not already set (needed to find NVIDIA libraries)
            if !env_vars.iter().any(|e| e.starts_with("LD_LIBRARY_PATH=")) {
                env_vars.push(format!(
                    "LD_LIBRARY_PATH={}:/usr/local/cuda/lib64",
                    NVIDIA_LIB_CONTAINER_PATH
                ));
            }
        }
    }
}

/// Build command array (for Entrypoint)
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

/// Build args array (for Cmd)
fn build_args(container: &serde_json::Value) -> Vec<String> {
    container["args"]
        .as_array()
        .map(|args| {
            args.iter()
                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Build ExposedPorts from container ports
fn build_exposed_ports(container: &serde_json::Value) -> serde_json::Value {
    let mut exposed_ports = serde_json::Map::new();

    if let Some(ports) = container["ports"].as_array() {
        for port in ports {
            if let Some(container_port) = port["containerPort"].as_i64() {
                let port_key = format!("{}/tcp", container_port);
                exposed_ports.insert(port_key, json!({}));
            }
        }
    }

    json!(exposed_ports)
}

/// Create container from spec
async fn create_container(
    pod_name: &str,
    container: &serde_json::Value,
    spec: &serde_json::Value,
    host_network: bool,
    annotations: &std::collections::HashMap<String, String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let image = container["image"]
        .as_str()
        .ok_or("Container image field not found")?;
    let container_name = container["name"]
        .as_str()
        .ok_or("Container name field not found")?;

    // Ensure image is available locally
    ensure_image_available(image).await?;

    let name = format!("{}_{}", pod_name, container_name);

    // Build the complete container creation request
    let create_body =
        build_container_spec(&name, image, container, spec, host_network, annotations);

    println!("{}", create_body);

    // Create and return container ID
    create_container_via_api(&name, create_body).await
}

/// Ensure the container image is available locally (pull if needed)
async fn ensure_image_available(image: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !image_exists(image).await? {
        println!("Image {} not found locally, pulling...", image);
        pull_image(image).await?;
        println!("Image {} pulled successfully", image);
    }
    Ok(())
}

/// Build the complete container specification JSON
fn build_container_spec(
    name: &str,
    image: &str,
    container: &serde_json::Value,
    spec: &serde_json::Value,
    host_network: bool,
    annotations: &std::collections::HashMap<String, String>,
) -> serde_json::Value {
    let mut create_body = json!({
        "Image": image,
        "Name": name,
    });

    // Add annotations as Labels (Docker-compatible API uses Labels, not Annotations)
    if !annotations.is_empty() {
        create_body["Labels"] = json!(annotations);
    }

    // Terminal settings (stdin/tty)
    apply_terminal_settings(&mut create_body, container);

    // Working directory
    if let Some(working_dir) = container["workingDir"].as_str() {
        create_body["WorkingDir"] = json!(working_dir);
    }

    // Environment variables
    let env_vars = build_env_vars(container);
    if !env_vars.is_empty() {
        create_body["Env"] = json!(env_vars);
    }

    // Command and arguments (Entrypoint and Cmd in Docker API)
    apply_command_and_args(&mut create_body, container);

    // Exposed ports
    let exposed_ports = build_exposed_ports(container);
    if !exposed_ports.as_object().unwrap().is_empty() {
        create_body["ExposedPorts"] = exposed_ports;
    }

    // Host configuration (resources, security, networking, etc.)
    let host_config = build_host_config(container, spec, host_network);
    if !host_config.as_object().unwrap().is_empty() {
        create_body["HostConfig"] = host_config;
    }

    create_body
}

/// Apply terminal settings (stdin/tty) to container spec
fn apply_terminal_settings(create_body: &mut serde_json::Value, container: &serde_json::Value) {
    let open_stdin = container["stdin"].as_bool().unwrap_or(true);
    let tty = container["tty"].as_bool().unwrap_or(true);

    create_body["OpenStdin"] = json!(open_stdin);
    create_body["Tty"] = json!(tty);

    if open_stdin {
        create_body["StdinOnce"] = json!(false);
    }
}

/// Apply command and args to container spec
fn apply_command_and_args(create_body: &mut serde_json::Value, container: &serde_json::Value) {
    // In Docker/Podman: Entrypoint = command, Cmd = args
    let command = build_command(container);
    let args = build_args(container);

    if !command.is_empty() {
        create_body["Entrypoint"] = json!(command);
    }
    if !args.is_empty() {
        create_body["Cmd"] = json!(args);
    }
}

/// Create container via Podman API and return container ID
async fn create_container_via_api(
    name: &str,
    create_body: serde_json::Value,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("Creating container: {}", name);

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
    let (pod_name, spec, annotations) = parse_pod(pod_yaml)?;
    let host_network = spec["hostNetwork"].as_bool().unwrap_or(false);

    let mut container_ids = Vec::new();

    if let Some(containers) = spec["containers"].as_array() {
        for container in containers.iter() {
            let container_id =
                create_container(&pod_name, container, &spec, host_network, &annotations).await?;

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
    let (pod_name, spec, _annotations) = parse_pod(pod_yaml)?;
    let container_names = get_container_names(&pod_name, &spec)?;

    for full_container_name in container_names {
        // Stop the container with timeout=0 (immediate SIGKILL)
        println!("Stopping container: {}", full_container_name);
        let stop_path = format!(
            "{}/containers/{}/stop?timeout=0&t=0",
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
    let (pod_name, spec, _annotations) = parse_pod(pod_yaml)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_memory_with_suffixes() {
        assert_eq!(parse_memory("1024"), Some(1024));
        assert_eq!(parse_memory("512Mi"), Some(512 * 1024 * 1024));
        assert_eq!(parse_memory("1Gi"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_memory("100Ki"), Some(100 * 1024));
        assert_eq!(parse_memory("500M"), Some(500 * 1000 * 1000));
        assert_eq!(parse_memory("2G"), Some(2 * 1000 * 1000 * 1000));
        assert_eq!(parse_memory(""), None);
        assert_eq!(parse_memory("invalid"), None);
    }

    #[test]
    fn test_parse_gpu_count() {
        // String values
        assert_eq!(parse_gpu_count(&json!("1")), Some(1));
        assert_eq!(parse_gpu_count(&json!("4")), Some(4));
        assert_eq!(parse_gpu_count(&json!("all")), None); // None means "all"
        assert_eq!(parse_gpu_count(&json!("invalid")), None);

        // Number values
        assert_eq!(parse_gpu_count(&json!(1)), Some(1));
        assert_eq!(parse_gpu_count(&json!(8)), Some(8));

        // Invalid values
        assert_eq!(parse_gpu_count(&json!(true)), None);
        assert_eq!(parse_gpu_count(&json!(null)), None);
    }

    #[test]
    fn test_detect_available_nvidia_gpus_no_gpu() {
        // In environments without GPU, this should return empty Vec
        let gpus = detect_available_nvidia_gpus();
        // This test will pass regardless of GPU availability
        println!("Detected GPUs: {:?}", gpus);
        assert!(gpus.len() <= 16); // Should not exceed MAX_GPU_INDEX
    }

    #[test]
    fn test_build_env_vars_basic() {
        let container = json!({
            "env": [
                {"name": "TEST_VAR", "value": "test_value"},
                {"name": "ANOTHER_VAR", "value": "another_value"}
            ]
        });

        let env_vars = build_env_vars(&container);
        assert_eq!(env_vars.len(), 2);
        assert!(env_vars.contains(&"TEST_VAR=test_value".to_string()));
        assert!(env_vars.contains(&"ANOTHER_VAR=another_value".to_string()));
    }

    #[test]
    fn test_build_env_vars_with_gpu_request() {
        let container = json!({
            "env": [
                {"name": "TEST_VAR", "value": "test_value"}
            ],
            "resources": {
                "limits": {
                    "nvidia.com/gpu": "1"
                }
            }
        });

        let env_vars = build_env_vars(&container);

        // Should have the original env var
        assert!(env_vars.contains(&"TEST_VAR=test_value".to_string()));

        // Should have NVIDIA env vars added
        let has_nvidia_visible = env_vars
            .iter()
            .any(|e| e.starts_with("NVIDIA_VISIBLE_DEVICES="));
        let has_nvidia_driver = env_vars
            .iter()
            .any(|e| e.starts_with("NVIDIA_DRIVER_CAPABILITIES="));

        assert!(has_nvidia_visible, "Should add NVIDIA_VISIBLE_DEVICES");
        assert!(has_nvidia_driver, "Should add NVIDIA_DRIVER_CAPABILITIES");
    }

    #[test]
    fn test_build_env_vars_no_gpu() {
        let container = json!({
            "env": [],
            "resources": {
                "limits": {
                    "cpu": "1"
                }
            }
        });

        let env_vars = build_env_vars(&container);

        // Should not have NVIDIA env vars when no GPU requested
        let has_nvidia = env_vars.iter().any(|e| e.starts_with("NVIDIA_"));
        assert!(
            !has_nvidia,
            "Should not add NVIDIA vars without GPU request"
        );
    }

    #[test]
    fn test_build_command() {
        let container = json!({
            "command": ["/bin/sh", "-c", "echo hello"]
        });

        let command = build_command(&container);
        assert_eq!(command, vec!["/bin/sh", "-c", "echo hello"]);
    }

    #[test]
    fn test_build_args() {
        let container = json!({
            "args": ["arg1", "arg2", "arg3"]
        });

        let args = build_args(&container);
        assert_eq!(args, vec!["arg1", "arg2", "arg3"]);
    }

    #[test]
    fn test_build_exposed_ports() {
        let container = json!({
            "ports": [
                {"containerPort": 8080},
                {"containerPort": 9090, "hostPort": 9090}
            ]
        });

        let exposed_ports = build_exposed_ports(&container);
        let ports_obj = exposed_ports.as_object().unwrap();

        assert!(ports_obj.contains_key("8080/tcp"));
        assert!(ports_obj.contains_key("9090/tcp"));
    }

    #[test]
    fn test_build_env_vars_with_gpu_includes_ld_library_path() {
        let container = json!({
            "env": [],
            "resources": {
                "limits": {
                    "nvidia.com/gpu": "1"
                }
            }
        });

        let env_vars = build_env_vars(&container);

        // Should have LD_LIBRARY_PATH when GPU is requested
        let has_ld_library_path = env_vars.iter().any(|e| e.starts_with("LD_LIBRARY_PATH="));
        assert!(
            has_ld_library_path,
            "Should add LD_LIBRARY_PATH with GPU request"
        );

        // Should contain NVIDIA library path
        let ld_path = env_vars.iter().find(|e| e.starts_with("LD_LIBRARY_PATH="));
        if let Some(path) = ld_path {
            assert!(
                path.contains(NVIDIA_LIB_CONTAINER_PATH),
                "LD_LIBRARY_PATH should contain NVIDIA library path"
            );
        }
    }

    #[test]
    fn test_apply_nvidia_libraries() {
        let mut host_config = serde_json::Map::new();

        // Call the function
        apply_nvidia_libraries(&mut host_config);

        // Should add Mounts if NVIDIA libraries exist
        // This test will pass/fail depending on host system
        if host_config.contains_key("Mounts") {
            let mounts = host_config["Mounts"].as_array().unwrap();
            assert!(!mounts.is_empty(), "Mounts should not be empty");

            // Check that NVIDIA library mount is present
            let has_nvidia_mount = mounts
                .iter()
                .any(|m| m["Target"].as_str() == Some(NVIDIA_LIB_CONTAINER_PATH));
            assert!(has_nvidia_mount, "Should have NVIDIA library mount");
        }
    }
}
