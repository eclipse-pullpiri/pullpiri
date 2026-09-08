# [2] [설계] 리소스 분류 + URI 체계 + 메서드

## 1. 리소스 분류

8종을 대칭적으로 CRUD하지 않는다. 코드 분석 결과 리소스는 **역할에 따라 3계층**으로 나뉜다. (근거: 모두 `Artifact` 트레잇 `get_name()`=metadata.name으로 식별. Node만 gRPC 등록 경로 별도 보유, Pod는 Model에서 파생.)

| 계층 | 리소스 | 성격 | 필요한 REST 동작 |
| --- | --- | --- | --- |
| A. 1급 배포 리소스 | Scenario, Package, Model, Pod | 사용자가 직접 배포/조회. Scenario는 배포 파이프라인 트리거 | 전체 CRUD + List |
| B. 보조 리소스 | Volume, Network, Policy, Schedule | Model/Package가 참조. 단독 배포보다 참조 대상 | CRUD + List (단, 참조 무결성 검증 필요) |
| C. 읽기전용/파생 | Node | Node=gRPC 등록·조회(`NodeManager`), Pod=`save_pod_yaml_from_package`로 자동 생성되는 파생물 | GET(조회)만. 생성/삭제는 내부 경로 유지 |


## 2. 리소스별 URI 체계

버전 접두사 `/api/v1/` 채택. 복수형 컬렉션 + 이름 식별자 규칙.

| 리소스 | 컬렉션 URI | 단건 URI | 계층 |
| --- | --- | --- | --- |
| Scenario | /api/v1/scenarios | /api/v1/scenarios/{name} | A |
| Package | /api/v1/packages | /api/v1/packages/{name} | A |
| Model | /api/v1/models | /api/v1/models/{name} | A |
| pod | /api/v1/pods | /api/v1/pods/{name} | A |
| Volume | /api/v1/volumes | /api/v1/volumes/{name} | B |
| Network | /api/v1/networks | /api/v1/networks/{name} | B |
| Policy | /api/v1/policies | /api/v1/policies/{name} | B |
| Schedule | /api/v1/schedules | /api/v1/schedules/{name} | B |
| Node | /api/v1/nodes | /api/v1/nodes/{name} | C (읽기전용) |

> **Note**
>
> URI `/api/v1/{resource}/{name}` → RocksDB 저장 키 `{Kind}/{name}` 매핑이 1:1 단순 변환 가능하다.
> (단, 저장 키의 kind는 단수 PascalCase(Scenario, Package..)이고, URI는 복수 소문자(scenario, package, ..)이므로 라우팅 레이어에서 매핑이 필요함)

### 이름 식별자 규칙

- name은 1~63자의 lowercase DNS label 형식을 사용한다.
- 허용 문자: `a-z`, `0-9`, `-`
- 시작과 끝은 영숫자여야 한다.
- 비교는 case-sensitive이며 서버는 대소문자를 자동 변환하지 않는다.

## 3. API 버전 관리 전략

- **결정:** URI 경로 버저닝 `/api/v1/` 채택. (목표에서 이미 방향 확정, settingsservice도 `/api/v1/` 사용 중이라 일관성 확보)
- 헤더/콘텐츠 협상 방식 대신 경로 방식 채택 — pirictl 등 클라이언트 구현이 단순.
- **v2 정책:** 하위 호환 깨지는 변경 시에만 `/api/v2/` 신설, v1은 유지(deprecation 후 제거). 무거운 별도 Task가 아니라 본 문서의 한 규칙으로 확정.

## 4. HTTP 메서드 매핑

| 동작 | 메서드 + URI | 설명 |
| --- | --- | --- |
| 목록 조회 | GET /api/v1/{resources} | 컬렉션 반환 |
| 단건 조회 | GET /api/v1/{resources}/{name} | 없으면 404 |
| 생성 | POST /api/v1/{resources} | body에 정의. 성공 201 + Location |
| 수정(전체 교체) | PUT /api/v1/{resources}/{name} | 멱등. 없으면 404(~~또는 201 upsert 정책 택1~~) |
| 삭제 | DELETE /api/v1/{resources}/{name} | 성공 200 ~~204~~ |

Node는 GET 2종만 제공하고 POST/PUT/DELETE는 405로 명시 거부.

## 5. HTTP 상태 코드 체계 (Task③)

현행 문제: `route/mod.rs::status()`가 모든 `Err`를 **405로 일괄 매핑**. 아래로 세분화한다.

| 코드 | 사용 상황 |
| --- | --- |
| 200 OK | 조회/수정/삭제(~~내용 반환~~) 성공 |
| 201 Created | 리소스 생성 성공 (POST). Location 헤더 포함 |
| ~~204 No Content~~ | ~~삭제 성공 등 본문 없는 성공~~ |
| 400 Bad Request | YAML/JSON 파싱 실패, 잘못된 형식 |
| 404 Not Found | 존재하지 않는 리소스 조회/수정/삭제 |
| 405 Method Not Allowed | 계층 C에 쓰기 시도 등 진짜 메서드 불허 (원래 의미로 복원) |
| 409 Conflict | 이미 존재하는 리소스 생성, 참조 무결성 위반(사용 중 리소스 삭제) |
| 422 Unprocessable Entity | 문법은 맞으나 의미 오류(필수 필드 누락, 참조 대상 없음 등 — 현행 apply()의 Scenario/Package 누락 검증이 여기 해당) |
| 500 Internal Server Error | RocksDB/gRPC 등 내부 오류 |

> **Note**
>
> 현행 `apply()`의 "There is not any scenario/package" 오류는 405가 아니라 **422**로 매핑되어야 의미론적으로 맞다.

```




