## 1. 문서의 목적
이 문서는 StateManager컴포넌트에 package의 state 변경하는 기능을 추가하기 위해 작성되었습니다.
StateManager의 manager.rs는 model의 state가 변경되면 연쇄적으로 state_machine.rs의 'package의 상태가 변경되는지 확인하는 함수'를 호출하고 state_machine.rs에 구현된 해당 함수는 `<model, state>` 리스트를 전달받아 package의 특정 state 조건과 일치하면 package의 state를 변경하여 return 합니다. 
변경된 결과는 ETCD에 `<package, state>`형태로 put 요청됩니다.

이 기능은 이 문서에 포함된 조건 및 규칙들을 따라야 합니다. 


## 2. package을 위해 StateManager에 구현되어야 하는 것 

```
+---------------------+         +-------------------+
|   StateManager      |   put   |       ETCD        |
|---------------------| ------> |-------------------|
							|
							--> +-------------------+
						gRPC    |  ActionController |
								|-------------------|
```
- **인터페이스:** 내부 함수로부터 수신, 외부 인터페이스(ETCD)로 발신
	- **수신:** manager.rs의 함수로부터 state_machine.rs의 함수는 model의 상태 변경 시 연쇄적으로 package의 상태가 변경되는지 확인 요청을 전달받음
	- **조건:** state_machine.rs의 함수는 `<model, state>` 리스트가 package의 특정 state 조건과 일치하면 package의 state를 변경하여 return 
	- **발신:** manager.rs의 함수는 ETCD에 `<package, state>` put 요청 그리고 만약 package dead 상태 시 ActionController에 reconcile 요청

## 4. pacakge의 state 변경 조건
package의 state는 package에 포함된 model들의 상태가 package의 특정 state 조건과 일치하면 package의 state를 변경해야 합니다.

### 4.1 package 상태 정의 및 상태 전이 조건 요약표
| 상태      | 설명 | 조건 |
|-----------|------|---------------------------------------------------|
| idle      | 맨 처음 package의 상태 | 생성 시 기본 상태 |
| paused    | 모든 model이 paused 상태일 때 | 모든 model이 paused 상태 |
| exited    | 모든 model이 exited 상태일 때 | 모든 model이 exited 상태 |
| degraded  | 일부 model이 dead 상태일 때 | 일부(1개 이상) model이 dead 상태, 단 모든 model이 dead가 아닐 때 |
| error     | 모든 model이 dead 상태일 때 | 모든 model이 dead 상태 |
| running   | 위 조건을 모두 만족하지 않을 때(기본 상태) | 위 조건을 모두 만족하지 않을 때(기본 상태) |

- **인터페이스:** 외부 인터페이스(gRPC)로부터 수신, 외부 인터페이스(ETCD)로 발신

### 5. etcd로 put, get 하는 방법 규칙 
etcd에 값을 저장(put)하거나 조회(get)할 때는 문서에 제시된 예시 코드의 지정된 key/value 포맷대로 작성해야 한다. 

예시1 : package의 state put 시 
```
let key = format!("/package/{}/state", package_name);
let value = package_state.as_str_name(); // 예: "Running"
if let Err(e) = common::etcd::put(&key, value).await {
	eprintln!("Failed to save package state: {:?}", e);
}
```
예시2 : get: etcd에서 값 조회

```
let key = "/package/my_package/state";
match common::etcd::get(key).await {
	Ok(value) => println!("Value: {}", value),
	Err(e) => eprintln!("Failed to get: {:?}", e),
}
```

예시3 : get_all_with_prefix: prefix로 여러 값 조회
```
let prefix = "/package/";
match common::etcd::get_all_with_prefix(prefix).await {
	Ok(kvs) => {
		for kv in kvs {
			println!("key: {}, value: {}", kv.key, kv.value);
		}
	}
	Err(e) => eprintln!("Failed to get: {:?}", e),
}
```
