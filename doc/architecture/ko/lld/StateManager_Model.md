<!--
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
-->
# StateManager Model LLD

## 1. 컴포넌트 구조 (Component Structure)

### 1.1 문서 목적
이 문서는 StateManager 컴포넌트에 Model 상태 전이 기능을 구현하기 위한 저수준 설계(LLD) 기준을 정의합니다.
StateManager는 NodeAgent로부터 Pod/Container 상태를 수신하고, `<container, state>` 목록이 Model 상태 조건을 만족하면 Model 상태를 변경한 뒤 Persistency(RocksDB)에 `<model, state>` 형태로 저장합니다.

### 1.2 내부 모듈 구성
- `main.rs`: 서비스 초기화, 설정 로딩, 서버 실행 진입점
- `manager.rs`: 상태 변경 요청 처리 및 상위/하위 리소스 연쇄 상태 관리
- `state_machine.rs`: Scenario/Package/Model 등 리소스별 상태 전이 규칙 함수
- `types.rs`: 상태 전이 처리에 필요한 구조체, enum, 타입 정의
- `mod.rs`: 모듈 선언 및 트리 구성
- `grpc/mod.rs`: gRPC 하위 모듈 선언
- `grpc/receiver.rs`: 외부 상태 변경 요청 수신/처리
- `grpc/sender.rs`: 외부 시스템으로 상태 변경 결과 송신

### 1.3 의존성 방향
- 입력: `NodeAgent -> gRPC receiver -> manager`
- 처리: `manager -> state_machine`
- 출력: `manager -> common::rocksdb(Persistency interface)`

```text
+-------------------+         +---------------------+         +------------------------+
|   NodeAgent       |  gRPC   |   StateManager      |   put   | Persistency (RocksDB) |
|-------------------| ------> |---------------------| ------> |------------------------|
```

## 2. 데이터 모델 (Data Model)

### 2.1 핵심 타입/스키마
- 입력 상태: `<container, state>` 목록
- 출력 상태: `<model, state>`
- 저장 키: `/model/{model_name}/state`
- 저장 값: `ModelState`의 문자열 표현(예: `Running`)

### 2.2 Model 상태 정의
| 상태 | 설명 | 전이 조건 |
|---|---|---|
| Created | Model의 초기 상태 | 생성 시 기본 상태 |
| Paused | 일시 정지 상태 | 모든 Container가 `Paused` |
| Exited | 종료 상태 | 모든 Container가 `Exited` |
| Dead | 비정상 상태 | 하나 이상 `Dead` 또는 Model 조회 실패 |
| Running | 실행 상태(기본) | 위 조건 외 모든 경우 |

### 2.3 Container 상태 정의
| 상태 | 설명 | 조건 |
|---|---|---|
| Created | 생성되었으나 미실행 | 설정 완료, 메인 프로세스 미실행 |
| Initialized | 초기화 완료 상태 | 런타임 환경 설정 완료, 메인 프로세스 미시작 |
| Running | 정상 실행 상태 | 메인 프로세스 활성화 |
| Paused | 일시 중지 상태 | 메모리 유지, CPU 실행 중단 |
| Exited | 종료 상태 | 정상 종료 또는 강제 중단 |
| Unknown | 상태 확인 불가 | 네트워크/시스템/런타임 이슈 |
| Dead | 비정상 종료 상태 | 종료 코드 0이 아닌 종료 |

### 2.4 저장/캐시 전략
- 단일 진실원(SoT)은 Persistency(RocksDB) 키-값 저장소로 간주합니다.
- StateManager는 입력 이벤트를 기준으로 상태를 계산하고, 최종 상태를 Persistency(RocksDB)에 반영합니다.
- 키 충돌 방지를 위해 Model 단위 key namespace(`/model/{name}/state`)를 유지합니다.

## 3. 시퀀스/상태 전이 (Sequence/State)

### 3.1 주요 처리 흐름
1. NodeAgent가 Pod/Container 상태 목록을 gRPC로 전송합니다.
2. StateManager receiver가 요청을 수신하고 manager로 전달합니다.
3. manager가 상태 목록을 집계하고 state_machine 규칙으로 Model 상태를 계산합니다.
4. 계산된 Model 상태를 Persistency(RocksDB)에 put 합니다.
5. 필요 시 상위 리소스 상태 연쇄 갱신을 수행합니다.

### 3.2 상태 전이 규칙
- `Dead` 조건이 최우선입니다.
- 그다음 `Paused`, `Exited`를 전체 Container 일치 조건으로 평가합니다.
- 어떤 조건도 만족하지 않으면 `Running`으로 전이합니다.
- 최초 생성 시점에는 `Created`를 기본 상태로 둡니다.

## 4. 에러 처리 정책 (Error Handling)

### 4.1 오류 분류
- 입력 오류: gRPC 요청 포맷 불일치, 필수 필드 누락
- 조회 오류: Model 메타데이터 조회 실패
- 저장 오류: Persistency(RocksDB) put/get 실패
- 상태 오류: 정의되지 않은 Container 상태 값

### 4.2 재시도/복구 전략
- 저장/조회 실패는 `Result`로 호출자에게 전파하고 상위 계층에서 재시도 정책을 적용합니다.
- Model 조회 실패 시 안전한 실패 상태로 `Dead`를 적용합니다.
- 상태 문자열 파싱 실패 시 `Unknown`으로 매핑하거나 요청 자체를 거부합니다(구현 정책 선택).

### 4.3 구현 예시(Result 전파)
```rust
async fn save_model_state(model_name: &str, model_state: ModelState) -> Result<(), String> {
    let key = format!("/model/{}/state", model_name);
    let value = model_state.as_str_name();
    common::rocksdb::put(&key, value)
        .await
        .map_err(|e| format!("failed to save model state: {e:?}"))
}
```

## 5. 인터페이스 계약 (Interface Contract)

### 5.1 API/이벤트/메시지 계약
- 수신 인터페이스(gRPC): NodeAgent로부터 Container 상태 목록 수신
- 발신 인터페이스(Persistency): Model 상태를 key-value로 저장
- key 포맷: `/model/{model_name}/state`
- value 포맷: `ModelState::as_str_name()` 결과 문자열

### 5.2 버전 호환성 정책
- 상태 문자열 enum은 기존 값과의 하위 호환을 유지합니다.
- 신규 상태 추가 시 수신 측은 미지원 상태를 `Unknown` 또는 오류로 처리하도록 명시합니다.
- key schema 변경이 필요한 경우 버전 prefix(`/v2/model/...`) 도입을 검토합니다.

## 6. 테스트 전략 (Test Strategy)

### 6.1 단위 테스트
- 상태 전이 함수 우선순위 검증(`Dead > Paused > Exited > Running`)
- 전부 `Paused`/전부 `Exited`/혼합 상태 케이스 검증
- 비정상 상태 입력(`Unknown`, 미정의 문자열) 처리 검증

### 6.2 통합 테스트
- gRPC 수신부터 Persistency(RocksDB) 저장까지 E2E 경로 검증
- Model 조회 실패 시 `Dead` 전이 및 저장 동작 검증
- 연쇄 상태 갱신(하위 -> 상위 리소스) 검증

### 6.3 회귀 테스트 및 검증 기준
- 상태 전이 규칙 변경 시 기존 상태 시나리오 회귀 테스트를 필수 수행합니다.
- 검증 기준:
  - 입력 대비 기대 상태가 100% 일치
  - Persistency(RocksDB) key/value 포맷 준수
  - 오류 발생 시 panic 없이 `Result` 전파
