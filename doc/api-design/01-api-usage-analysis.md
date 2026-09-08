# [1] [분석] API 사용자 분석

## 1. API Server(47099) REST 엔드포인트별 사용처 매핑

| 엔드포인트 | 메서드 | 실제 호출 주체 (코드 근거) | 실사용 |
| --- | --- | --- | --- |
| /api/notify | GET | 호출 주체 없음 — 정의(`route/api.rs`), 문서, 테스트에만 존재 | 미사용(dead) |
| /api/artifact | POST | pirictl `commands/yaml.rs` → `post_yaml("/api/artifact")`, settingsservice `send_artifact_to_api_server()` (POST) | 실사용 |
| /api/artifact | DELETE | pirictl `delete_yaml("/api/artifact")`, settingsservice `send_artifact_to_api_server()` (DELETE) | 실사용 |

## 2. /api/notify 실사용 검증

**결론: 현재 코드 어디에서도 호출되지 않는, 죽은(dead) 엔드포인트입니다.**

1. **핸들러 자체가 no-op**: `notify(artifact_name)`는 `logd!` 한 줄만 찍고 `Ok` 반환. 실제 아티팩트 다운로드/처리 로직 없음.
2. **클라우드 연동 부분도 비어있음**: `manager.rs`의 `send_download_request()`는 `#[allow(dead_code)]` 빈 함수(TODO).   
3. **호출자 부재**: 전체 소스 코드에서 /api/notify REST 호출 코드 없음. pirictl에도 notify 명령 없음.
4. 문서(`pullpiri-apis.md`)에는 "클라우드에서 새 아티팩트 릴리스 알림 수신"으로 기술돼 있으나 구현/연동 미완성.

## 3. Timpani 연동 검증

**결론: Timpani는 API Server REST(47099)를 전혀 사용하지 않음. 양방향 모두 gRPC.**

- 송신(Pullpiri → Timpani): `proactioncontroller` → `grpc::sender::timpani::add_sched_info()` (gRPC)
- 수신(Timpani → Pullpiri): `statemanager` → `TimpaniReceiver` / `FaultService::notify_fault()` (결함 수신, gRPC)
- proto: `common/proto/external/timpani/*.proto`

**→** 이번 REST 재설계는 Timpani에 직접 영향 없음

## 4. Cloud 연동 검증

**결론: `/api/notify` 외에 Cloud → API Server REST 경로 없음.** "cloud"는 대부분 NodeType/topology 열거값(`NodeType::Cloud`, `TOPOLOGY_TYPE_HYBRID_CLOUD`)일 뿐 REST 호출 아님. 실질적 Cloud 연동은 미구현.

## 5. Dashboard 연동 검증

**결론: dashboard도 API Server(47099) 사용하지 않음.** dashboard는 settingsservice(8080)만 사용함.

## 6. 실제 사용하는 부분

```text
[pirictl] --apply/delete--> POST/DELETE 47099 /api/artifact (text/plain raw YAML) [OK]
[settingsservice]: 8080 --/api/v1/yaml 프록시--> 47099 /api/artifact [OK]
[settingsservice]: 8080 --/api/v1/... <--조회-- pirictl get/describe/top [OK]
[dashboard] : 8080 --/api/v1/*, :47097 logs                 [OK]
[Timpani] <--gRPC--> actioncontroller / statemanager (REST 아님) [OK]
[Cloud] --> /api/notify (미구현 dead endpoint) [X]
```

## 7. 결론

1. 실제 재설계 대상 REST 트래픽은 **/api/artifact(POST/DELETE) 뿐** — 나머지 조회는 8080에 있음.
2. **/api/notify는 dead** — 제거 또는 예약 결정 필요.
3. **Timpani는 REST 무관(gRPC)** — 호환성 영향 없음.
4. **호환성 영향 실질 대상은 2곳**: pirictl `commands/yaml.rs`, settingsservice `send_artifact_to_api_server()`. 이 둘의 `/api/artifact` + `text/plain` 의존만 마이그레이션하면 됨.
5. **조회/배포 이원화(8080 vs 47099)**가 Task⑦(인터페이스 일원화)의 실제 대상임을 확정.
 