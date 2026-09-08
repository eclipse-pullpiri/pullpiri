# [3] [설계] 일괄 Apply 처리 방안 및 Content-Type/스키마

## 1. 현재 구현 사실 (코드 근거)

### 1.1 일괄 Apply는 이미 "번들" 처리다

```rust
// src/server/apiserver/src/artifact/mod.rs — apply()
let docs: Vec<&str> = body.split(YAML_SEPARATOR).collect();   // "---" 로 분할
for doc in docs {
    if let Some((kind, artifact_str)) = process_artifact_document(doc).await? {
        match kind.as_str() {
            KIND_SCENARIO => scenario_str = artifact_str,
            KIND_PACKAGE  => package_str  = artifact_str,
            _ => continue,        // Model/Volume/Network/Policy/Schedule은 저장만
        }
    }
}
if scenario_str.is_empty() { Err("...no scenario...") }        // 번들 불완전 → 거부
else if package_str.is_empty() { Err("...no package...") }     // 번들 불완전 → 거부
else { save_pod_yaml_from_package(&package_str).await?; Ok(scenario_str) }
```

- **하나의 요청 = 여러 리소스**(Scenario+Package+Model+…)를 `---`로 이어 붙인 multi-doc YAML.
- 각 문서는 `process_artifact_document()`에서 **개별적으로 kvstore에 즉시 write**된다(`{Kind}/{name}` 키).
- 최종적으로 Scenario·Package **둘 다 있어야** 성공. 하나라도 없으면 에러.
- 예시(`examples/helloworld.sh`): `curl -X POST :47099/api/artifact --header 'Content-Type: text/plain' --data "$BODY"`.

### 1.2 현재 일괄 처리의 3가지 빈틈

| 빈틈 | 코드 근거 | 문제 |
|---|---|---|
| **부분 실패 비원자성** | `process_artifact_document`가 루프 안에서 문서별로 `write_to_kvstore` 즉시 실행 | 앞 문서 저장 후 뒤 문서에서 에러 나면 **kvstore에 일부만 남는다**(롤백 없음) |
| **Content-Type 무검증** | 핸들러가 `async fn apply_artifact(body: String)` — axum `String` 추출기는 타입 불문 수신 | 잘못된 형식도 그대로 파싱 시도 |
| **오류 종류 뭉개짐** | `status()`가 모든 Err를 `405`로 | 파싱 실패/검증 실패/부분 실패 구분 불가 |

### 1.3 Content-Type은 사실상 무시된다

```rust
// route/api.rs
async fn apply_artifact(body: String) -> Response {   // ← String 추출기: Content-Type 검증 없음
    let result = crate::manager::apply_artifact(&body).await;
    super::status(result)
}
```

`helloworld.sh`는 `text/plain`을 보내지만, 서버는 **헤더를 보지 않고** 본문을 YAML로 파싱한다.

---

## 2. 일괄 Apply(번들 배포) 처리 방안

### 2.1 엔드포인트 

```text
POST   /api/v1/artifacts            # multi-doc YAML 번들 일괄 배포 → 202 Accepted
DELETE /api/v1/artifacts/{scenario} # Scenario 키로 번들 철회      → 202 Accepted
```

- 리소스별 생성 URI(`POST /scenarios` 등)는 두지 않는다.
- 성공은 **202 Accepted**(apply 비동기).

### 2.2 번들 파싱·검증 파이프라인 (설계)

요청 1건을 다음 순서로 처리하도록 규정한다.

```text
① Content-Type 검사        → 불일치 시 415
② multi-doc 분리("---")    → 분리 실패/빈 본문 시 400
③ 각 문서 kind/name 파싱    → 알 수 없는 kind·필수필드 누락 시 400
④ 번들 완전성 검사          → Scenario·Package 필수. 누락 시 422
⑤ 참조 무결성 검사(권장)    → Package가 가리키는 Model/Volume/Network/Policy/Schedule 존재 확인 → 누락 시 422
⑥ 일괄 커밋              → ①~⑤ 통과분을 한 번에 kvstore write (부분 실패 방지)
⑦ filtergateway로 enqueue  → 202 반환
```

> 현재는 Content-Type 검사 안하고 있고, multi-doc 분리는 하지만 오류 코드 없이 실패 시 그냥 빈 결과로 진행, 각 문서 kind/name 파싱은 하고 있지만 알 수 없는 kind는 조용히 skip 하고 있고, 번들 완전성 검사는 하지만 오류 코드는 무조건 405로 보내고 있고, 참조 무결성은 Model/Volume/Network만 읽고 없으면 422가 아니라 kvstore 에러 발생시키고, 일괄커밋은 없고, filtergateway enqueue는 마지막 한 세트만.   
> **핵심 개선**: 현재는 ②·③·⑥이 뒤섞여(문서별 즉시 write) **부분 저장**이 발생한다.
> 재설계는 **"검증을 모두 통과한 뒤 커밋"**(staging→commit) 순서로 바꿔 **번들 무결성**을 보장한다.
> (구현은 Out of Scope. 설계 문서에는 "일괄 커밋 요구사항"으로 명시)

### 2.3 부분 실패 시맨틱

| 상황 | 응답 | 비고 |
|---|---|---|
| 번들 전체 검증 통과 + 커밋 성공 | **202 Accepted** | 배포 접수 |
| 일부 문서 파싱/검증 실패 | **422** (또는 400) + 실패 목록 | **전체 거부**(원자적). 부분 성공 없음 |
| Scenario/Package 누락 | **422** | 번들 불완전 |
| 커밋(kvstore) 중 오류 | **500** | 이미 쓴 분은 롤백(설계 요구사항) |

> 결정: pullpiri는 번들 트랜잭션 모델이므로 **207 Multi-Status(부분 성공)를 쓰지 않는다.** all-or-nothing.

### 2.4 다중 시나리오 세트 처리 (현행 버그 + 재설계 요구)

**"일괄 Apply"의 의미를 재정의해야 한다.** 현재 "번들"은 사실상 *시나리오 세트 1개*(Scenario+Package+Model…)만 정상 배포한다.

#### 현행 동작 (코드 검증됨)

`apply()`는 문서를 순회하며 `scenario_str`/`package_str`을 **단일 변수에 덮어쓴다.**

```rust
// artifact/mod.rs apply()
KIND_SCENARIO => scenario_str = artifact_str,   // 매번 덮어씀 → 마지막 것만 생존
KIND_PACKAGE  => package_str  = artifact_str,   // 매번 덮어씀 → 마지막 것만 생존
...
Ok(scenario_str)                                 // 마지막 scenario 1개만 반환
// manager.rs apply_artifact(): 이 1개만 filtergateway로 전송
```

→ 하나의 YAML에 시나리오 세트가 A/B/C 여러 개 있으면:

| 항목 | 결과 |
|---|---|
| kvstore 저장 | A·B·C **모두 저장**(`process_artifact_document` 문서별 즉시 write) |
| Pod YAML 생성 | **마지막 세트(C)만** |
| filtergateway 배포 트리거 | **마지막 세트(C)만** |
| A·B | 저장은 됐으나 **배포되지 않는 유령 상태** (silent failure) |

#### 두 가지 "일괄 배포" 시나리오

| # | 사용자 기대 | 현행 | 재설계 방향 |
|---|---|---|---|
| **(가)** | 1개 YAML 안에 **여러 시나리오 세트** | 마지막 1개만 배포(버그) | `apply()`가 세트를 **그룹핑→리스트**로 수집, 각 세트를 개별 `HandleScenarioRequest`로 **모두 전송** |
| **(나)** | **세트 1개짜리 YAML 파일 여러 개**를 일괄 배포 | REST 1회 호출 불가. cat 병합 시 (가)의 버그에 걸림 | 서버가 (가)를 지원하면 **cat 병합 → 1회 POST**로 자연 해결. 또는 클라이언트가 파일별 순차 POST |

#### 설계 요구사항

1. **세트 그룹핑**: 번들을 `Scenario`를 경계로 세트 단위로 묶는다(Scenario N개 → 세트 N개). Package/Model 등은 각 세트에 귀속.
2. **다중 전송**: 각 세트마다 `HandleScenarioRequest`를 생성해 **모두** filtergateway로 enqueue한다(현행처럼 마지막 1개가 아님).
3. **무결성 결합**: §2.2의 일괄 커밋과 결합 — 모든 세트가 검증 통과해야 **전체 커밋 + 전체 전송**, 하나라도 실패면 전체 거부(all-or-nothing).
4. **네이밍 충돌 처리**: 같은 번들에 동일 `Scenario/{name}`이 중복되면 400(중복 선언)으로 거부.
5. **응답**: 다중 세트라도 접수는 **202 Accepted** 하나로 응답하고, 세트별 실제 기동 결과는 GET 상태 조회로 확인(v2 Task 3.2 비동기 근거).

> **결론**: "일괄 Apply"는 **① 다중 리소스(문서) 배포**뿐 아니라 **② 다중 시나리오 세트 배포**까지 포함해야 한다.
> 현행의 한계는 다음과 같다.
> - **① 다중 문서 배포 — 부분 지원**: 문서 8종은 모두 kvstore에 *저장*되나, ⓐ 중간 실패 시 앞 문서 롤백이 없어 **부분 저장**이 남고(일괄 커밋 아님), ⓑ Network는 읽기만 하고 **미반영(`TODO`)**이다. 즉 "저장"은 전부지만 "배포 반영"은 일부다.
> - **② 다중 시나리오 세트 — 미지원(버그)**: `scenario_str`/`package_str` 단일 변수 덮어쓰기로 **마지막 세트 1개만 배포**된다.
>
> 따라서 `apply()`의 단일 변수 구조를 **세트 리스트 구조로 재설계**하고, 일괄 커밋(all-or-nothing)과 결합하는 것이 핵심 요구사항이다. (구현은 Out of Scope, 설계 요구사항으로 명시)

### 2.4 철회(withdraw)의 멱등성

```rust
// 현재 withdraw(): Scenario 문서를 찾아 "Scenario/{name}"만 삭제
```

| 상황 | 응답 |
|---|---|
| 존재하는 scenario 철회 | 202 Accepted |
| **이미 없는** scenario 철회 | **204 No Content** 권장(멱등) 또는 404(엄격) 중 택1 |

> 권장: DELETE는 멱등이 자연스러우므로 "없어도 성공(204)". 단 감사 목적이면 404도 허용 — 문서에 택1 명시.

---

## 3. Content-Type 및 요청/응답 스키마

### 3.1 요청 Content-Type 규정

| Content-Type | 용도 | 처리 |
|---|---|---|
| `application/yaml` (권장 정식) | multi-doc 번들 배포 | 파싱 수행 |
| `application/x-yaml`, `text/yaml` | 관용 별칭 | 허용(정규화) |
| `text/plain` | **현행 호환**(helloworld.sh) | 한시 허용(deprecated 경고) |
| 지원하지 않는 타입(예: JSON 미지원 시점) | — | **415 Unsupported Media Type** |
| 헤더 누락/빈 값 | — | **400 Bad Request** (또는 text/plain로 관용 처리 — 택1) |

> **415 vs 400 구분 원칙** (v2 Task 3 매핑과 대칭):
> - **400**: `Content-Type` 헤더가 **아예 없거나 비어 있음** → 요청이 자신의 형식을 선언하지 않은 것.
> - **415**: 헤더는 있으나 **서버가 지원하지 않는 타입**(예: 아직 미지원인 `application/json`).

#### 별칭(alias) 허용 정책

`application/yaml`은 RFC 9512(2024)에서야 공식 등록된 미디어 타입이라, 구형 클라이언트·프록시·SDK는 아래 관용 표기를 보낼 수 있다. 이를 **거부하지 않고 정규 타입으로 정규화(normalize)**하여 수용한다.

| 수신 표기 | 정규화 결과 | 비고 |
|---|---|---|
| `application/yaml` | `application/yaml` | 정식(권장) |
| `application/x-yaml` | `application/yaml` | 표준화 이전 관용 표기 |
| `text/yaml` | `application/yaml` | 표준화 이전 관용 표기 |
| `text/plain` | `application/yaml`로 **한시 처리** | 현행 호환, deprecated 경고 로그 |

- **정규화 지점**: Content-Type 검증 미들웨어/추출기에서 별칭을 정식 타입으로 매핑한 뒤 파서로 라우팅한다.
- **파라미터 무시**: `application/yaml; charset=utf-8`처럼 `;` 뒤 파라미터가 붙어도 **주 타입만 비교**한다.
- **대소문자 무시**: 미디어 타입 비교는 대소문자를 구분하지 않는다(RFC 규정).
- **sunset 대상**: `text/plain`만 향후 제거 예정. `application/x-yaml`·`text/yaml`은 영구 별칭으로 유지.

- **검증 추가**: axum `String` 추출기 대신 **Content-Type을 먼저 확인**하는 미들웨어/추출기 도입(설계 요구사항).
- **JSON 요청**은 초기 범위에서 제외(향후 구조화 페이로드 옵션). 현행 워크플로는 YAML 유지.

### 3.2 요청 본문 스키마 (번들)

기존 형식을 유지한다(하위호환).

> **용어 — 문서(document)**: YAML 표준에서 하나의 스트림(여기서는 HTTP 본문 하나)은 `---`(YAML_SEPARATOR) 구분자로 **여러 문서(multi-document)**를 담을 수 있다. `---`로 나뉜 **각 YAML 블록 하나**가 "문서"이며, 본 설계에서는 문서 하나 = **리소스 선언 하나**(Scenario/Package/Model/…)에 대응한다. 아래 예시는 **1개의 번들 본문 안에 여러 개의 문서**가 담긴 형태다.

```yaml
# Content-Type: application/yaml
apiVersion: v1
kind: Scenario
metadata: { name: helloworld }
spec: { ... }
---
apiVersion: v1
kind: Package
metadata: { name: helloworld }
spec: { models: [ ... ], policy: ..., schedule: ... }
---
apiVersion: v1
kind: Model
metadata: { name: helloworld-core }
spec: { ... }
# (+ 필요 시 Volume/Network/Policy/Schedule 문서 추가)
```

문서별 필수 필드: `apiVersion`, `kind`, `metadata.name`, `spec`.
`kind`는 8종(Scenario/Package/Model/Volume/Network/Node/Policy/Schedule) 중 하나. (Node는 REST 배포 비대상 — v2 참조)

### 3.3 응답 스키마

현재는 응답 본문 규격이 없다(상태코드만). 재설계는 **일관된 JSON envelope**로 표준화한다.

```jsonc
// 성공 (202)
{
  "data": {
    "scenario": "helloworld",
    "accepted": ["Scenario/helloworld", "Package/helloworld", "Model/helloworld-core"],
    "status": "accepted"          // 비동기: 접수만 의미
  },
  "error": null
}
```

```jsonc
// 실패 (4xx/5xx)
{
  "data": null,
  "error": {
    "code": "BUNDLE_INCOMPLETE",  // 기계 판독용 코드
    "message": "There is no Package in the bundle",
    "details": [                  // 다중 검증 실패 시
      { "doc": 2, "kind": "Model", "reason": "referenced model 'x' not found" }
    ]
  }
}
```

- 조회(GET) 응답도 같은 envelope를 사용: `{ "data": {...}, "error": null }`.
- `error.code`는 Task 3의 오류 enum(선행조건)과 1:1로 매핑한다.

#### 조회(GET) 응답 예시

GET은 apply 때 kvstore(`{Kind}/{name}`)에 저장된 **선언(desired) 그대로**를 반환한다.
저장값은 정규화 YAML이며, 각 리소스 struct가 이미 `serde::Serialize`를 구현하므로
**YAML → JSON 직렬화는 두 줄(`serde_yaml::from_str` → `serde_json::to_string`)로 충분**하다(별도 매핑 불필요).

**단건 조회** — 저장된 리소스 선언 전체를 JSON으로:

```jsonc
// GET /api/v1/scenarios/helloworld → 200 OK
{
  "data": {
    "apiVersion": "v1",
    "kind": "Scenario",
    "metadata": { "name": "helloworld" },
    "spec": {
      "condition": {
        "express": "eq",
        "value": "true",
        "operands": { "type": "DDS", "name": "value", "value": "ADASObstacleDetectionIsWarning" }
      },
      "action": "update",
      "target": "helloworld"
    }
  },
  "error": null
}
```

**목록 조회** — 가벼운 요약 배열(전체 spec은 단건으로 재조회):

```jsonc
// GET /api/v1/scenarios → 200 OK
{
  "data": {
    "items": [
      { "name": "helloworld", "kind": "Scenario" },
      { "name": "antipinch",  "kind": "Scenario" }
    ],
    "count": 2
  },
  "error": null
}
```

**없을 때**:

```jsonc
// GET /api/v1/scenarios/nope → 404 Not Found
{ "data": null, "error": { "code": "NOT_FOUND", "message": "scenario 'nope' not found" } }
```

##### 리소스별 저장(=반환) 값 (코드 기준)

키는 `{Kind}/{name}`, 값은 아래 spec을 담은 정규화 YAML이다.

| 리소스 | 저장되는 주요 필드(spec) | 스키마 정의 파일 |
|---|---|---|
| **Scenario** | `condition{express,value,operands{type,name,value}}`, `action`, `target` | `common/src/spec/artifact/scenario.rs` (`ScenarioSpec`) |
| **Package** | `pattern[]`, `models[]{name,node,resources{volume,network}}`, `policy?`, `schedule?` | `common/src/spec/artifact/package.rs` (`PackageSpec`) |
| **Model** | `spec` = **PodSpec**(containers/이미지/자원 등, k8s Pod 스펙 타입) | `common/src/spec/artifact/model.rs` (`ModelSpec=PodSpec`) |
| **Volume** | `spec.volumes[]`(k8s Pod Volume 목록) | `common/src/spec/artifact/volume.rs` (`VolumeSpec`) |
| **Network** | `spec.dummy`(현재 **미구현 placeholder**) | `common/src/spec/artifact/network.rs` (`NetworkSpec`) |
| **Policy** | `spec.placement.availableNodes[]`, `spec.procedure{...}` | `common/src/spec/artifact/policy.rs` (`PolicySpec`) |
| **Schedule** | `spec[]{name,priority,policy,cpu_affinity,period,release_time,runtime,deadline,node_id,max_dmiss}`, `temporal_class` | `common/src/spec/artifact/schedule.rs` (`ScheduleSpec`) |

> 참고: Scenario에는 `status`(state) 구조가 별도로 있으나 apply 시점엔 **선언(spec)만** 저장된다. 실제 기동 상태(actual)는 settingsservice(런타임/모니터링) 소관이다.

##### 저장(kvstore/RocksDB) 경로 — 값이 실제로 기록되는 지점

RocksDB는 apiserver에 내장되지 않고 **별도 gRPC 서비스(`rocksdbservice`)** 로 분리돼 있다. apply한 값은 아래 4단계를 거쳐 RocksDB에 기록된다.

| 단계 | 파일:라인 | 역할 |
|---|---|---|
| ① | `apiserver/src/artifact/data.rs:41` `write_to_kvstore()` | apiserver 측 래퍼 |
| ② | `common/src/kvstore.rs:22` `put()` | RocksDB **gRPC 클라이언트**(`ROCKSDB_SERVICE_URL`) |
| ③ | (gRPC 전송) | `RocksDbServiceClient::connect(...).put(PutRequest{key,value})` |
| ④ | `server/rocksdbservice/src/main.rs:111` `put()` → `db_lock.put(key,value)` | **실제 RocksDB write** (DB 경로 `/tmp/pullpiri_shared_rocksdb`) |

> **설계 함의**: RocksDB가 gRPC로 분리돼 있으므로, **일괄 커밋(all-or-nothing)** 은 `BatchPutRequest`(kvstore.rs가 이미 import) 활용 또는 rocksdbservice의 트랜잭션 엔드포인트가 필요하다.

### 3.4 응답 Content-Type

- 응답 본문은 **항상 `application/json`**.
- 요청이 YAML이어도 응답은 JSON으로 통일(파싱·툴링 용이).

---

## 4. 이슈 Task 대응 정리

| 이슈 항목 | 결정 |
|---|---|
| ④ 일괄 Apply | `POST /api/v1/artifacts`로 번들 수신(현행 `---` 계승). **검증 후 일괄 커밋(all-or-nothing)**으로 부분 저장 제거. 202 반환 |
| ④ 철회 | `DELETE /api/v1/artifacts/{scenario}` — Scenario 키 단위, 멱등(204) 또는 404 택1 |
| ⑤ Content-Type | `application/yaml` 정식화 + **415/400 검증 추가**. `text/plain`은 한시 호환(deprecated) |
| ⑤ 요청 스키마 | 기존 multi-doc YAML 유지(하위호환), 문서별 필수필드 규정 |
| ⑤ 응답 스키마 | **통일 JSON 봉투**(`data`/`error`), `error.code`는 오류 enum과 매핑, 응답은 항상 JSON |

---