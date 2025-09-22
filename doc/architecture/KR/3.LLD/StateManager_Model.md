## 1. StateManager의 기능 
	- 상태 변경 요청 처리
		- NodeAgent, FilterGateway, PolicyManager, ActionController 등 각 컴포넌트는 자신이 다루는 리소스의 상태를 변화시키기 위해 StateManager에게 상태 변경을 요청합니다.
		- StateManager는 이 요청을 받아 해당 리소스의 상태를 ETCD에 저장합니다.

	- 상위 리소스 연쇄 상태 관리
		- StateManager는 하위 리소스의 상태가 변경될 때 연쇄적으로 상위 리소스의 상태도 변경되는지 확인합니다.
		- 변경이 필요한 경우 상위 리소스의 새로운 상태를 ETCD에 저장합니다.


## 2. StateManager의 구현 구조
main.rs: StateManager 실행의 진입점(메인 함수)입니다. 서비스 초기화, 설정 로딩, 서버 실행 등을 담당합니다.

manager.rs: StateManager의 핵심 로직(상태 변경 처리, 상위/하위 리소스 상태 연쇄 관리 등)을 구현합니다. 상태 변경이 필요하면 state_machine.rs에 구현된 함수를 호출합니다. 

state_machine.rs: 리소스(Scenario, Package, Model 등)의 상태 전이 규칙과 상태 관리 핵심 알고리즘을 담당하는 상태머신 구현 파일입니다. 

types.rs: StateManager에서 사용하는 데이터 구조체, enum, 타입 정의가 모여 있습니다.

mod.rs: src 디렉터리의 모듈 트리 구성을 위한 모듈 선언 파일입니다.

grpc/
	mod.rs: grpc 하위 모듈 트리 구성을 위한 모듈 선언 파일입니다.
	
    receiver.rs: gRPC를 통해 외부에서 들어오는 상태 변경 요청을 수신하고 처리하는 역할을 합니다.
	
    sender.rs: gRPC를 통해 외부 시스템에 상태 변경 결과나 알림을 전송하는 역할을 합니다.

## 3. model을 위해 StateManager에 구현되어야 하는 것 
```
+-------------------+         +---------------------+         +-------------------+
|   NodeAgent       |  gRPC   |   StateManager      |   put   |       ETCD        |
|-------------------| ------> |---------------------| ------> |-------------------|
```

- **인터페이스:** 외부 인터페이스(gRPC)로부터 수신, 외부 인터페이스(ETCD)로 발신
	- **수신:** NodeAgent 컴포넌트로부터 pod과 container들의 상태 정보를 전달받음
	- **조건:** `<container, state>` 리스트가 model의 특정 state 조건과 일치하면 model의 state를 변경
	- **발신:** ETCD에 `<model, state>` put 요청

## 4. model의 state 변경 조건
model의 state는 model에 포함된 container의 상태가 model의 의 특정 state 조건과 일치하면 model의 state를 변경해야 합니다.

### 4.1 model 상태 정의 및 상태 전이 조건 요약표
| 상태      | 설명 | 조건 |
|-----------|------|---------------------------------------------------|
| Created   | model의 최초 상태 | 생성 시 기본 상태 |
| Paused    | 모든 container가 paused 상태일 때 | 모든 container가 paused 상태 |
| Exited    | 모든 container가 exited 상태일 때 | 모든 container가 exited 상태 |
| Dead      | 하나 이상의 container가 dead 상태이거나, model 정보 조회 실패 | 하나 이상의 container가 dead 상태이거나, model 정보 조회 실패 |
| Running   | 위 조건을 모두 만족하지 않을 때(기본 상태) | 위 조건을 모두 만족하지 않을 때(기본 상태) |

### 4.2 container 상태 정의 및 상태 전이 조건 요약표
| 상태     | 설명                                                                 | 조건                                                         |
|----------|----------------------------------------------------------------------|--------------------------------------------------------------|
| Created  | 아직 실행된 컨테이너가 없는 상태. 컨테이너가 생성되지 않았거나 모두 삭제된 경우 | 컨테이너가 생성되지 않았거나 모두 삭제된 경우                |
| Running  | 하나 이상의 컨테이너가 실행 중인 상태                                 | 하나 이상의 컨테이너가 실행 중                                |
| Stopped  | 하나 이상의 컨테이너가 중지되었고, 실행 중인 컨테이너는 없음          | 하나 이상의 컨테이너가 중지, 실행 중인 컨테이너는 없음        |
| Exited   | Pod 내 모든 컨테이너가 종료된 상태                                    | 모든 컨테이너가 종료됨                                       |
| Dead     | Pod의 상태 정보를 가져오는 데 실패한 경우 (메타데이터 손상, 시스템 오류 등) | 상태 정보 조회 실패, 시스템 오류 등                           |

- **인터페이스:** 외부 인터페이스(gRPC)로부터 수신, 외부 인터페이스(ETCD)로 발신
 
## 5. etcd로 put, get 하는 방법 규칙 
etcd에 값을 저장(put)하거나 조회(get)할 때는 문서에 제시된 예시 코드의 지정된 key/value 포맷대로 작성해야 한다. 

예시1 : model의 state put 시 
```
let key = format!("/model/{}/state", model_name);
let value = model_state.as_str_name(); // 예: "Running"
if let Err(e) = common::etcd::put(&key, value).await {
    eprintln!("Failed to save model state: {:?}", e);
}
```
예시2 : get: etcd에서 값 조회

```
let key = "/model/my_model/state";
match common::etcd::get(key).await {
    Ok(value) => println!("Value: {}", value),
    Err(e) => eprintln!("Failed to get: {:?}", e),
}
```

예시3 : get_all_with_prefix: prefix로 여러 값 조회
```
let prefix = "/model/";
match common::etcd::get_all_with_prefix(prefix).await {
    Ok(kvs) => {
        for kv in kvs {
            println!("key: {}, value: {}", kv.key, kv.value);
        }
    }
    Err(e) => eprintln!("Failed to get: {:?}", e),
}
```

