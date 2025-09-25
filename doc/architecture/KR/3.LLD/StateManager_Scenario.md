## 0. 문서의 목적
이 문서는 StateManager컴포넌트에 scenario의 state 변경하는 기능을 추가하기 위해 작성되었습니다.
StateManager는 FilterGateway, PolicyManager, ActionController 컴포넌트로부터 scenario의 상태 정보를 전달받아 ETCD에 `<scenario_name, state>`형태로 put을 요청합니다. 

이 기능은 이 문서에 포함된 조건 및 규칙들을 따라야 합니다. 

## 1. StateManager의 기능 
	- 상태 변경 요청 처리
		- NodeAgent, FilterGateway, PolicyManager, ActionController 등 각 컴포넌트는 자신이 다루는 리소스의 상태를 변화시키기 위해 StateManager에게 상태 변경을 요청합니다.
		- StateManager는 이 요청을 받아 해당 리소스의 상태를 ETCD에 저장합니다.

## 2. StateManager의 구현 구조
main.rs: StateManager 실행의 진입점(메인 함수)입니다. 서비스 초기화, 설정 로딩, 서버 실행 등을 담당합니다.

manager.rs: StateManager의 핵심 로직(ETCD를 통한 상태 변경 처리, 상위/하위 리소스 상태 연쇄 관리 등)을 구현합니다.

state_machine.rs: 리소스(Package, Model 등)의 상태 전이 판단 기능은 이 파일에 함수로 구현되어야 합니다.

types.rs: StateManager에서 사용하는 데이터 구조체, enum, 타입 정의가 모여 있습니다.

mod.rs: src 디렉터리의 모듈 트리 구성을 위한 모듈 선언 파일입니다.

grpc/
	mod.rs: grpc 하위 모듈 트리 구성을 위한 모듈 선언 파일입니다.
	
    receiver.rs: gRPC를 통해 외부에서 들어오는 상태 변경 요청을 수신하고 처리하는 역할을 합니다.
	
    sender.rs: gRPC를 통해 외부 시스템에 상태 변경 결과나 알림을 전송하는 역할을 합니다.

## 3. scenario를 위해 StateManager에 구현되어야 하는 것 

- **인터페이스:** 외부 인터페이스(gRPC)로부터 수신, 외부 인터페이스(ETCD)로 발신
	- **수신:** FilterGateway, PolicyManager, ActionController 컴포넌트로부터 scenario의 상태 변경 정보를 전달받음
	- **조건:** 없음 
	- **발신:** ETCD에 `<scenario_name, state>` put 요청

## 4. Scenario의 state machine과 조건
Scenario의 state 변경 조건
Scenario의 state는 각 컴포넌트에서 전이 조건을 만족하면 변경됩니다.

### 4.1 시나리오 상태 머신 통합 정의
| 상태 | 설명 | 전이 조건 | 담당 컴포넌트 | 다음 상태 |
|------|------|-----------|---------------|-----------|
| idle | 시나리오가 초기화된 상태 (아직 활성화되지 않음) | 생성 시 초기 상태 | - | waiting |
| waiting | 조건이 등록된 상태 | 조건 등록 | FilterGateway | satisfied |
| satisfied | 조건이 만족된 상태 | 조건 만족 | ActionController | allowed 또는 denied |
| allowed | 정책에 의해 실행이 허용된 상태 | 정책 검증 성공 | PolicyManager | completed |
| denied | 정책에 의해 실행이 거부된 상태 | 정책 검증 실패 | PolicyManager | - |
| completed | 시나리오 실행이 완료된 상태 | 시나리오 완료 시 | ActionController| - | 
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

