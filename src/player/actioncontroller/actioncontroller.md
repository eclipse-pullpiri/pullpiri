# ActionController

## 1. Introduction

### Major features

ActionController는 FilterGateway로부터 특정 시나리오의 조건 충족 이벤트를 전달받아, 해당 시나리오의 Action과 Target 정보를 기반으로 Bluechi Controller API 또는 NodeAgent API를 호출하여 작업을 수행하는 모듈입니다.  
또한, ETCD에서 시나리오 정보를 읽어와 각 노드의 상태를 조정하며, Bluechi와 NodeAgent를 사용하는 노드를 구분하여 처리합니다. filtergateway와 statmaanger로부터 메세지를 전달받고 Bluechi와 nodeagent로 함수를 호출합니다. 노드 별로 알맞은 함수 호출을 담당합니다.

### Main Dataflow

1. **초기화**: `settings.json` 파일에서 노드 정보를 읽어와 Bluechi 노드와 NodeAgent 노드를 구분합니다.
1. **시나리오 처리**: FilterGateway로부터 전달받은 시나리오 이름으로 ETCD에서 Action과 Target 정보를 조회합니다.
1. **작업 수행**: Action과 Target 정보를 기반으로 Bluechi API 또는 NodeAgent API를 호출하여 작업을 수행합니다.
1. **상태 관리**: 외부 모듈 StateManager로부터 시나리오(scenario), 현재상태(current), 기존스펙(desired) 상태 정보를 전달받습니다.
1. **상태 조정**: current와 desired를 상태를 비교해서 desired 상태로 current를 변경합니다.

## 2. File information

ActionController는 다음과 같은 파일들로 구성됩니다:

```text
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

## 3. Function information

### API : Initialize

- **API Name**: Initialize
- **File**: main.rs
- **Type**: function
- **Parameters**: None
- **Returns**: None
- **Description**: `settings.json` 파일에서 노드 정보를 읽어와 Bluechi 노드와 NodeAgent 노드를 구분하여 초기화합니다.

### API : TriggerAction

- **API Name**: trigger_action
- **File**: grpc/receiver.rs
- **Type**: grpc
- **Parameters**: scenario_name: string
- **Returns**: common::Result<()>
- **Description**: FilterGateway로부터 전달받은 시나리오 데이터를 manager의 TriggerManagerAction으로 전달합니다.

### API : Reconcile

- **API Name**: reconcile
- **File**: grpc/receiver.rs
- **Type**: grpc
- **Parameters**: scenario_name: string, current: i32, desired: i32
- **Returns**: common::Result<()>
- **Description**: Statemanger로부터 전달받은 시나리오 데이터를 manager의 ReconcileDo로 전달합니다.

### API : CheckPolicy

- **API Name**: check_policy
- **File**: grpc/sender.rs
- **Type**: grpc
- **Parameters**: scenario_name: string
- **Returns**: common::Result<()>
- **Description**: 해당 시나리오가 수행 가능한지 policy를 확인합니다.

### API : HandleWorkload

- **API Name**: handle_workload
- **File**: grpc/sender.rs
- **Type**: grpc
- **Parameters**: workload_name: string, action: i32, description: string
- **Returns**: common::Result<()>
- **Description**: nodeagent에서 수행 할 workload action의 내용을 전달 합니다

### API : TriggerManagerAction

- **API Name**: trigger_manager_action
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: common::Result<()>
- **Description**: grpc receiver로부터 전달받은 시나리오 데이터를 기반으로 ETCD에서 Action과 Target 정보를 조회하고, 작업을 수행합니다.

### API : ReconcileDo

- **API Name**: reconcile_do
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string, current: i32, desired: i32
- **Returns**: common::Result<()>
- **Description**: grpc reconcile로부터 전달받은 시나리오 데이터를 기반으로 scenario, current, desired 정보를 확인하고, 보정 작업을 수행합니다.

### API : CreateWorkload

- **API Name**: create_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: common::Result<()>
- **Description**: ETCD에서 Systemd 파일 및 Pod YAML 파일을 읽어와 작업을 생성합니다. Bluechi, NodeAgent따라 적절한 API를 호출합니다.

### API : DeleteWorkload

- **API Name**: delete_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: common::Result<()>
- **Description**: 기존 작업 파일을 삭제하고, Bluechi 또는 NodeAgent API를 호출하여 작업을 제거합니다.

### API : RestartWorkload

- **API Name**: restart_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: common::Result<()>
- **Description**: Bluechi 또는 NodeAgent API를 호출하여 작업을 재실행합니다.

### API : PauseWorkload

- **API Name**: pause_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: common::Result<()>
- **Description**: Bluechi 또는 NodeAgent API를 호출하여 작업을 일시정지합니다.

### API : StartWorkload

- **API Name**: start_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: common::Result<()>
- **Description**: Bluechi 또는 NodeAgent API를 호출하여 작업을 시작합니다.

### API : StopWorkload

- **API Name**: stop_workload
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: string
- **Returns**: common::Result<()>
- **Description**: Bluechi 또는 NodeAgent API를 호출하여 작업을 중지합니다.

## 4. 참고

- `Cargo.toml` 파일에 아래 내용을 추가합니다.

```text
common = {workspace = true}
tokio = { version = "1.43.1", features = ["full"] }
tonic = "0.12.3"
prost = "0.13.3"
serde = { version = "1.0.214", features = ["derive"] }
serde_yaml = "0.9"
common = {workspace = true}
```

- `tokio::sync::mpsc`를 사용하여 모듈 간 통신 채널에 사용합니다

- 로직 코드는 만들지 말고 함수형태만 만들어주고 아래 링크 참고해서  함수마다 주석 생성합니다.
 [링크](https://doc.rust-lang.org/stable/rustdoc/index.html)
