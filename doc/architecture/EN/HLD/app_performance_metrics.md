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

| Metric | Unit | Collection Interval | Scope | Notes |
|--------|------|---------------------|-------|-------|
| FPS | frames/second | Push (on demand from App) | Per App (process) | Useful for rendering/multimedia workloads |
| Latency | milliseconds | Push (on demand from App) | Per App (process) | End-to-end response time |
| Per-core CPU Load | % per core | Push (on demand from App) | Per App (process) | Granular CPU usage by core |

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
