<!--
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
-->

# 모니터링 수집을 위한 App 성능 메트릭 정의

## 배경

App 성능 기반 Reconcile Use Case에서는 PolicyManager가 App 성능 메트릭을 기반으로
Reconcile 동작을 트리거할 수 있어야 합니다.

본 문서는 MonitoringServer가 수집하여 PolicyManager로 전달해야 하는
App 성능 메트릭을 정의합니다.

## 현황 분석

### 기존 전달 경로 (Node 수준 메트릭)

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

### 누락된 전달 경로 (App 수준 메트릭)

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

## 수집 대상 메트릭 확정

App 성능 기반 Reconcile Use Case에 활용할 메트릭을 다음과 같이 확정합니다.

### Node 수준 메트릭 (PolicyManager 전달 경로 존재)

| 메트릭 | 단위 | 수집 주기 | 수집 범위 | 비고 |
|--------|------|-----------|-----------|------|
| CPU 사용률 | % | 설정값 (`collection_interval`) | Node 단위 | 임계값 기반 Offloading 구현 완료 |
| 메모리 사용률 | % | 설정값 (`collection_interval`) | Node 단위 | 임계값 기반 Offloading 구현 완료 |
| 네트워크 수신량 (RX) | bytes/주기 | 설정값 (`collection_interval`) | Node 단위 | 마지막 수집 이후 델타 값 |
| 네트워크 송신량 (TX) | bytes/주기 | 설정값 (`collection_interval`) | Node 단위 | 마지막 수집 이후 델타 값 |
| 스토리지 읽기 | bytes/주기 | 설정값 (`collection_interval`) | Node 단위 | 마지막 수집 이후 델타 값 |
| 스토리지 쓰기 | bytes/주기 | 설정값 (`collection_interval`) | Node 단위 | 마지막 수집 이후 델타 값 |

### App 수준 메트릭 (PolicyManager 전달 경로 누락)

| 메트릭 | 단위 | 수집 주기 | 수집 범위 | 비고 |
|--------|------|-----------|-----------|------|
| FPS | frames/초 | Push (App에서 요청 시) | App 단위 (프로세스) | 렌더링/멀티미디어 워크로드에 활용 |
| 지연 시간 (Latency) | 밀리초 (ms) | Push (App에서 요청 시) | App 단위 (프로세스) | End-to-end 응답 시간 |
| 코어별 CPU 부하 | % / 코어 | Push (App에서 요청 시) | App 단위 (프로세스) | 코어 단위 CPU 사용량 세부 정보 |

## 갭 분석

| 항목 | 상태 |
|------|------|
| Node 수준 메트릭 → MonitoringServer | ✅ 구현 완료 (`SendNodeInfo` gRPC) |
| Node 수준 메트릭 → PolicyManager | ✅ 구현 완료 (`ReportNodeMetrics` gRPC) |
| App 수준 메트릭 → MonitoringServer | ✅ 구현 완료 (`SendStressMonitoringMetric` gRPC) |
| App 수준 메트릭 → etcd 저장 | ✅ 구현 완료 |
| **App 수준 메트릭 → PolicyManager** | ❌ **미구현 (전달 경로 없음)** |

## 다음 단계

본 문서는 메트릭 정의 선행 조건을 충족합니다. 이후 수행할 작업:

1. **수집 주체 결정**: MonitoringServer가 App 메트릭을 PolicyManager에 Push할지
   (Push 모델), PolicyManager가 etcd에서 직접 Pull할지(Pull 모델)를 결정합니다.
2. **인터페이스 설계 및 구현**: `policymanager.proto`와 `ReportNodeMetrics` RPC를
   확장하거나 신규 RPC를 추가하여 App 수준 메트릭을 포함하도록 합니다.
3. **E2E 검증**: PolicyManager가 App 수준 메트릭 임계값(FPS, Latency, 코어별 CPU
   부하 등)을 기반으로 Reconcile 동작을 올바르게 트리거하는지 검증합니다.

## 참고 자료

- 이슈: [#518 – Define App Performance Metrics for Monitoring Collection](https://github.com/eclipse-pullpiri/pullpiri/issues/518)
- 토론: [#509 – Stateful Cloud-Local Offloading & App Performance-based Reconcile](https://github.com/eclipse-pullpiri/pullpiri/discussions/509)
- Proto 정의: `src/common/proto/monitoringserver.proto`, `src/common/proto/policymanager.proto`
- MonitoringServer gRPC Sender: `src/server/monitoringserver/src/grpc/sender.rs`
- PolicyManager gRPC Receiver: `src/server/policymanager/src/grpc/receiver.rs`
