# FilterGateway

## 1. 목적 프롬프트
### 주요기능
ActionController는 settings.json파일을 읽어서 각 노드가 bluechi node인지 normal node 인지 판단한다. ActionController는 외부 모듈 FilterGateway에서 트리거이벤트를 받아 시나리오 이름을 전달받는다. 전달 받은 시나리오 이름으로 ETCD에서 시나리오의 전체 정보를 읽어온다. 해당 시나리오의 Action과 Target을 확인하여 Bluechi Controller API를 호출할지 아니면 NodeAgent API를 호출할지 판단하여 수행한다. 
모든 bluechi node에 대해서 bluechi API를 한번만 호출하면 되고 모든 normal node에 대해서는 각 nodeagent 마다 API를 호출 해야한다. 외부모듈 statemanager로 부터 시나리오의 상태를 reconcile 함수로 수신 받아서 시나리오의 current state를 desire state로 조정하는 작업을 한다. 그 조정 작업은 runtime API 호출하여 조정한다.



### 주요 데이터 흐름
1. settings.json 파일로 부터 node 정보를 읽어온다.  
2. 외부 모듈 filtergateway로 부터 전달받은 시나리오의 이름으로 Action과 Target 정보를 ETCD에서 불러온다.  
3. Action과 Target 정보를 가지고 각 노드의 모드에 따라 runtime API(bluech API 혹은 nodeagent API) 를 호출한다.  
4. 외부 모듈 statemanager로 부터 전달받은 desire state와 current state를 확인하고 desire state로 조정하기 위해 runtime API(bluech API 혹은 nodeagent API)를 호출한다.  

## 2. 파일 정보
main 에서 grpc sender, receiver, manager를 생성한다. manager에서 각 노드별로 bluechi와 nodeagent 를 생성한다. 

ActionController  
├── main.rs  
├── manager.rs  
└── grpc  
    ├── mod.rs  
    └── receiver.rs  
└── runtime  
    ├── mod.rs  
    ├── bluechi
        └── mod.rs  
    └── nodeagent
        └── mod.rs  

main.rs   - initialize 만 수행한다. 
manager.rs  
grpc/mod.rs  
grpc/receiver.rs  
runtime/mod.rs  
runtime/bluechi/mod.rs 
Runtime/nodeagent/mod.rs  

- **main.rs** - initialize 만 수행.
- **manager.rs** - settins.json 파일에서 노드들의 정보를 읽어옴. 시나리오 정보를 ETCD에서 읽어와서 Action과 Target 정보를 확인. 노드 별로 API 호출. 
- **grpc/mod.rs**
- **grpc/receiver.rs** - filtergateway 로부터 받은 gRPC 메시지를 manager 로 channel을 통해 전달, statemanger로부터 받은 gRPC 메세지를 manager를 통해 전달. 
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
- **Returns**: core::result::Result
- **Description**:  API-Server로 부터 PICCOLO 시나리오 이름을 받아서 ETCD에 저장된 시나리오 정보를 추가하거나 삭제한다. 
API-Server로 부터 시나리오 yaml string 을 받아서 Scenario struct 에 넣는다. 차량 데이터 토픽을 구독등록하고 Filter 생성 후 실행 한다.

### API : subscribe_vehicle_data
- **API Name**: subscribe_vehicle_data
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: String, vehicle_message: Struct (message type, topic 등을 포함)
- **Returns**: core::result::Result
- **Description**: 차량 데이터 토픽을 수신 받도록 구독 신청을 한다.

### API : unsubscribe_vehicle_data
- **API Name**: unsubscribe_vehicle_data
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: String, vehicle_message Struct
- **Returns**: core::result::Result
- **Description**: 차량 데이터 토픽을 수신받지 않도록 구독 해제 한다.(현재 버전에서 생략)

### API : launch_scenario_filter
- **API Name**: launch_scenario_filter
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario: struct Scenario
- **Returns**: core::result::Result
- **Description**: 전달 받은 PICCOLO Scenario에 해당하는 Filter를 생성하고 실행한다. Filter는 Scenario 하나 당 별도 thread로 생성되고 생성된 후 Scenario에 선언된 차량데이터를 Listener부터 전달 받는다. 전달 받은 차량 데이터로 Scenario의 조건을 판단 한다.

### API : remove_scenario_filter
- **API Name**: remove_scenario_filter
- **File**: manager.rs
- **Type**: function
- **Parameters**: scenario_name: String
- **Returns**: core::result::Result
- **Description**: PICCOLO Scenario에 해당하는 Filter를 삭제한다.

### API : meet_scenario_condition
- **API Name**: meet_scenario_condition
- **File**: filter.rs
- **Type**: function
- **Parameters**: scenario_data: vehicle message struct
- **Returns**: core::result::Result
- **Description**: 수신받고 있는 차량데이터들이 Scenario 조건에 부합 하면 Action Controller의 TriggerAction 함수를 실행한다.

### API : pause_scenario_filter
- **API Name**: pause_scenario_filter
- **File**: filter.rs
- **Type**: function
- **Parameters**: scenario_name: String
- **Returns**: core::result::Result
- **Description**: 해당 시나리오 Filter에서 수신받고 있는 차량 데이터들의 Scenario 조건 판단을 하지 않는다.

### API : resume_scenario_filter
- **API Name**: resume_scenario_filter
- **File**: filter.rs
- **Type**: function
- **Parameters**: scenario_name: String
- **Returns**: core::result::Result
- **Description**: 해당 Filter에서 수신받고 있는 차량 데이터들의 Scenario 조건 판단을 재개 한다.

## 4.참고(참조파일 먼저 입력필요)
Cargo.toml 파일을 생성한 후 [dependencies] 에 'common = {workspace = true}' 이 문구를 추가한다.
'common/src/spec/scenario/mod.rs' 참고해서 시나리오를 파싱한다.(수정불가)   
'common/proto/filtergateway.proto'를 이용해서 grpc/receiver.rs를 구현한다.  
'common/proto/actioncontroller.proto'를 이용해서 grpc/sender.rs를 구현한다.  
'example/resource/scenario/bms/high-performance.yaml' 은 예제 시나리오 파일이니 참고해서 DDS 토픽을 수신받도록 구독한다.
'example/resource/scenario/bms/high-performance.yaml' 은 예제 시나리오 파일이니 참고해서 수신 받은 DDS 토픽의 조건이 만족하는지 판단한다.
모듈 내부에서 통신하는 채널을 사용할때는 std::sync::mpsc 대신 tokio::sync::mpsc를 사용한다.   

## 5.코드 생성 이후 추가 요구사항
주요 api가 제대로된 파일위치에 모두 확인하고 제대로 구현해줘
생성된 코드의 라이브러리 버전이랑 문법을 다시 확인해서 수정해줘
각 API 테스트를 위한 임의의 데이터를 만들어서 테스트 수행하는 테스트코드 만들어줘
빌드랑 테스트 수행하고 결과 보여줘.
전체 파일 다 보여줘