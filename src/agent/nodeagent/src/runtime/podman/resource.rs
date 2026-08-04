/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! Runtime resource update for Dynamic Resource Scaling (#514 / #526).
//!
//! Updates the CPU / memory limits of a *running* container through the Podman
//! REST API without restarting the workload.
//!
//! Podman's libpod update endpoint (`POST /libpod/containers/{id}/update`)
//! accepts an OCI `LinuxResources` object and applies the change directly to the
//! container cgroup (verified via `cpu.max` / `memory.max`). Note that Podman
//! does **not** reflect a runtime update back into `inspect`'s `HostConfig`
//! (which keeps the creation-time spec), so the applied values are returned
//! directly from a successful update.

use super::{get, post};
use hyper::Body;
use serde_json::{json, Value};

/// Docker-compatible Podman API version prefix (see `container.rs`).
const PODMAN_API_VERSION: &str = "/v4.0.0";

const MIB: u64 = 1024 * 1024;
/// Nanoseconds of CPU time per millicore (1 core == 1000 millicores == 1e9 ns).
const NANO_PER_MILLICORE: u64 = 1_000_000;
/// Millicores per whole CPU core.
const MILLICORES_PER_CORE: i64 = 1000;
/// CPU CFS period (microseconds). One core == quota equal to the period.
const CPU_PERIOD_US: i64 = 100_000;

/// Actual runtime resource limits of a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceStatus {
    /// CPU limit in millicores (1000 == 1 core, 0 means unlimited).
    pub cpu_limit: u32,
    /// Memory limit in MiB (0 means unlimited).
    pub memory_limit: u64,
}

/// Apply new CPU / memory limits to a running container at runtime.
///
/// `cpu_limit` is expressed in millicores (1000 == 1 core) and
/// `memory_limit_mib` in MiB. Either field may be omitted to update only one
/// resource type. On success the applied values are returned.
pub async fn update_resources(
    workload_id: &str,
    cpu_limit: Option<u32>,
    memory_limit_mib: Option<u64>,
) -> Result<ResourceStatus, Box<dyn std::error::Error>> {
    let mut resources = serde_json::Map::new();
    if let Some(cpu) = cpu_limit {
        // OCI LinuxCPU: quota per period. quota == millicores * period / 1000.
        resources.insert(
            "cpu".to_string(),
            json!({
                "period": CPU_PERIOD_US,
                "quota": cpu as i64 * CPU_PERIOD_US / MILLICORES_PER_CORE,
            }),
        );
    }
    if let Some(mem) = memory_limit_mib {
        // OCI LinuxMemory: limit in bytes.
        resources.insert("memory".to_string(), json!({ "limit": (mem * MIB) as i64 }));
    }

    if resources.is_empty() {
        return Err("no resource fields provided for update".into());
    }

    let path = format!(
        "{}/libpod/containers/{}/update",
        PODMAN_API_VERSION, workload_id
    );
    let raw = post(&path, Body::from(Value::Object(resources).to_string())).await?;

    // The get/post helpers do not expose the HTTP status code, so detect the
    // outcome from the payload: on failure Podman returns a JSON error object
    // ({cause, message, response}); on success it returns the container id.
    if let Some(err) = parse_error_payload(&raw) {
        return Err(err.into());
    }
    let text = String::from_utf8_lossy(&raw);
    let body = text.trim();
    let looks_like_id = body.len() >= 12 && body.chars().all(|c| c.is_ascii_hexdigit());
    if !looks_like_id {
        return Err(format!("unexpected update response: {}", body).into());
    }

    // Podman does not reflect the update in inspect's HostConfig, so the
    // requested (now applied) values are the actual runtime state.
    Ok(ResourceStatus {
        cpu_limit: cpu_limit.unwrap_or(0),
        memory_limit: memory_limit_mib.unwrap_or(0),
    })
}

/// Read the CPU / memory limits recorded in the container's `HostConfig`.
///
/// Note: Podman's `inspect` reports the creation-time spec and is **not**
/// updated by a runtime `update`. This is a best-effort query; the authoritative
/// applied values are those returned by [`update_resources`].
pub async fn get_resource_status(
    workload_id: &str,
) -> Result<ResourceStatus, Box<dyn std::error::Error>> {
    let path = format!("{}/containers/{}/json", PODMAN_API_VERSION, workload_id);
    let raw = get(&path).await?;

    if let Some(err) = parse_error_payload(&raw) {
        return Err(err.into());
    }

    let inspect: Value = serde_json::from_slice(&raw)?;
    let host_config = inspect
        .get("HostConfig")
        .ok_or("inspect response missing HostConfig")?;

    let nano_cpus = host_config
        .get("NanoCpus")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let memory_bytes = host_config
        .get("Memory")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Round NanoCpus to the nearest millicore.
    let cpu_limit = ((nano_cpus + NANO_PER_MILLICORE / 2) / NANO_PER_MILLICORE) as u32;
    let memory_limit = memory_bytes / MIB;

    Ok(ResourceStatus {
        cpu_limit,
        memory_limit,
    })
}

/// Return a descriptive error string if `raw` is a Podman error payload.
fn parse_error_payload(raw: &[u8]) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    let value: Value = serde_json::from_slice(raw).ok()?;
    // An inspect / success response is either an array or an object that does
    // not carry a top-level `cause`/`message` error field.
    let obj = value.as_object()?;
    let has_error = obj.contains_key("cause") || obj.contains_key("message");
    // `HostConfig`-bearing objects are successful inspect responses.
    if has_error && !obj.contains_key("HostConfig") {
        let message = obj
            .get("message")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("cause").and_then(|v| v.as_str()))
            .unwrap_or("podman resource update failed");
        return Some(message.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_payload_detects_error() {
        let raw = br#"{"cause":"no such container","message":"no container with name or ID \"x\" found","response":404}"#;
        assert!(parse_error_payload(raw).is_some());
    }

    #[test]
    fn parse_error_payload_ignores_inspect_success() {
        let raw = br#"{"Id":"abc","HostConfig":{"NanoCpus":2000000000,"Memory":2147483648}}"#;
        assert!(parse_error_payload(raw).is_none());
    }

    #[test]
    fn parse_error_payload_ignores_empty() {
        assert!(parse_error_payload(b"").is_none());
    }

    /// Live integration test against a running Podman container.
    ///
    /// Ignored by default; run with a prepared container:
    ///   SCALING_TEST_CONTAINER=<name> cargo test runtime::podman::resource \
    ///       -- --ignored --nocapture
    /// Requires the Podman socket at the path in `PODMAN_SOCKET`.
    #[tokio::test]
    #[ignore = "requires a running Podman container (set SCALING_TEST_CONTAINER)"]
    async fn integration_update_and_get_resources() {
        let container = std::env::var("SCALING_TEST_CONTAINER")
            .expect("set SCALING_TEST_CONTAINER to a running container name/id");

        // Scale the running container to 1 core (1000m) / 256 MiB at runtime.
        let updated = update_resources(&container, Some(1000), Some(256))
            .await
            .expect("update_resources should succeed");
        assert_eq!(
            updated.cpu_limit, 1000,
            "cpu should be 1000 millicores (1 core) after update"
        );
        assert_eq!(
            updated.memory_limit, 256,
            "memory should be 256 MiB after update"
        );

        // The applied values are verified against the real cgroup by the
        // accompanying script (scripts/test_dynamic_resource_scaling.sh).
        // `get_resource_status` reads inspect's HostConfig which Podman keeps at
        // the creation-time spec, so it is only exercised for reachability here.
        let _ = get_resource_status(&container)
            .await
            .expect("get_resource_status should be reachable");
    }
}
