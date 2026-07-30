/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! Minimal end-to-end client for the Dynamic Resource Scaling feature
//! (#514 / #526).
//!
//! Sends a `RequestResourceScaling` RPC to a running ActionController, which
//! then drives the ResourceManager and the target NodeAgent to apply the new
//! CPU / memory limits at runtime. The ActionController endpoint defaults to
//! the local node's address read from `/etc/pullpiri/settings.yaml`
//! (`host.ip`), so it does not normally need to be supplied.
//!
//! Usage:
//!   cargo run -p actioncontroller --example scaling_client -- \
//!       <workload_id> <cpu> <mem_mib> [node] [endpoint]
//!
//! `<cpu>` accepts fractional cores or Kubernetes-style millicores:
//!   "1"   -> 1 core (1000m)   "0.5" -> 500m   "500m" -> 500m   "2000m" -> 2 cores
//!
//! `[node]` is the target node, given either as a hostname (e.g. "sdv") or an
//! IP address. A hostname is resolved to an IP by the ActionController via the
//! cluster node registry, so a workload on a remote node can be targeted by
//! name. When omitted, the local node is used.
//!
//! `[endpoint]` overrides the ActionController gRPC endpoint. When omitted, it
//! is derived from the local settings file as `http://<host.ip>:47001`.
//!
//! The scale direction (up / down) is decided by the ResourceManager from the
//! current desired state, so it is not a client argument.
//!
//! Examples:
//!   # resize container "helloworld_helloworld" to 0.5 core / 64 MiB on node "sdv"
//!   cargo run -p actioncontroller --example scaling_client -- \
//!       helloworld_helloworld 0.5 64 sdv
//!
//!   # resize to 2 cores / 512 MiB, targeting a node by IP
//!   cargo run -p actioncontroller --example scaling_client -- \
//!       my-workload 2 512 192.168.0.10

use common::actioncontroller::action_controller_connection_client::ActionControllerConnectionClient;
use common::actioncontroller::{ScalingActionRequest, ScalingType};

/// Parse a CPU quantity into millicores (1000 == 1 core).
///
/// Accepts a Kubernetes-style millicore suffix ("500m") or a fractional /
/// whole core count ("0.5", "1", "2").
fn parse_cpu_millicores(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(milli) = s.strip_suffix('m') {
        return milli
            .trim()
            .parse::<u32>()
            .map_err(|e| format!("invalid millicore value '{s}': {e}"));
    }
    let cores: f64 = s
        .parse()
        .map_err(|e| format!("invalid cpu value '{s}': {e}"))?;
    if cores < 0.0 {
        return Err(format!("cpu must be non-negative, got '{s}'"));
    }
    Ok((cores * 1000.0).round() as u32)
}

/// Build the default ActionController endpoint from the local settings file.
///
/// The client always runs on the local (master) node, so the endpoint is
/// derived from `host.ip` in `/etc/pullpiri/settings.yaml`. Falls back to
/// `127.0.0.1` when the address is unset or a wildcard bind (`0.0.0.0`).
fn default_endpoint() -> String {
    let ip = common::setting::get_config().host.ip.clone();
    let ip = if ip.is_empty() || ip == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        ip
    };
    format!("http://{ip}:47001")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "usage: {} <workload_id> <cpu> <mem_mib> [node] [endpoint]\n\
             <cpu>      accepts cores ('0.5', '1', '2') or millicores ('500m', '1500m')\n\
             [node]     target node hostname (e.g. 'sdv') or IP address\n\
             [endpoint] ActionController address; defaults to host.ip from settings.yaml",
            args[0]
        );
        std::process::exit(2);
    }

    let workload_id = args[1].clone();
    let cpu_millicores: u32 = parse_cpu_millicores(&args[2]).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    let mem_mib: u64 = args[3].parse().expect("mem_mib must be an integer");
    // Target node: hostname or IP. The ActionController resolves a hostname to
    // an IP through the cluster node registry.
    let node_id = args.get(4).cloned().unwrap_or_default();
    // ActionController endpoint. Defaults to the local node from the settings
    // file so it does not need to be supplied for local scaling.
    let endpoint = args.get(5).cloned().unwrap_or_else(default_endpoint);

    println!(
        "-> RequestResourceScaling endpoint={endpoint} node='{node_id}' workload='{workload_id}' cpu={cpu_millicores}m mem={mem_mib}MiB"
    );

    let mut client = ActionControllerConnectionClient::connect(endpoint).await?;
    let response = client
        .request_resource_scaling(ScalingActionRequest {
            node_id,
            workload_id,
            target_cpu_limit: cpu_millicores,
            target_memory_limit: mem_mib,
            // Deprecated: the scale direction is derived server-side from the
            // current desired state. Kept only to satisfy the proto schema.
            scaling_type: ScalingType::ScaleUp as i32,
        })
        .await?
        .into_inner();

    println!("<- success        : {}", response.success);
    println!("<- message        : {}", response.message);
    println!("<- sync_state     : {}", response.sync_state);
    println!("<- actual_cpu     : {}m", response.actual_cpu_limit);
    println!("<- actual_memory  : {} MiB", response.actual_memory_limit);

    if response.success {
        Ok(())
    } else {
        std::process::exit(1)
    }
}
