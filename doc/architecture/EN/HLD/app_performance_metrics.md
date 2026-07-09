<!--
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
-->

# App Performance Metrics for Monitoring Collection

## Background

As part of the App Performance-based Reconcile use case, PolicyManager must be able to
trigger Reconcile actions based on application performance metrics.

This document defines the app performance metrics to be collected by MonitoringServer
and delivered to PolicyManager.

## Candidate Metric Research

The following candidate metrics were evaluated for applicability as PolicyManager
triggering signals. Candidates include metrics already present in the codebase and
additional metrics explored beyond the representative examples listed in the issue.

| Metric | Unit | Scope | Feasibility | Collection Complexity | Notes |
|--------|------|-------|-------------|----------------------|-------|
| FPS | frames/s | Per App | High | Low | Already in `StressMonitoringMetric`; App Data Provider pushes it |
| Latency | ms | Per App | High | Low | Already in `StressMonitoringMetric`; end-to-end response time |
| Per-core CPU Load | % / core | Per App | High | Low | Already in `StressMonitoringMetric`; pushed per collection event |
| CPU Usage (Node) | % | Per Node | High | Low | Already in `NodeInfo`; collected by NodeAgent via sysinfo |
| Memory Usage (Node) | % | Per Node | High | Low | Already in `NodeInfo`; collected by NodeAgent via sysinfo |
| Network I/O (Node) | bytes/interval | Per Node | High | Low | Already in `NodeInfo`; delta values since last collection |
| Storage I/O (Node) | bytes/interval | Per Node | High | Low | Already in `NodeInfo`; delta values since last collection |
| GPU Utilization | % | Per Node / Per App | Medium | Medium | Not yet tracked; requires vendor-specific API (e.g., NVML) or sysfs |
| Frame Drop Count | count/interval | Per App | High | Low | Derivable from FPS delta; App Data Provider can include in payload |
| Latency Jitter | ms | Per App | Medium | Low | Derivable from variance of successive latency samples |
| Throughput | requests/s or msg/s | Per App | Medium | Medium | App must expose metric; no standard interface exists yet |
| Error / Fault Rate | count/interval | Per App | Medium | Medium | App must expose metric; useful for reliability-based Reconcile |
| CPU Temperature | °C | Per Node | Low | Medium | Available via sysfs thermal zones; platform-specific and not safety-critical in most cases |
| Power Consumption | watts | Per Node | Low | High | Platform-specific (RAPL on x86, vendor API on ARM); limited availability in vehicle ECUs |
| Context Switch Rate | count/s | Per Node / Per App | Low | Medium | Available from `/proc/stat`; limited direct value for Reconcile decisions |
| Real-time Deadline Violation | count/interval | Per App | Low | High | Requires RT-Linux or RTOS integration; out of scope for current architecture |

### Feasibility Summary

- **High feasibility**: Metric source already exists in the codebase or is trivially
  derivable; low engineering effort to forward to PolicyManager.
- **Medium feasibility**: Metric is technically accessible but requires new collection
  integration or App-side instrumentation.
- **Low feasibility**: Metric is platform-specific, requires significant infrastructure
  changes, or has limited direct value for the Reconcile use case.

## Current State

### Existing Delivery Path (Node-Level Metrics)

A gRPC path already exists between MonitoringServer and PolicyManager via
`ReportNodeMetrics` (defined in `policymanager.proto`). This path currently carries
**node-level** metrics from the `NodeInfo` message:

| Metric | Type | Unit | Scope |
|--------|------|------|-------|
| `cpu_usage` | `double` | % (0–100) | Per Node |
| `cpu_count` | `uint64` | count | Per Node |
| `gpu_count` | `uint64` | count | Per Node |
| `used_memory` | `uint64` | bytes | Per Node |
| `total_memory` | `uint64` | bytes | Per Node |
| `mem_usage` | `double` | % (0–100) | Per Node |
| `rx_bytes` | `uint64` | bytes/interval | Per Node |
| `tx_bytes` | `uint64` | bytes/interval | Per Node |
| `read_bytes` | `uint64` | bytes/interval | Per Node |
| `write_bytes` | `uint64` | bytes/interval | Per Node |

Node-level metrics are collected by NodeAgent at the configured `collection_interval`
(see `MetricsConfig` in `src/agent/nodeagent/src/config.rs`).

### Missing Delivery Path (App-Level Metrics)

App-level metrics are already received by MonitoringServer via `SendStressMonitoringMetric`
(defined in `monitoringserver.proto`) and stored to etcd, but are **not forwarded to
PolicyManager**. The `StressMonitoringMetric` JSON payload contains:

| Metric | Type | Unit | Scope |
|--------|------|------|-------|
| `fps` | `f64` | frames/second | Per App (process) |
| `latency` | `u64` | milliseconds | Per App (process) |
| `cpu_loads[].load` | `f64` | % (0–100) per core | Per App (process) |
| `process_name` | `string` | — | Per App identifier |
| `pid` | `u32` | — | Per App identifier |
| `core_masking` | `string` (optional) | bitmask | Per App |
| `core_count` | `u32` (optional) | count | Per App |

App-level metrics are pushed on demand from the App Data Provider; no fixed collection
interval is enforced at the MonitoringServer level.

## Confirmed Metrics for PolicyManager Triggering

The following metrics are confirmed as targets for the App Performance-based Reconcile
use case. They are grouped by scope:

### Node-Level Metrics (already delivered to PolicyManager)

| Metric | Unit | Collection Interval | Scope | Notes |
|--------|------|---------------------|-------|-------|
| CPU Usage | % | Configurable (`collection_interval`) | Per Node | Threshold-based offloading already implemented |
| Memory Usage | % | Configurable (`collection_interval`) | Per Node | Threshold-based offloading already implemented |
| Network RX | bytes/interval | Configurable (`collection_interval`) | Per Node | Delta since last collection |
| Network TX | bytes/interval | Configurable (`collection_interval`) | Per Node | Delta since last collection |
| Storage Read | bytes/interval | Configurable (`collection_interval`) | Per Node | Delta since last collection |
| Storage Write | bytes/interval | Configurable (`collection_interval`) | Per Node | Delta since last collection |

### App-Level Metrics (delivery path to PolicyManager is missing)

These metrics were confirmed based on High feasibility rating from the candidate
evaluation above.

| Metric | Unit | Collection Interval | Scope | Notes |
|--------|------|---------------------|-------|-------|
| FPS | frames/second | Push (on demand from App) | Per App (process) | Useful for rendering/multimedia workloads |
| Latency | milliseconds | Push (on demand from App) | Per App (process) | End-to-end response time |
| Per-core CPU Load | % per core | Push (on demand from App) | Per App (process) | Granular CPU usage by core |
| Frame Drop Count | count/interval | Push (on demand from App) | Per App (process) | Derivable from FPS delta; indicates rendering degradation |
| Latency Jitter | milliseconds | Push (on demand from App) | Per App (process) | Derivable from variance of successive latency samples |

Metrics with Medium or Low feasibility (GPU Utilization, Throughput, Error Rate,
Temperature, Power Consumption, Context Switch Rate, Deadline Violation) are deferred
to future iterations pending further feasibility assessment and platform availability.

## Gap Analysis

| Item | Status |
|------|--------|
| Node-level metrics → MonitoringServer | ✅ Implemented (`SendNodeInfo` gRPC) |
| Node-level metrics → PolicyManager | ✅ Implemented (`ReportNodeMetrics` gRPC) |
| App-level metrics → MonitoringServer | ✅ Implemented (`SendStressMonitoringMetric` gRPC) |
| App-level metrics stored in etcd | ✅ Implemented |
| **App-level metrics → PolicyManager** | ❌ **Not implemented (missing delivery path)** |

## Next Steps

This document satisfies the metric definition prerequisite. The following tasks remain:

1. **Collection method decision**: Determine whether MonitoringServer should push
   app metrics to PolicyManager (push model) or PolicyManager should pull from etcd
   (pull model).
2. **Interface design and implementation**: Extend `policymanager.proto` and the
   `ReportNodeMetrics` RPC (or add a new RPC) to include app-level metrics alongside
   node-level metrics.
3. **E2E verification**: Validate that PolicyManager correctly triggers Reconcile
   actions based on app-level metric thresholds (FPS, latency, per-core CPU load).

## References

- Issue: [#518 – Define App Performance Metrics for Monitoring Collection](https://github.com/eclipse-pullpiri/pullpiri/issues/518)
- Discussion: [#509 – Stateful Cloud-Local Offloading & App Performance-based Reconcile](https://github.com/eclipse-pullpiri/pullpiri/discussions/509)
- Proto definitions: `src/common/proto/monitoringserver.proto`, `src/common/proto/policymanager.proto`
- MonitoringServer gRPC sender: `src/server/monitoringserver/src/grpc/sender.rs`
- PolicyManager gRPC receiver: `src/server/policymanager/src/grpc/receiver.rs`
