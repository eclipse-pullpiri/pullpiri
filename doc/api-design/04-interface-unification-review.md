# [4] [검토] pirictl <-> APIServer 인터페이스 일원화 방향

## 1. 현행 인터페이스 구조 (코드 근거)

### 1.1 pirictl은 클라이언트를 **2개** 만든다

```rust
// src/tools/pirictl/src/main.rs — main()
let settings_client = SettingsClient::new(&settings_url, ...);  // :8080  (settingsservice)
let api_client      = SettingsClient::new(&api_url, ...);       // :47099 (apiserver)
// 주석: "YAML commands go directly to API Server; others go to SettingsService"
```

| pirictl 서브커맨드 | 대상 서비스 | 포트 |
|---|---|---|
| `apply -f <file>` / `delete -f <file>` | **apiserver** | `:47099` |
| `get {boards\|nodes\|socs\|containers}` | settingsservice | `:8080` |
| `describe` / `raw` / `top` / `health` | settingsservice | `:8080` |

> 기본 포트는 CLI 인자로 노출된다: `--api-port 47099`, `--settings-port 8080`(`main.rs`).

### 1.2 settingsservice는 배포 요청을 apiserver로 **프록시**한다

```rust
// src/server/settingsservice/src/settings_api/mod.rs — send_artifact_to_api_server()
let api_server_url = format!("http://{}/api/artifact", common::apiserver::open_rest_server());
let request = match method { "POST" => client.post(url), "DELETE" => client.delete(url), ... };
request.header("Content-Type", "text/plain").body(yaml_content).send().await
```

즉 settingsservice의 `/api/v1/yaml`(apply/withdraw) 엔드포인트는 자체 처리를 하지 않고 **apiserver `/api/artifact`로 재전송**한다.

### 1.3 결과: ~~3중 경로~~

```text
[배포·직접]   pirictl ──(:47099)─▶ apiserver /api/artifact          ← pirictl apply/delete
[배포·프록시]  (다른 클라이언트) ─(:8080)─▶ settingsservice /api/v1/yaml
                                             └─(:47099)─▶ apiserver /api/artifact
[조회]        pirictl ──(:8080)──▶ settingsservice /api/v1/...       ← get/describe/raw/top
```

- 같은 "배포"가 **두 개의 진입점**(apiserver 직접 / settingsservice 프록시)을 가진다.
- pirictl 사용자는 조회와 배포에서 **서로 다른 포트**를 의식해야 한다.

---

## 2. 문제의 본질 — "공백"과 "중복"을 분리해서 봐야 한다

이원화가 전부 나쁜 게 아니다. **의도된 분리**와 **진짜 공백**을 구분해야 올바른 방향이 나온다.

| 조회 종류 | 데이터 출처 | 현재 제공자 | 성격 | 조치 |
|---|---|---|---|---|
| **① Artifact 조회** (Scenario/Package/Model/…) | apiserver가 apply 시 kvstore에 쓴 **선언(desired)** | **없음 → 진짜 공백** | apiserver 소관 | **apiserver에 GET 신설** |
| **② 런타임 조회** (Node 상태·metrics·logs·container) | nodeagent/monitoring이 채우는 **텔레메트리(actual)** | settingsservice `monitoring_kvstore` | monitoring 소관 | **settingsservice 존치** |

- **①은 진짜 공백**: apiserver가 데이터를 갖고도(kvstore) 읽는 문을 안 열어놨다. → pirictl이 "배포한 걸 확인"하려면 엉뚱하게 settingsservice로 가야 한다.
- **②는 의도된 분리**: 런타임 상태는 애초에 apiserver가 만든 데이터가 아니다. 옮기려면 monitoring 데이터 접근을 통째로 이관해야 하므로 **단순 이동이 아니다.**

> **핵심 통찰**: pirictl의 조회가 settingsservice로 가는 것은 "apiserver에 GET이 없어서"만이 아니라, **원래 다른 종류의 데이터(런타임 상태)를 보기 때문**이다. 따라서 "모두 apiserver로 몰기"는 틀린 목표다.

---

## 3. 통합 방향 비교

| 방향 | 내용 | 장점 | 단점 |
|---|---|---|---|
| **A (권장)** | apiserver에 **artifact 조회 GET(①)만 신설**. artifact 조회+배포는 `:47099`로 일원화, **런타임 조회(②)는 settingsservice 유지** | 진짜 공백(artifact 조회) 해소, 데이터 플레인 경계 유지, 배포와 조회가 같은 서비스 | apiserver에 조회 책임 일부 추가 |
| B | settingsservice를 **조회 게이트웨이**로 유지(모든 조회 통일), apiserver는 배포 전용 | 조회 진입점 단일 | artifact 조회 공백 지속(settingsservice가 apiserver kvstore를 다시 읽어야 함 → 새 결합 발생) |
| C | **settingsservice 프록시 제거**, pirictl이 모든 배포를 apiserver 직접 호출 | 3중 경로의 프록시 변 제거 | settingsservice 배포 API 소비자(대시보드 등) 영향 → Out of Scope 위반 소지 |

> **결정: 방향 A.**
> - artifact **조회+배포**를 apiserver(`:47099`)로 모으면 pirictl의 artifact 워크플로가 **단일 포트**로 정리된다.
> - **런타임 조회는 settingsservice에 존치** — apiserver 소관이 아니고, `GET /api/v1/nodes`는 settingsservice가 **이미 점유** 중이라 apiserver가 같은 경로를 신설하면 충돌한다.
> - 방향 C(프록시 제거)는 settingsservice 기능 변경이라 **Out of Scope**. 다만 "향후 정리 후보"로만 기록한다(§5).

---

## 4. 방향 A 적용 시 인터페이스 재배치 (설계 관점)

### 4.1 서비스별 책임 경계

| 서비스 | 포트 | 책임 (방향 A) |
|---|---|---|
| **apiserver** | `:47099` | 번들 **배포/철회** + **artifact 7종 조회(GET 신설)** |
| **settingsservice** | `:8080` | **런타임/모니터링 조회**(Node·Pod·metrics·container·top) — 기존 유지 |

### 4.2 pirictl 라우팅 재정리(방향 A 반영)

| pirictl 서브커맨드 | 대상(현행) | 대상(방향 A) | 변화 |
|---|---|---|---|
| `apply` / `delete` | apiserver `:47099` | apiserver `:47099` | 유지 |
| **`get scenarios/packages/models/…`** (신규) | (불가) | **apiserver `:47099`** | **신설** — artifact 조회 공백 해소 |
| `get nodes/boards/socs/containers` | settingsservice `:8080` | settingsservice `:8080` | 유지(런타임) |
| `describe` / `raw` / `top` / `health` | settingsservice `:8080` | settingsservice `:8080` | 유지 |

> pirictl은 이미 `api_client`(:47099)와 `settings_client`(:8080)를 **둘 다 보유**하므로, artifact 조회 커맨드를 `api_client`로 라우팅만 추가하면 된다(클라이언트 구조 변경 불필요). — 단, 이는 *구현 범위*이며 본 문서는 방향만 확정.

### 4.3 경로 충돌 회피 규칙

- apiserver는 `GET /api/v1/nodes`, `GET /api/v1/nodes/{name}`를 **신설하지 않는다**(settingsservice 점유).
- apiserver의 GET은 **artifact 7종**(Scenario/Package/Model/Volume/Network/Policy/Schedule)으로 한정한다.
- Pod(파생)·Node 조회는 **런타임 상태 영역**으로 보고 settingsservice에 위임한다.

---

## 5. 마이그레이션 / 정리 개요 (구현 제외, 방향만)

| 단계 | 내용 | 범위 |
|---|---|---|
| 1 | apiserver에 artifact 조회 GET 신설(방향 A) | 본 이슈 설계 대상 |
| 2 | pirictl에 `get <artifact>` 커맨드를 `api_client`로 추가 | 후속 구현 이슈 |
| 3 | `/api/artifact` → `/api/v1/artifacts` 경로 버저닝 정렬(Task ⑥) | 본 이슈 설계 대상 |
| 4 | settingsservice 배포 프록시(`/api/v1/yaml` → `/api/artifact`) 존치/정리 판단 | **Out of Scope** (향후 후보) |

> **호환성**: pirictl·Timpani·settingsservice에 대한 영향은 아래와 같다.
> - **pirictl**: artifact 조회 커맨드 추가(가산적 변경). 기존 apply/delete·런타임 조회 무영향.
> - **settingsservice**: 방향 A에서는 **기능 변경 없음**(런타임 조회 존치). 프록시 정리(방향 C)는 별도 이슈.

---

## 6. 이슈 Task ⑦ 대응 정리

| 이슈 항목 | 결정 |
|---|---|
| ⑦ pirictl ↔ apiserver 일원화 | **방향 A**: apiserver에 artifact 조회 GET 신설 → artifact 조회+배포를 `:47099`로 일원화 |
| ⑦ settingsservice 통합 여부 | 런타임/모니터링 조회는 **settingsservice 존치**(데이터 플레인 경계·경로 충돌 회피). 통합 아님 |
| ⑦ 범위 | **방향 검토로 한정**. settingsservice 기능 변경·프록시 제거·pirictl 구현은 Out of Scope |

---