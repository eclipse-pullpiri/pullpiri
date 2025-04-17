# FilterGateway

## 1. Introduction

### Major features

FilterGateway는  시나리오에서 사용하는 차량 DDS 토픽을 수신받고, 시나리오 조건을 판단하여 충족할 경우 이벤트 트리거를 호출 하는 모듈이다. FilterGateway는 API-Server로 부터 시나리오 데이터를 전달 받고, 이벤트트리거를 호출하여 ActionController로 scenario name을 전달한다. FilterGateway는 manager와 Filter, vehicle로 구성되어 있으며 manager는 차량 데이터를 등록/수신/해제 하고 Filter를 생성한다. API-Server로 부터 grpc로 전달받은 시나리오는 struct Scenario로 파싱한다.
vehicle은 manager로 부터 전달받은 DDS Topic들을 수신받도록 구독한다.
한 시나리오는 여러개의 Topic을 가질수 있고 하나의 시나리오당 하나의 Filter가 생성된다. 생성된 Filter는 해당 시나리오의 Topic을 vehicle로 부터 전달받고 조건 충족 여부를 판단한다. 조건이 충족하면 Action Controller로 시나리오 이름을 전달하기 위한 gRPC로 이벤트 트리거를 호출한다.

### Main dataflow

1. API-Server로 부터 grpc로  struct Scenario를 전달 받는다.
1. 전달받은 Scenario 정보를 파싱하고 manager로 전달한다.  
1. manager는 전달받은 내용을 Vehicle로 전달하여 DDS 토픽을 구독한다.  
1. 이후 수신받은 DDS 토픽 데이터는 Filter로 전달한다.  
1. Filter에서 데이터 토픽의 조건을 판단하고 조건이 충족되면 Action-Controller로 grpc 호출을 한다.  

## 2. File information

main 에서 grpc sender, receiver, manager, dds listener를 생성하고 manager에서 filter를 생성한다.

```text
src/
├── main.rs
├── manager.rs
├── grpc/
│   ├── mod.rs
│   ├── receiver.rs
│   └── sender.rs
├── filter/
│   └── mod.rs
└── vehicle/
    ├── mod.rs
    └── dds/
       ├── mod.rs
       └── listener.rs
```

- **main.rs** - initialize 만 수행
- **manager.rs** - 차량 신호 listener 등록, gRPC receiver 로부터 받은 condition 정보로 filter 생성
- **grpc/mod.rs**
- **grpc/receiver.rs** - apiserver 로부터 받은 gRPC 메시지를 manager 로 channel을 통해 전달,
- **grpc/sender.rs** - actioncontroller 로 gRPC 메시지 전달
- **filter/mod.rs** - filter 관련된 모든 함수 들어 있음. Filter에 대한 struct 생성
- **vehicle/mod.rs**
- **vehicle/dds/mod.rs**
- **vehicle/dds/listener.rs**  - DDS listener 를 topic 마다 thread 로 생성. 각 thread는 channel 을 통해 manager 로 전달

## 3. 주요 API 정보

### API : Initialize

- **API Name**: Initialize
- **File**: main.rs
- **Type**: function
- **Parameters**:
- **Returns**:
- **Description**: manager 스레드 초기화 및 listener 생성 작업 진행.

### API : HandleScenario

- **API Name**: handle_scenario
- **File**: grpc/receiver.rs
- **Type**: grpc
- **Parameters**: scenario_yaml_str: String, action: i32
- **Returns**: common::Result<()>
- **Description**:  API-Server로 부터 PICCOLO 시나리오 이름을 받아서 ETCD에 저장된 시나리오 정보를 추가하거나 삭제한다.
API-Server로 부터 시나리오 yaml string 을 받아서 Scenario struct 에 넣는다. 차량 데이터 토픽을 구독등록하고 Filter 생성 후 실행 한다.

### API : subscribe_vehicle_data

- **API Name**: subscribe_vehicle_data
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: String, vehicle_message: Struct (message type, topic 등을 포함)
- **Returns**: common::Result<()>
- **Description**: 차량 데이터 토픽을 수신 받도록 구독 신청을 한다.

### API : unsubscribe_vehicle_data

- **API Name**: unsubscribe_vehicle_data
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: String, vehicle_message Struct
- **Returns**: common::Result<()>
- **Description**: 차량 데이터 토픽을 수신받지 않도록 구독 해제 한다.(현재 버전에서 생략)

### API : launch_scenario_filter

- **API Name**: launch_scenario_filter
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario: struct Scenario
- **Returns**: common::Result<()>
- **Description**: 전달 받은 PICCOLO Scenario에 해당하는 Filter를 생성하고 실행한다. Filter는 Scenario 하나 당 별도 thread로 생성되고 생성된 후 Scenario에 선언된 차량데이터를 Listener부터 전달 받는다. 전달 받은 차량 데이터로 Scenario의 조건을 판단 한다.

### API : remove_scenario_filter

- **API Name**: remove_scenario_filter
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: String
- **Returns**: common::Result<()>
- **Description**: PICCOLO Scenario에 해당하는 Filter를 삭제한다.

### API : meet_scenario_condition

- **API Name**: meet_scenario_condition
- **File**: filter.rs
- **Type**: function
- **Parameters**: scenario_data: vehicle message struct
- **Returns**: common::Result<()>
- **Description**: 수신받고 있는 차량데이터들이 Scenario 조건에 부합 하면 Action Controller의 TriggerAction 함수를 실행한다.

### API : pause_scenario_filter

- **API Name**: pause_scenario_filter
- **File**: filter.rs
- **Type**: function
- **Parameters**: scenario_name: String
- **Returns**: common::Result<()>
- **Description**: 해당 시나리오 Filter에서 수신받고 있는 차량 데이터들의 Scenario 조건 판단을 하지 않는다.

### API : resume_scenario_filter

- **API Name**: resume_scenario_filter
- **File**: filter.rs
- **Type**: function
- **Parameters**: scenario_name: String
- **Returns**: common::Result<()>
- **Description**: 해당 Filter에서 수신받고 있는 차량 데이터들의 Scenario 조건 판단을 재개 한다.

## 4.참고(참조파일 먼저 입력필요)

1. Cargo.toml 파일을 생성한 후 [dependencies] 에 'common = {workspace = true}' 문구를 추가한다.
1. 'common/src/spec/artifact/scenario.rs' 참고한다.
1. 'common/proto/filtergateway.proto'의 빌드 결과물을 참고해서 grpc/receiver.rs의 import를 작성한다.
1. 'common/proto/actioncontroller.proto'의 빌드 결과물을 참고해서 grpc/sender.rs의 import를 작성한다.
1. 'example/resource/bms-performance.yaml' 은 예제 시나리오 파일이니 참고해서 DDS 토픽을 수신받도록 구독한다.
1. 'example/resource//bms-performance.yaml' 은 예제 시나리오 파일이니 참고해서 수신 받은 DDS 토픽으로 사용한다.
1. 모듈 내부에서 통신하는 채널을 사용할때는 std::sync::mpsc 대신 tokio::sync::mpsc를 사용한다.
1. DDS 코드 개발은 test-dds.rs  참고해서 작성해줘.

## 5.코드 생성 이후 추가 요구사항

주요 api가 제대로된 파일위치에 모두 확인하고 제대로 구현한다
생성된 코드의 라이브러리 버전이랑 문법을 다시 확인해서 수정한다  
각 API 테스트를 위한 임의의 데이터를 만들어서 테스트 수행하는 테스트코드 만든다
빌드랑 테스트 수행하고 결과 보여줘.
전체 파일 다 보여줘
