<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# Custom Metrics Collection Path Design (LLD)

**Document Number**: Pullpiri-MONITORING-CUSTOM-LLD-2025-001  
**Version**: 1.0  
**Date**: 2025-07-08  
**Author**: Pullpiri Team  
**Classification**: LLD (Low-Level Design)  
**Related Issue**: #520

---

## 1. Overview

### 1.1 Purpose

This document investigates the current custom metrics collection structure in Pullpiri's MonitoringServer and proposes a design for extending it to deliver application-specific performance metrics (e.g., FPS, Latency) to the PolicyManager.

### 1.2 Scope

- Analysis of the current MonitoringServer metric collection structure
- Identification of the existing custom metric collection path
- Gap analysis between the existing path and the required end-to-end path to PolicyManager
- Design direction for completing the custom metric forwarding path
- Interface definitions using FPS as the example metric

---

## 2. Current Architecture Analysis

### 2.1 System Metrics Collection (Existing — Fully Implemented)

NodeAgent collects and sends the following system-level metrics to MonitoringServer via gRPC:

| Metric Type     | gRPC Message     | Fields                                              |
|-----------------|------------------|-----------------------------------------------------|
| Node Resources  | `NodeInfo`       | cpu_usage, mem_usage, gpu_count, rx/tx bytes, disk I/O |
| Container State | `ContainerList`  | container id, name, image, state, annotations, stats |

**Flow:**
```
NodeAgent
  │
  │ gRPC SendNodeInfo / SendContainerList
  ▼
MonitoringServer (MonitoringServerReceiver)
  │
  ├─► etcd (/pullpiri/metrics/nodes/{name})
  ├─► etcd (/pullpiri/metrics/containers/{id})
  │
  │ gRPC ReportNodeMetrics
  ▼
PolicyManager → threshold evaluation → workload offloading
```

### 2.2 Custom Metrics Collection (Existing — Partially Implemented)

A dedicated custom metrics collection path **already exists** in the codebase. It uses a gRPC message called `StressMonitoringMetric` that carries a JSON payload supporting FPS, Latency, and per-core CPU loads.

**Relevant source files:**

| File | Role |
|------|------|
| `src/common/proto/monitoringserver.proto` | Proto definition for `SendStressMonitoringMetric` RPC and `StressMonitoringMetric` message |
| `src/server/monitoringserver/src/grpc/receiver.rs` | gRPC handler, JSON validation, channel forwarding |
| `src/server/monitoringserver/src/manager.rs` | `process_stress_requests()` loop, etcd persistence |
| `src/server/monitoringserver/src/etcd_storage.rs` | `store_stress_metric_json()`, `get_all_stress_metrics()` |

**Existing proto definition:**

```protobuf
// monitoringserver.proto
service MonitoringServerConnection {
  rpc SendContainerList (ContainerList) returns (SendContainerListResponse);
  rpc SendNodeInfo (NodeInfo) returns (SendNodeInfoResponse);
  rpc SendStressMonitoringMetric (StressMonitoringMetric) returns (StressMonitoringMetricResponse);
}

// JSON-based custom metric message
message StressMonitoringMetric {
  string json = 1; // JSON: process_name, pid, core_masking, core_count, fps, latency, cpu_loads
}
```

**Existing JSON schema (validated in receiver.rs):**

```json
{
  "process_name": "example_process",
  "pid": 12345,
  "core_masking": "0x0000F",
  "core_count": 20,
  "fps": 58.7,
  "latency": 38,
  "cpu_loads": [
    { "core_id": 0, "load": 23.5 },
    { "core_id": 1, "load": 45.2 }
  ]
}
```

**Existing flow (App → etcd):**
```
Application (Data Provider)
  │
  │ gRPC SendStressMonitoringMetric (JSON payload)
  ▼
MonitoringServer (send_stress_monitoring_metric)
  │  JSON validation
  │  tx_stress.send(json)
  ▼
process_stress_requests() loop
  │
  ▼
etcd (/pullpiri/metrics/stress/{process_name}/{pid})
```

**etcd storage path:** `/pullpiri/metrics/stress/{process_name}/{pid}`

### 2.3 Gap Analysis

The following table summarizes what is implemented and what is missing:

| Path Segment | Status | Notes |
|---|---|---|
| App → MonitoringServer (gRPC `SendStressMonitoringMetric`) | ✅ Implemented | Proto defined, handler exists |
| MonitoringServer → etcd (storage) | ✅ Implemented | `store_stress_metric_json()` exists |
| MonitoringServer → PolicyManager (custom metrics forwarding) | ❌ Missing | `ReportNodeMetricsRequest` only contains `NodeInfo` |
| PolicyManager custom metric threshold evaluation | ❌ Missing | Only CPU/memory thresholds implemented |
| SettingsService REST API for custom metrics | ❌ Missing | No `/api/v1/metrics/custom` endpoint |
| NodeAgent involvement | ➖ Not required | Direct App→MonitoringServer path is preferred (push model) |

---

## 3. Design Direction

### 3.1 Delivery Model: Push vs Pull

**Recommendation: Push model (already implemented)**

The existing `SendStressMonitoringMetric` RPC uses a push model where the application actively sends metrics to MonitoringServer. This is appropriate for real-time performance metrics such as FPS and Latency, which must be delivered immediately when measured.

A pull model (MonitoringServer polling the app's endpoint) would introduce unnecessary complexity and latency for time-sensitive data.

### 3.2 Proposed Extended Architecture

The complete target flow for custom metrics is:

```
Application (Data Provider)
  │
  │ gRPC SendStressMonitoringMetric
  │ { process_name, pid, fps, latency, cpu_loads, ... }
  ▼
MonitoringServer (grpc/receiver.rs)
  │  validate JSON → forward to channel
  ▼
process_stress_requests() (manager.rs)
  │
  ├─► etcd /pullpiri/metrics/stress/{process}/{pid}    [existing]
  │
  └─► [NEW] report_custom_metrics_to_policy_manager()
        │
        │ gRPC ReportCustomMetrics
        ▼
      PolicyManager
        │  evaluate FPS/Latency thresholds
        │  trigger policy action if threshold exceeded
        ▼
      ActionController → workload offloading / scaling
```

### 3.3 Interface Design

#### 3.3.1 Option A: Extend `ReportNodeMetricsRequest` (Recommended for minimal change)

Extend the existing `policymanager.proto` to include custom metrics in the node report.

```protobuf
// policymanager.proto (extended)
message ReportNodeMetricsRequest {
  monitoringserver.NodeInfo node_info = 1;
  repeated RunningContainer running_containers = 2;
  repeated CustomMetric custom_metrics = 3;   // NEW
}

message CustomMetric {
  string process_name = 1;  // Identifies the reporting application
  string container_name = 2; // Associated container (for policy lookup)
  string metric_name = 3;   // e.g., "fps", "latency", "cpu_load_avg"
  string metric_unit = 4;   // e.g., "frames_per_second", "milliseconds", "percent"
  double value = 5;         // Current measured value
  int64 timestamp_ms = 6;   // Measurement time (epoch ms)
}
```

This approach piggybacks on the existing `ReportNodeMetrics` call triggered each time a `NodeInfo` arrives. Custom metrics accumulated since the last `NodeInfo` are batched and sent together.

#### 3.3.2 Option B: New gRPC Method `ReportCustomMetrics`

Add a dedicated RPC for custom metric reporting, decoupled from system metric reporting.

```protobuf
// policymanager.proto (alternative)
service PolicyManagerConnection {
  rpc CheckNodePolicy(CheckNodePolicyRequest) returns (CheckNodePolicyResponse);
  rpc ReportNodeMetrics(ReportNodeMetricsRequest) returns (ReportNodeMetricsResponse);
  rpc ReportCustomMetrics(ReportCustomMetricsRequest) returns (ReportCustomMetricsResponse);  // NEW
}

message ReportCustomMetricsRequest {
  string node_name = 1;
  repeated CustomMetric metrics = 2;
}

message ReportCustomMetricsResponse {
  bool processed = 1;
  string message = 2;
}
```

This approach allows custom metrics to be reported immediately when received, without waiting for a `NodeInfo` update. It is more responsive but requires a new gRPC client call path in MonitoringServer.

**Trade-off comparison:**

| Criterion | Option A (Extend existing) | Option B (New RPC) |
|---|---|---|
| Implementation effort | Low | Medium |
| Real-time responsiveness | Dependent on NodeInfo frequency | Immediate |
| Coupling | Custom metrics tied to node metrics | Decoupled |
| Extensibility | Limited by NodeInfo periodicity | Flexible |
| Recommended for | MVP / initial implementation | Long-term design |

#### 3.3.3 Metric Registration and Identification

Custom metrics should include the following identification fields to enable accurate policy lookup:

| Field | Description | Example |
|---|---|---|
| `process_name` | Name of the process reporting the metric | `"camera_app"` |
| `container_name` | Container name from annotation | `"helloworld_camera"` |
| `metric_name` | Standardized metric identifier | `"fps"`, `"latency_ms"` |
| `metric_unit` | Unit of measurement | `"frames_per_second"`, `"milliseconds"` |
| `value` | Measured numeric value | `58.7` |
| `timestamp_ms` | Epoch milliseconds of measurement | `1720435200000` |

### 3.4 FPS Metric Example

**FPS metric reporting flow:**

1. Camera application measures frame rate: `fps = 45.2`
2. Application calls `SendStressMonitoringMetric` with JSON:
   ```json
   {
     "process_name": "camera_app",
     "pid": 1234,
     "fps": 45.2,
     "latency": 22,
     "cpu_loads": [{"core_id": 0, "load": 78.5}]
   }
   ```
3. MonitoringServer validates, stores to etcd, and (new) reports to PolicyManager
4. PolicyManager checks if `fps < threshold` (e.g., policy requires `fps >= 60`)
5. If threshold exceeded → trigger workload offloading to a higher-performance node

**Policy definition structure (proposed):**

```yaml
apiVersion: pullpiri.io/v1
kind: Policy
metadata:
  name: camera_fps_policy
spec:
  thresholds:
    - metric: fps
      process: camera_app
      operator: less_than
      value: 60.0
      action: offload
      target_node: HPC
```

### 3.5 NodeAgent Involvement

NodeAgent involvement for custom metrics collection is **not required**. The current design supports direct application-to-MonitoringServer communication:

- NodeAgent is responsible for system-level metrics (CPU, memory, container state)
- Custom application metrics are reported directly by the application
- This avoids coupling application-level instrumentation to NodeAgent

If future requirements need NodeAgent as a proxy (e.g., for apps that cannot make direct gRPC calls), a NodeAgent relay path can be added using the existing `SendStressMonitoringMetric` interface with NodeAgent as the sender.

---

## 4. SettingsService REST API Extension

To expose custom metrics via the existing REST API, add the following endpoint:

```
GET /api/v1/metrics/custom
GET /api/v1/metrics/custom/{process_name}
```

**Response format:**

```json
[
  {
    "process_name": "camera_app",
    "pid": 1234,
    "fps": 45.2,
    "latency": 22,
    "cpu_loads": [{"core_id": 0, "load": 78.5}],
    "timestamp": "2025-07-08T11:00:00Z"
  }
]
```

This endpoint reads from the existing etcd path `/pullpiri/metrics/stress/` using the already-implemented `get_all_stress_metrics()` function.

---

## 5. Summary and Recommendations

### 5.1 Current State

- ✅ A custom metrics collection path **already exists** in the codebase
- ✅ Applications can send FPS, Latency, and CPU load metrics to MonitoringServer via gRPC
- ✅ Metrics are validated and stored in etcd
- ❌ Custom metrics are **not yet forwarded** to PolicyManager
- ❌ PolicyManager **cannot evaluate** custom metric thresholds
- ❌ No REST API endpoint for querying custom metrics

### 5.2 Recommended Next Steps (for separate implementation issues)

1. **Extend `policymanager.proto`** to include `CustomMetric` in `ReportNodeMetricsRequest` (Option A)
2. **Extend `MonitoringServerManager::process_stress_requests()`** to call `report_custom_metrics_to_policy_manager()`
3. **Extend `PolicyManagerGrpcServer::report_node_metrics()`** to evaluate custom metric thresholds
4. **Add REST endpoint** `GET /api/v1/metrics/custom` in SettingsService

### 5.3 Design Decision Points (for maintainer review)

- **Option A vs Option B** for PolicyManager gRPC interface
- **Batch vs immediate** forwarding of custom metrics to PolicyManager
- **Structured vs JSON** metric payload (current JSON approach is flexible but weakly typed)
- **Metric naming convention** standardization (e.g., `fps` vs `frames_per_second`)

---

## 6. References

- `src/common/proto/monitoringserver.proto` — `StressMonitoringMetric` message and `SendStressMonitoringMetric` RPC
- `src/common/proto/policymanager.proto` — `ReportNodeMetrics` RPC and request/response messages
- `src/server/monitoringserver/src/grpc/receiver.rs` — gRPC handler implementation
- `src/server/monitoringserver/src/manager.rs` — `process_stress_requests()` method
- `src/server/monitoringserver/src/etcd_storage.rs` — `store_stress_metric_json()`, `get_all_stress_metrics()`
- `src/server/monitoringserver/src/grpc/sender.rs` — `report_node_metrics()` to PolicyManager
