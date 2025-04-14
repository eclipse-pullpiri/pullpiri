# ActionController

## 1. 목적 프롬프트
### 주요기능
ActionController는 FilterGateway로부터 특정 시나리오의 조건 충족 이벤트를 전달받아, 해당 시나리오의 Action과 Target 정보를 기반으로 Bluechi Controller API 또는 NodeAgent API를 호출하여 작업을 수행하는 모듈입니다.  
또한, ETCD에서 시나리오 정보를 읽어와 각 노드의 상태를 조정하며, Bluechi와 NodeAgent를 사용하는 노드를 구분하여 처리합니다. filtergateway와 statmaanger로부터 메세지를 전달받고 Bluechi와 nodeagent로 함수를 호출합니다. 노드 별로 알맞은 함수 호출을 담당합니다. 

### 주요 데이터 흐름
1. **초기화**: `settings.json` 파일에서 노드 정보를 읽어와 Bluechi 노드와 NodeAgent 노드를 구분합니다.
2. **시나리오 처리**: FilterGateway로부터 전달받은 시나리오 이름으로 ETCD에서 Action과 Target 정보를 조회합니다.
3. **작업 수행**: Action과 Target 정보를 기반으로 Bluechi API 또는 NodeAgent API를 호출하여 작업을 수행합니다.

1. **상태 관리**: 외부 모듈 StateManager로부터 시나리오(scenario), 현재상태(current), 기존스펙(desired) 상태 정보를 전달받습니다.
2. **상태 조정**: current와 desired를 상태를 비교해서 desired 상태로 current를 변경합니다.

---

## 2. 파일 정보
ActionController는 다음과 같은 파일들로 구성됩니다:

```
ActionController/
├── main.rs
├── manager.rs
├── grpc/
│   ├── mod.rs
│   ├── receiver.rs
│   └── sender.rs
├── runtime/
│   ├── mod.rs
│   ├── bluechi/
│   │   └── mod.rs
│   └── nodeagent/
│       └── mod.rs
```

- **main.rs**: 초기화 작업 수행.
- **manager.rs**: `settings.json` 파일에서 노드 정보를 읽어오고, 시나리오 정보를 처리하여 API 호출.
- **grpc/mod.rs**: gRPC 관련 모듈 정의.
- **grpc/receiver.rs**: FilterGateway 및 StateManager로부터 gRPC 메시지를 수신.
- **grpc/sender.rs**: nodeagent, policymanager로  gRPC 메시지 전송.
- **runtime/mod.rs**: Bluechi 및 NodeAgent 관련 작업 수행.
- **runtime/bluechi/mod.rs**: Bluechi 관련 API 호출.
- **runtime/nodeagent/mod.rs**: NodeAgent 관련 API 호출.

---

## 3. 주요 API 정보

### API : Initialize
- **API Name**: Initialize
- **File**: main.rs
- **Type**: function
- **Parameters**: None
- **Returns**: None
- **Description**: `settings.json` 파일에서 노드 정보를 읽어와 Bluechi 노드와 NodeAgent 노드를 구분하여 초기화합니다.

---

### API : TriggerAction
- **API Name**: trigger_action
- **File**: grpc/receiver.rs
- **Type**: grpc
- **Parameters**: scenario_name: string
- **Returns**: core::result::Result
- **Description**: FilterGateway로부터 전달받은 시나리오 데이터를 기반으로 ETCD에서 Action과 Target 정보를 조회하고, 작업을 수행합니다.
---

### API : Reconcile
- **API Name**: reconcile
- **File**: grpc/receiver.rs
- **Type**: grpc
- **Parameters**: scenario_name: string, current: i32, desired: i32
- **Returns**: core::result::Result
- **Description**: Statemanger로부터 전달받은 시나리오 데이터를 기반으로 scenario, current, desired 정보를 
확인하고, 보정 작업을 수행합니다.

---

### API : CheckPolicy
- **API Name**: check_policy
- **File**: grpc/sender.rs
- **Type**: grpc
- **Parameters**: scenario_name: string
- **Returns**: core::result::Result
- **Description**: 해당 시나리오가 수행 가능한지 policy를 확인합니다.

---

### API : HandleWorkload
- **API Name**: handle_workload
- **File**: grpc/sender.rs
- **Type**: grpc
- **Parameters**: workload_name: string, action: i32, description: string
- **Returns**: core::result::Result
- **Description**: nodeagent에서 수행 할 workload action의 내용을 전달 합니다
---

### API : CreateWorkload
- **API Name**: create_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: core::result::Result
- **Description**: ETCD에서 Systemd 파일 및 Pod YAML 파일을 읽어와 작업을 생성합니다. Bluechi, NodeAgent따라 적절한 API를 호출합니다.

---

### API : DeleteWorkload
- **API Name**: delete_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: core::result::Result
- **Description**: 기존 작업 파일을 삭제하고, Bluechi 또는 NodeAgent API를 호출하여 작업을 제거합니다.

---

### API : RestartWorkload
- **API Name**: restart_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: core::result::Result
- **Description**: Bluechi 또는 NodeAgent API를 호출하여 작업을 재실행합니다.

---

### API : PauseWorkload
- **API Name**: pause_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: core::result::Result
- **Description**: Bluechi 또는 NodeAgent API를 호출하여 작업을 일시정지합니다.

---

### API : StartWorkload
- **API Name**: start_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: core::result::Result
- **Description**: Bluechi 또는 NodeAgent API를 호출하여 작업을 시작합니다.

---

### API : StopWorkload
- **API Name**: stop_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: core::result::Result
- **Description**: Bluechi 또는 NodeAgent API를 호출하여 작업을 중지합니다.

---

## 4. 참고
- `Cargo.toml` 파일에 `[dependencies]` 섹션에 `common = {workspace = true}`를 추가합니다.
- `common/src/spec/scenario/mod.rs`를 참고하여 시나리오를 파싱합니다.
- `common/proto/actioncontroller.proto`를 기반으로 `grpc/receiver.rs`를 구현합니다.
- `common/proto/policymanager.proto`와 `common/proto/nodeagent.proto`를 기반으로 `grpc/sender.rs`를 구현합니다.
- `example/resource/bms-performance.yaml` 파일을 참고하여 DDS 토픽을 구독합니다.
- `tokio::sync::mpsc`를 사용하여 모듈 간 통신 채널을 구현합니다.

---

## 5. 추가 요구사항
1. 주요 API가 올바른 파일에 구현되었는지 확인합니다.
2. 생성된 코드의 라이브러리 버전과 문법을 검토하여 수정합니다.
3. 각 API 테스트를 위한 임의 데이터를 생성하고 테스트 코드를 작성합니다.
4. 빌드 및 테스트를 수행하고 결과를 확인합니다.