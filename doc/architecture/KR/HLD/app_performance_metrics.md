<!--
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
-->

# 모니터링 수집을 위한 App 성능 메트릭 정의 / App Performance Metrics for Monitoring Collection

## 배경 / Background

### 한국어

App 성능 기반 Reconcile Use Case에서는 PolicyManager가 App 성능 메트릭을 기반으로
Reconcile 동작을 트리거할 수 있어야 합니다.

본 문서는 MonitoringServer가 수집하여 PolicyManager로 전달해야 하는
App 성능 메트릭을 정의합니다.

### English

As part of the App Performance-based Reconcile use case, PolicyManager must be able to
trigger Reconcile actions based on application performance metrics.

This document defines the app performance metrics to be collected by MonitoringServer
and delivered to PolicyManager.

## 후보 메트릭 조사 / Candidate Metric Research

### 한국어

PolicyManager 트리거에 활용 가능한 메트릭 후보 목록과 실질적 활용 가능성 및
수집 난이도를 평가합니다. 후보 목록은 코드베이스에 이미 존재하는 메트릭뿐만 아니라
이슈에서 언급한 대표 예시를 넘어 폭넓게 검토하였습니다.

| 메트릭 | 단위 | 범위 | 활용 가능성 | 수집 난이도 | 비고 |
|--------|------|------|------------|------------|------|
| FPS | frames/초 | App 단위 | 높음 | 낮음 | `StressMonitoringMetric`에 이미 포함; App Data Provider가 Push |
| 지연 시간 (Latency) | ms | App 단위 | 높음 | 낮음 | `StressMonitoringMetric`에 이미 포함; End-to-end 응답 시간 |
| 코어별 CPU 부하 | % / 코어 | App 단위 | 높음 | 낮음 | `StressMonitoringMetric`에 이미 포함; 수집 이벤트마다 Push |
| CPU 사용률 (Node) | % | Node 단위 | 높음 | 낮음 | `NodeInfo`에 이미 포함; NodeAgent가 sysinfo로 수집 |
| 메모리 사용률 (Node) | % | Node 단위 | 높음 | 낮음 | `NodeInfo`에 이미 포함; NodeAgent가 sysinfo로 수집 |
| 네트워크 I/O (Node) | bytes/주기 | Node 단위 | 높음 | 낮음 | `NodeInfo`에 이미 포함; 마지막 수집 이후 델타 값 |
| 스토리지 I/O (Node) | bytes/주기 | Node 단위 | 높음 | 낮음 | `NodeInfo`에 이미 포함; 마지막 수집 이후 델타 값 |
| GPU 사용률 | % | Node/App 단위 | 보통 | 보통 | 미수집; NVML 등 벤더 API 또는 sysfs 필요 |
| 프레임 드롭 수 | count/주기 | App 단위 | 높음 | 낮음 | FPS 델타로 도출 가능; App Data Provider가 페이로드에 포함 가능 |
| 지연 시간 지터 (Jitter) | ms | App 단위 | 보통 | 낮음 | 연속 Latency 샘플의 분산으로 도출 가능 |
| 처리량 (Throughput) | req/s 또는 msg/s | App 단위 | 보통 | 보통 | App에서 노출 필요; 표준 인터페이스 미존재 |
| 오류/결함 발생률 | count/주기 | App 단위 | 보통 | 보통 | App에서 노출 필요; 신뢰성 기반 Reconcile에 활용 가능 |
| CPU 온도 | °C | Node 단위 | 낮음 | 보통 | sysfs thermal zone에서 접근 가능; 플랫폼 의존적 |
| 전력 소비량 | watts | Node 단위 | 낮음 | 높음 | 플랫폼 의존적 (x86 RAPL, ARM 벤더 API); 차량 ECU에서 제한적 |
| 컨텍스트 스위치 횟수 | count/s | Node/App 단위 | 낮음 | 보통 | `/proc/stat`에서 접근 가능; Reconcile 결정에 직접적 활용 가치 낮음 |
| 실시간 데드라인 위반 | count/주기 | App 단위 | 낮음 | 높음 | RT-Linux 또는 RTOS 통합 필요; 현 아키텍처 범위 외 |

**활용 가능성 기준:**
- **높음**: 코드베이스에 이미 수집 수단이 존재하거나 간단히 도출 가능; PolicyManager 전달에 소요 공수가 낮음
- **보통**: 기술적으로 접근 가능하나 신규 수집 통합 또는 App 측 계측 작업 필요
- **낮음**: 플랫폼 의존적이거나, 상당한 인프라 변경이 필요하거나, Reconcile Use Case에서 직접적 활용 가치가 낮음

### English

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

**Feasibility levels:**
- **High**: Metric source already exists in the codebase or is trivially derivable; low engineering effort to forward to PolicyManager.
- **Medium**: Metric is technically accessible but requires new collection integration or App-side instrumentation.
- **Low**: Metric is platform-specific, requires significant infrastructure changes, or has limited direct value for the Reconcile use case.

## 현황 분석 / Current State

### 기존 전달 경로 (Node 수준 메트릭) / Existing Delivery Path (Node-Level Metrics)

#### 한국어

`policymanager.proto`에 정의된 `ReportNodeMetrics` gRPC를 통해
MonitoringServer → PolicyManager 간 전달 경로가 이미 존재합니다.
현재 이 경로는 `NodeInfo` 메시지의 **Node 수준** 메트릭을 전달합니다:

| 메트릭 | 타입 | 단위 | 수집 범위 |
|--------|------|------|-----------|
| `cpu_usage` | `double` | % (0–100) | Node 단위 |
| `cpu_count` | `uint64` | 개수 | Node 단위 |
| `gpu_count` | `uint64` | 개수 | Node 단위 |
| `used_memory` | `uint64` | bytes | Node 단위 |
| `total_memory` | `uint64` | bytes | Node 단위 |
| `mem_usage` | `double` | % (0–100) | Node 단위 |
| `rx_bytes` | `uint64` | bytes/수집주기 | Node 단위 |
| `tx_bytes` | `uint64` | bytes/수집주기 | Node 단위 |
| `read_bytes` | `uint64` | bytes/수집주기 | Node 단위 |
| `write_bytes` | `uint64` | bytes/수집주기 | Node 단위 |

Node 수준 메트릭은 NodeAgent가 `MetricsConfig`의 `collection_interval` 설정값에
따라 주기적으로 수집합니다 (`src/agent/nodeagent/src/config.rs` 참조).

#### English

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

### 누락된 전달 경로 (App 수준 메트릭) / Missing Delivery Path (App-Level Metrics)

#### 한국어

App 수준 메트릭은 `monitoringserver.proto`의 `SendStressMonitoringMetric` gRPC를 통해
MonitoringServer에 수신되고 etcd에 저장되지만, **PolicyManager로는 전달되지 않습니다**.
`StressMonitoringMetric` JSON 페이로드에 포함된 필드:

| 메트릭 | 타입 | 단위 | 수집 범위 |
|--------|------|------|-----------|
| `fps` | `f64` | frames/초 | App 단위 (프로세스) |
| `latency` | `u64` | 밀리초 (ms) | App 단위 (프로세스) |
| `cpu_loads[].load` | `f64` | % (0–100) / 코어 | App 단위 (프로세스) |
| `process_name` | `string` | — | App 식별자 |
| `pid` | `u32` | — | App 식별자 |
| `core_masking` | `string` (선택) | bitmask | App 단위 |
| `core_count` | `u32` (선택) | 개수 | App 단위 |

App 수준 메트릭은 App Data Provider에서 필요 시 Push 방식으로 전송되며,
MonitoringServer 측에서 고정 수집 주기를 강제하지 않습니다.

#### English

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

## 수집 대상 메트릭 확정 / Confirmed Metrics for PolicyManager Triggering

### Node 수준 메트릭 (PolicyManager 전달 경로 존재) / Node-Level Metrics (already delivered to PolicyManager)

#### 한국어

| 메트릭 | 단위 | 수집 주기 | 수집 범위 | 비고 |
|--------|------|-----------|-----------|------|
| CPU 사용률 | % | 설정값 (`collection_interval`) | Node 단위 | 임계값 기반 Offloading 구현 완료 |
| 메모리 사용률 | % | 설정값 (`collection_interval`) | Node 단위 | 임계값 기반 Offloading 구현 완료 |
| 네트워크 수신량 (RX) | bytes/주기 | 설정값 (`collection_interval`) | Node 단위 | 마지막 수집 이후 델타 값 |
| 네트워크 송신량 (TX) | bytes/주기 | 설정값 (`collection_interval`) | Node 단위 | 마지막 수집 이후 델타 값 |
| 스토리지 읽기 | bytes/주기 | 설정값 (`collection_interval`) | Node 단위 | 마지막 수집 이후 델타 값 |
| 스토리지 쓰기 | bytes/주기 | 설정값 (`collection_interval`) | Node 단위 | 마지막 수집 이후 델타 값 |

#### English

| Metric | Unit | Collection Interval | Scope | Notes |
|--------|------|---------------------|-------|-------|
| CPU Usage | % | Configurable (`collection_interval`) | Per Node | Threshold-based offloading already implemented |
| Memory Usage | % | Configurable (`collection_interval`) | Per Node | Threshold-based offloading already implemented |
| Network RX | bytes/interval | Configurable (`collection_interval`) | Per Node | Delta since last collection |
| Network TX | bytes/interval | Configurable (`collection_interval`) | Per Node | Delta since last collection |
| Storage Read | bytes/interval | Configurable (`collection_interval`) | Per Node | Delta since last collection |
| Storage Write | bytes/interval | Configurable (`collection_interval`) | Per Node | Delta since last collection |

### App 수준 메트릭 (PolicyManager 전달 경로 누락) / App-Level Metrics (delivery path to PolicyManager is missing)

#### 한국어

위 후보 평가에서 활용 가능성이 높음으로 분류된 메트릭을 우선 확정합니다.

| 메트릭 | 단위 | 수집 주기 | 수집 범위 | 비고 |
|--------|------|-----------|-----------|------|
| FPS | frames/초 | Push (App에서 요청 시) | App 단위 (프로세스) | 렌더링/멀티미디어 워크로드에 활용 |
| 지연 시간 (Latency) | 밀리초 (ms) | Push (App에서 요청 시) | App 단위 (프로세스) | End-to-end 응답 시간 |
| 코어별 CPU 부하 | % / 코어 | Push (App에서 요청 시) | App 단위 (프로세스) | 코어 단위 CPU 사용량 세부 정보 |
| 프레임 드롭 수 | count/주기 | Push (App에서 요청 시) | App 단위 (프로세스) | FPS 델타로 도출 가능; 렌더링 품질 저하 감지 |
| 지연 시간 지터 (Jitter) | 밀리초 (ms) | Push (App에서 요청 시) | App 단위 (프로세스) | 연속 Latency 샘플의 분산으로 도출 가능 |

활용 가능성이 보통 또는 낮음으로 분류된 메트릭(GPU 사용률, 처리량, 오류 발생률,
온도, 전력 소비량, 컨텍스트 스위치 횟수, 데드라인 위반)은 추가 가능성 검토 및
플랫폼 가용성 확인 후 차후 반복 작업에서 검토합니다.

#### English

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

## 갭 분석 / Gap Analysis

| 항목 / Item | 상태 / Status |
|-------------|---------------|
| Node 수준 메트릭 → MonitoringServer / Node-level metrics → MonitoringServer | ✅ 구현 완료 / Implemented (`SendNodeInfo` gRPC) |
| Node 수준 메트릭 → PolicyManager / Node-level metrics → PolicyManager | ✅ 구현 완료 / Implemented (`ReportNodeMetrics` gRPC) |
| App 수준 메트릭 → MonitoringServer / App-level metrics → MonitoringServer | ✅ 구현 완료 / Implemented (`SendStressMonitoringMetric` gRPC) |
| App 수준 메트릭 → etcd 저장 / App-level metrics stored in etcd | ✅ 구현 완료 / Implemented |
| **App 수준 메트릭 → PolicyManager / App-level metrics → PolicyManager** | ❌ **미구현 / Not implemented (전달 경로 없음 / missing delivery path)** |

## 다음 단계 / Next Steps

### 한국어

본 문서는 메트릭 정의 선행 조건을 충족합니다. 이후 수행할 작업:

1. **수집 주체 결정**: MonitoringServer가 App 메트릭을 PolicyManager에 Push할지
   (Push 모델), PolicyManager가 etcd에서 직접 Pull할지(Pull 모델)를 결정합니다.
2. **인터페이스 설계 및 구현**: `policymanager.proto`와 `ReportNodeMetrics` RPC를
   확장하거나 신규 RPC를 추가하여 App 수준 메트릭을 포함하도록 합니다.
3. **E2E 검증**: PolicyManager가 App 수준 메트릭 임계값(FPS, Latency, 코어별 CPU
   부하 등)을 기반으로 Reconcile 동작을 올바르게 트리거하는지 검증합니다.

### English

This document satisfies the metric definition prerequisite. The following tasks remain:

1. **Collection method decision**: Determine whether MonitoringServer should push
   app metrics to PolicyManager (push model) or PolicyManager should pull from etcd
   (pull model).
2. **Interface design and implementation**: Extend `policymanager.proto` and the
   `ReportNodeMetrics` RPC (or add a new RPC) to include app-level metrics alongside
   node-level metrics.
3. **E2E verification**: Validate that PolicyManager correctly triggers Reconcile
   actions based on app-level metric thresholds (FPS, latency, per-core CPU load).

## 참고 자료 / References

- 이슈 / Issue: [#518 – Define App Performance Metrics for Monitoring Collection](https://github.com/eclipse-pullpiri/pullpiri/issues/518)
- 토론 / Discussion: [#509 – Stateful Cloud-Local Offloading & App Performance-based Reconcile](https://github.com/eclipse-pullpiri/pullpiri/discussions/509)
- Proto 정의 / Proto definitions: `src/common/proto/monitoringserver.proto`, `src/common/proto/policymanager.proto`
- MonitoringServer gRPC Sender: `src/server/monitoringserver/src/grpc/sender.rs`
- PolicyManager gRPC Receiver: `src/server/policymanager/src/grpc/receiver.rs`
