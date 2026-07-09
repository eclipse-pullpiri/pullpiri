<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# Custom Metrics 수집 경로 설계 (LLD)

**문서 번호**: Pullpiri-MONITORING-CUSTOM-LLD-2025-001-KR  
**버전**: 1.0  
**작성일**: 2025-07-08  
**작성자**: Pullpiri 팀  
**분류**: LLD (Low-Level Design)  
**관련 이슈**: #520

---

## 1. 개요

### 1.1 목적

본 문서는 Pullpiri MonitoringServer의 현재 Custom Metrics 수집 구조를 분석하고, 애플리케이션 특화 성능 메트릭(FPS, Latency 등)을 PolicyManager까지 전달하는 설계 방향을 제시합니다.

### 1.2 범위

- 현재 MonitoringServer의 메트릭 수집 구조 분석
- 기존 Custom Metrics 수집 경로 확인
- 기존 경로와 PolicyManager 전달 경로 간 갭(Gap) 분석
- Custom Metrics 전달 경로 완성을 위한 설계 방향 제시
- FPS를 예시 메트릭으로 사용한 인터페이스 정의

---

## 2. 현재 아키텍처 분석

### 2.1 시스템 메트릭 수집 (기존 — 완전 구현됨)

NodeAgent는 다음과 같은 시스템 수준 메트릭을 gRPC를 통해 MonitoringServer에 전송합니다.

| 메트릭 유형 | gRPC 메시지 | 필드 |
|------------|------------|------|
| 노드 리소스 | `NodeInfo` | cpu_usage, mem_usage, gpu_count, 네트워크 I/O, 디스크 I/O |
| 컨테이너 상태 | `ContainerList` | 컨테이너 ID, 이름, 이미지, 상태, 어노테이션, 통계 |

**흐름도:**
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
PolicyManager → 임계값 평가 → 워크로드 오프로딩
```

### 2.2 Custom Metrics 수집 (기존 — 부분 구현됨)

Custom Metrics 수집 경로가 **이미 코드베이스에 존재**합니다. `StressMonitoringMetric`이라는 gRPC 메시지를 사용하며, FPS, Latency, 코어별 CPU 부하를 지원하는 JSON 페이로드를 전달합니다.

**관련 소스 파일:**

| 파일 | 역할 |
|------|------|
| `src/common/proto/monitoringserver.proto` | `SendStressMonitoringMetric` RPC 및 `StressMonitoringMetric` 메시지 프로토 정의 |
| `src/server/monitoringserver/src/grpc/receiver.rs` | gRPC 핸들러, JSON 유효성 검사, 채널 전달 |
| `src/server/monitoringserver/src/manager.rs` | `process_stress_requests()` 루프, etcd 저장 |
| `src/server/monitoringserver/src/etcd_storage.rs` | `store_stress_metric_json()`, `get_all_stress_metrics()` |

**기존 프로토 정의:**

```protobuf
// monitoringserver.proto
service MonitoringServerConnection {
  rpc SendContainerList (ContainerList) returns (SendContainerListResponse);
  rpc SendNodeInfo (NodeInfo) returns (SendNodeInfoResponse);
  rpc SendStressMonitoringMetric (StressMonitoringMetric) returns (StressMonitoringMetricResponse);
}

// JSON 기반 Custom Metrics 메시지
message StressMonitoringMetric {
  string json = 1; // JSON: process_name, pid, core_masking, core_count, fps, latency, cpu_loads
}
```

**기존 JSON 스키마 (receiver.rs에서 유효성 검사):**

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

**기존 흐름 (App → etcd):**
```
애플리케이션 (Data Provider)
  │
  │ gRPC SendStressMonitoringMetric (JSON 페이로드)
  ▼
MonitoringServer (send_stress_monitoring_metric)
  │  JSON 유효성 검사
  │  tx_stress.send(json)
  ▼
process_stress_requests() 루프
  │
  ▼
etcd (/pullpiri/metrics/stress/{process_name}/{pid})
```

**etcd 저장 경로:** `/pullpiri/metrics/stress/{process_name}/{pid}`

### 2.3 갭(Gap) 분석

다음 표는 구현된 항목과 누락된 항목을 정리합니다.

| 경로 구간 | 상태 | 비고 |
|---|---|---|
| App → MonitoringServer (gRPC `SendStressMonitoringMetric`) | ✅ 구현됨 | 프로토 정의, 핸들러 존재 |
| MonitoringServer → etcd (저장) | ✅ 구현됨 | `store_stress_metric_json()` 존재 |
| MonitoringServer → PolicyManager (Custom Metrics 전달) | ❌ 미구현 | `ReportNodeMetricsRequest`에 `NodeInfo`만 포함 |
| PolicyManager Custom Metrics 임계값 평가 | ❌ 미구현 | CPU/메모리 임계값만 구현됨 |
| SettingsService REST API를 통한 Custom Metrics 조회 | ❌ 미구현 | `/api/v1/metrics/custom` 엔드포인트 없음 |
| NodeAgent 연동 여부 | ➖ 불필요 | 직접 App→MonitoringServer 경로가 권장됨 |

---

## 3. 설계 방향

### 3.1 전달 방식: Push vs Pull

**권장: Push 모델 (이미 구현됨)**

기존 `SendStressMonitoringMetric` RPC는 애플리케이션이 MonitoringServer에 능동적으로 메트릭을 전송하는 Push 모델을 사용합니다. FPS, Latency와 같이 측정 즉시 전달이 필요한 실시간 성능 메트릭에 적합합니다.

Pull 모델(MonitoringServer가 앱 엔드포인트를 폴링)은 시간에 민감한 데이터에서 불필요한 복잡성과 지연을 초래합니다.

### 3.2 목표 아키텍처 (확장 후)

Custom Metrics의 전체 목표 흐름은 다음과 같습니다.

```
애플리케이션 (Data Provider)
  │
  │ gRPC SendStressMonitoringMetric
  │ { process_name, pid, fps, latency, cpu_loads, ... }
  ▼
MonitoringServer (grpc/receiver.rs)
  │  JSON 유효성 검사 → 채널 전달
  ▼
process_stress_requests() (manager.rs)
  │
  ├─► etcd /pullpiri/metrics/stress/{process}/{pid}    [기존]
  │
  └─► [신규] report_custom_metrics_to_policy_manager()
        │
        │ gRPC ReportCustomMetrics
        ▼
      PolicyManager
        │  FPS/Latency 임계값 평가
        │  임계값 초과 시 정책 액션 트리거
        ▼
      ActionController → 워크로드 오프로딩 / 스케일링
```

### 3.3 인터페이스 설계

#### 3.3.1 옵션 A: `ReportNodeMetricsRequest` 확장 (최소 변경으로 권장)

기존 `policymanager.proto`를 확장하여 노드 리포트에 Custom Metrics를 포함합니다.

```protobuf
// policymanager.proto (확장)
message ReportNodeMetricsRequest {
  monitoringserver.NodeInfo node_info = 1;
  repeated RunningContainer running_containers = 2;
  repeated CustomMetric custom_metrics = 3;   // 신규
}

message CustomMetric {
  string process_name = 1;  // 메트릭 보고 프로세스 식별
  string container_name = 2; // 연관 컨테이너 (정책 조회용)
  string metric_name = 3;   // 예: "fps", "latency_ms"
  string metric_unit = 4;   // 예: "frames_per_second", "milliseconds", "percent"
  double value = 5;         // 현재 측정값
  int64 timestamp_ms = 6;   // 측정 시각 (에포크 ms)
}
```

이 방식은 `NodeInfo`가 수신될 때마다 호출되는 기존 `ReportNodeMetrics` 호출에 Custom Metrics를 함께 배치(batch)하여 전송합니다.

#### 3.3.2 옵션 B: 신규 gRPC 메서드 `ReportCustomMetrics`

Custom Metrics 보고를 위한 별도 RPC를 추가하여 시스템 메트릭 보고와 분리합니다.

```protobuf
// policymanager.proto (대안)
service PolicyManagerConnection {
  rpc CheckNodePolicy(CheckNodePolicyRequest) returns (CheckNodePolicyResponse);
  rpc ReportNodeMetrics(ReportNodeMetricsRequest) returns (ReportNodeMetricsResponse);
  rpc ReportCustomMetrics(ReportCustomMetricsRequest) returns (ReportCustomMetricsResponse);  // 신규
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

이 방식은 Custom Metrics를 수신하는 즉시 보고할 수 있으므로 `NodeInfo` 업데이트를 기다릴 필요가 없습니다. 더 높은 응답성을 제공하지만 MonitoringServer에 새로운 gRPC 클라이언트 호출 경로가 필요합니다.

**트레이드오프 비교:**

| 기준 | 옵션 A (기존 확장) | 옵션 B (신규 RPC) |
|---|---|---|
| 구현 난이도 | 낮음 | 중간 |
| 실시간 응답성 | NodeInfo 주기에 의존 | 즉시 |
| 결합도 | Custom Metrics가 노드 메트릭과 결합 | 분리됨 |
| 확장성 | NodeInfo 주기에 제한 | 유연 |
| 권장 시나리오 | MVP / 초기 구현 | 장기 설계 |

#### 3.3.3 메트릭 등록 및 식별

Custom Metrics에는 정확한 정책 조회를 위해 다음과 같은 식별 필드가 포함되어야 합니다.

| 필드 | 설명 | 예시 |
|---|---|---|
| `process_name` | 메트릭을 보고하는 프로세스 이름 | `"camera_app"` |
| `container_name` | 어노테이션의 컨테이너 이름 | `"helloworld_camera"` |
| `metric_name` | 표준화된 메트릭 식별자 | `"fps"`, `"latency_ms"` |
| `metric_unit` | 측정 단위 | `"frames_per_second"`, `"milliseconds"` |
| `value` | 측정된 수치 값 | `58.7` |
| `timestamp_ms` | 측정 시각 (에포크 ms) | `1720435200000` |

### 3.4 FPS 메트릭 예시

**FPS 메트릭 보고 흐름:**

1. 카메라 앱이 프레임 레이트 측정: `fps = 45.2`
2. 앱이 JSON과 함께 `SendStressMonitoringMetric` 호출:
   ```json
   {
     "process_name": "camera_app",
     "pid": 1234,
     "fps": 45.2,
     "latency": 22,
     "cpu_loads": [{"core_id": 0, "load": 78.5}]
   }
   ```
3. MonitoringServer가 유효성 검사, etcd 저장, 그리고 (신규) PolicyManager에 보고
4. PolicyManager가 `fps < 임계값` 확인 (예: 정책에서 `fps >= 60` 요구)
5. 임계값 초과 시 → 더 높은 성능의 노드로 워크로드 오프로딩 트리거

**정책 정의 구조 (제안):**

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

### 3.5 NodeAgent 연동 여부 검토

Custom Metrics 수집에서 NodeAgent 연동은 **불필요**합니다. 현재 설계는 애플리케이션과 MonitoringServer 간의 직접 통신을 지원합니다.

- NodeAgent는 시스템 수준 메트릭(CPU, 메모리, 컨테이너 상태) 담당
- 앱 커스텀 메트릭은 애플리케이션이 직접 보고
- 이를 통해 애플리케이션 수준의 계측(instrumentation)이 NodeAgent에 결합되는 것을 방지

향후 직접 gRPC 호출이 불가능한 앱을 위해 NodeAgent를 프록시로 활용해야 하는 경우, 기존 `SendStressMonitoringMetric` 인터페이스를 사용하는 NodeAgent 릴레이 경로를 추가할 수 있습니다.

---

## 4. SettingsService REST API 확장

기존 REST API를 통해 Custom Metrics를 조회할 수 있도록 다음 엔드포인트를 추가합니다.

```
GET /api/v1/metrics/custom
GET /api/v1/metrics/custom/{process_name}
```

**응답 형식:**

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

이 엔드포인트는 이미 구현된 `get_all_stress_metrics()` 함수를 사용하여 기존 etcd 경로 `/pullpiri/metrics/stress/`에서 읽어옵니다.

---

## 5. 요약 및 권고사항

### 5.1 현재 상태

- ✅ Custom Metrics 수집 경로가 **이미 코드베이스에 존재**합니다
- ✅ 앱은 gRPC를 통해 FPS, Latency, CPU 부하 메트릭을 MonitoringServer에 전송 가능합니다
- ✅ 메트릭은 유효성 검사 후 etcd에 저장됩니다
- ❌ Custom Metrics가 PolicyManager에 **아직 전달되지 않습니다**
- ❌ PolicyManager가 Custom Metrics 임계값을 **평가할 수 없습니다**
- ❌ Custom Metrics 조회를 위한 REST API 엔드포인트가 없습니다

### 5.2 권장 후속 구현 작업 (별도 이슈)

1. **`policymanager.proto` 확장**: `ReportNodeMetricsRequest`에 `CustomMetric` 포함 (옵션 A)
2. **`MonitoringServerManager::process_stress_requests()` 확장**: `report_custom_metrics_to_policy_manager()` 호출 추가
3. **`PolicyManagerGrpcServer::report_node_metrics()` 확장**: Custom Metrics 임계값 평가 로직 추가
4. **SettingsService REST 엔드포인트 추가**: `GET /api/v1/metrics/custom`

### 5.3 설계 결정 사항 (담당자 검토 필요)

- PolicyManager gRPC 인터페이스의 **옵션 A vs 옵션 B** 선택
- Custom Metrics의 PolicyManager 전달 방식: **배치(batch) vs 즉시(immediate)**
- 메트릭 페이로드 방식: **구조화(structured) vs JSON** (현재 JSON 방식은 유연하지만 타입이 약함)
- **메트릭 명명 규칙** 표준화 (예: `fps` vs `frames_per_second`)

---

## 6. 참고 자료

- `src/common/proto/monitoringserver.proto` — `StressMonitoringMetric` 메시지 및 `SendStressMonitoringMetric` RPC
- `src/common/proto/policymanager.proto` — `ReportNodeMetrics` RPC 및 요청/응답 메시지
- `src/server/monitoringserver/src/grpc/receiver.rs` — gRPC 핸들러 구현
- `src/server/monitoringserver/src/manager.rs` — `process_stress_requests()` 메서드
- `src/server/monitoringserver/src/etcd_storage.rs` — `store_stress_metric_json()`, `get_all_stress_metrics()`
- `src/server/monitoringserver/src/grpc/sender.rs` — PolicyManager로 `report_node_metrics()` 전송
