<!--
SPDX-License-Identifier: Apache-2.0
-->

# Pullpiri 프로젝트 구조

<img alt="Pullpiri overview" src="../images/overview.png"
width="75%"
height="75%"
/>

## 개요

Pullpiri는 마이크로서비스 아키텍처 기반의 Rust 차량 서비스 오케스트레이터입니다. 프로젝트는 4개의 주요 계층으로 구성됩니다:

```bash
.
├── containers/          # Docker/Podman 컨테이너 정의
├── doc/                 # 문서 및 이미지
├── examples/            # 예제 시나리오 및 설정
├── LICENSES/            # 라이선스 파일
├── scripts/             # 빌드 및 CI/CD 스크립트
└── src/
    ├── agent/           # 원격 노드 에이전트 (게스트 노드에서 실행)
    │   └── nodeagent/
    ├── common/          # 공유 유틸리티 및 gRPC 정의
    ├── player/          # 실행 평면 (조건, 상태, 액션)
    │   ├── actioncontroller/
    │   ├── filtergateway/
    │   └── statemanager/
    ├── server/          # 관리 평면 (아티팩트, 저장소, 정책, 설정, 로그)
    │   ├── apiserver/
    │   ├── logservice/
    │   ├── monitoringserver/
    │   ├── policymanager/
    │   ├── rocksdbservice/
    │   └── settingsservice/
    └── tools/           # CLI 도구 및 유틸리티
        ├── idl2rs/
        ├── pirictl/
        └── rocksdb-inspector/
```

---

## 컴포넌트 상세

### Agent 계층

#### NodeAgent (`src/agent/nodeagent/`)

**목적**: 차량 노드에서 실행되어 컨테이너화된 워크로드를 관리하고 오케스트레이션합니다.

**주요 기능**:
- 클라우드/서버로부터 워크로드 배포 지시 수신
- 컨테이너 생명주기 관리 (생성, 실행, 제거)
- 서버에 노드 상태 및 건강 정보 보고
- 스케줄 작업 및 정책 실행
- gRPC를 통해 서버 컴포넌트와 통신

**아키텍처**:
- 차량 노드 배포를 위해 컴파일된 독립 실행 바이너리
- 다중 아키텍처 크로스 컴파일 지원 (x86_64, aarch64)
- Podman을 컨테이너 런타임으로 사용

---

### Server 계층

서버 계층은 중앙 오케스트레이션, 저장소, 관리 서비스를 제공합니다. Player 계층과 동일한 호스트 노드(예: HPC)에서 함께 실행됩니다.

#### API Server (`src/server/apiserver/`)

**목적**: 아티팩트 배포 및 오케스트레이션의 주요 진입점입니다.

**주요 기능**:
- 포트 47099에서 아티팩트 배포를 위한 REST API 제공
- YAML 형식 아티팩트 파싱 (Scenario, Package, Model, Volume, Network, Node, Policy, Schedule)
- gRPC 서비스(포트 47007)를 통해 RocksDB에 아티팩트 저장
- gRPC를 통해 FilterGateway에 시나리오 업데이트 전달
- 노드 등록 및 메타데이터 관리
- PolicyManager와 협력하여 정책 적용

**주요 인터페이스**:
- REST API: `POST /api/artifact`, `DELETE /api/artifact`, `GET /api/notify`
- gRPC: FilterGateway, PolicyManager, RocksDB 서비스와 통신

#### RocksDB Service (`src/server/rocksdbservice/`)

**목적**: 모든 Pullpiri 아티팩트 및 메타데이터를 위한 영구 키-값 저장소입니다.

**주요 기능**:
- 포트 47007에서 gRPC 인터페이스 제공
- 모든 아티팩트, 시나리오, 정책, 노드 정보 저장
- 중앙 집중식 저장소로 etcd를 대체
- 효율성을 위한 배치 연산 지원
- 재시작 후에도 데이터 영속성 보장

**저장소 키 구조**:
- `Scenario/{name}`: 시나리오 정의
- `Package/{name}`: 패키지 정의
- `Policy/{name}`: 정책 규칙
- `nodes/{hostname}`: 노드 정보
- `node/{node_id}`: 상세 노드 메타데이터

#### Settings Service (`src/server/settingsservice/`)

**목적**: Pullpiri 설정의 형상 관리 및 버전 관리입니다.

**주요 기능**:
- 포트 8080에서 REST API 제공
- `/etc/pullpiri/settings.yaml` 설정 관리
- 설정 이력 조회 및 롤백 기능
- 설정 변경 버전 관리
- 경로별 설정 조회

**엔드포인트**:
- `GET /api/v1/settings/{path}`: 설정 값 조회
- `PUT /api/v1/settings/{path}`: 설정 값 업데이트
- `GET /api/v1/history/{path}/version/{version}`: 특정 버전 조회
- `POST /api/v1/history/{path}/rollback/{version}`: 버전 롤백

#### Policy Manager (`src/server/policymanager/`)

**목적**: 중앙 집중식 정책 관리 및 적용입니다.

**주요 기능**:
- 보안 및 리소스 정책 관리
- 워크로드 배포 시 정책 규칙 적용
- 정책 검증을 위해 다른 컴포넌트와 통신
- RocksDB에 정책 저장

#### Monitoring Server (`src/server/monitoringserver/`)

**목적**: 배포된 워크로드의 메트릭을 수집하고 집계합니다.

**주요 기능**:
- 플레이어 컴포넌트로부터 메트릭 리포트 수신
- 성능 및 건강 데이터 집계
- 대시보드용 메트릭 엔드포인트 제공
- 워크로드 상태 및 리소스 사용량 추적

#### Log Service (`src/server/logservice/`)

**목적**: 모든 Pullpiri 컴포넌트의 중앙 집중식 로깅입니다.

**주요 기능**:
- 모든 컴포넌트의 로그 집계
- 구조화된 로깅 인터페이스 제공
- 로그 검색 및 분석 지원
- 다양한 로그 레벨 및 필터링 지원

---

### Player 계층

플레이어 계층은 워크로드 실행을 관리합니다. Server 계층과 동일한 호스트 노드에서 실행되며, 3개의 협력 컴포넌트로 구성됩니다.

#### Action Controller (`src/player/actioncontroller/`)

**목적**: 배포 액션을 실행하고 워크로드 생명주기를 관리합니다.

**주요 기능**:
- 포트 47001에서 API Server의 gRPC 명령 수신
- Podman을 통해 컨테이너 생성 및 관리
- 워크로드 배포, 업데이트, 제거 액션 실행
- 액션 실행 상태 보고
- 목표 상태 vs 실제 상태 조정(Reconciliation) 처리

#### Filter Gateway (`src/player/filtergateway/`)

**목적**: 차량 상태를 모니터링하고 시나리오 액션을 트리거합니다.

**주요 기능**:
- gRPC를 통해 API Server로부터 시나리오 조건 수신
- 차량 버스 메시지 및 상태 변화 모니터링
- 실시간 시나리오 조건 평가
- 일치하는 시나리오를 State Manager에 전달
- 포트 47002에서 gRPC 통신 수신
- 조건 필터링 및 집계 처리

#### State Manager (`src/player/statemanager/`)

**목적**: 전체 워크로드 상태를 오케스트레이션하고 실행을 조정합니다.

**주요 기능**:
- 배포된 모든 워크로드의 현재 상태 유지
- Filter Gateway로부터 시나리오 트리거 수신
- 워크로드 배포를 위해 Action Controller와 협력
- 워크로드 의존성 및 순서 관리
- Monitoring Server에 상태 변화 보고
- 시스템 일관성 및 조정(Reconciliation) 보장

**데이터 흐름**:
1. Filter Gateway가 조건 일치 감지 → State Manager에 전송
2. State Manager가 필요한 액션 결정
3. State Manager가 Action Controller에 실행 요청
4. Action Controller가 Podman을 통해 워크로드 배포
5. State Manager가 상태 업데이트 후 상태 보고

---

### Common 계층

#### Common (`src/common/`)

**목적**: 공유 유틸리티, 데이터 타입, gRPC 서비스 정의입니다.

**주요 구성 요소**:
- **proto/**: gRPC 프로토콜 정의
  - `apiserver.proto`: API Server 통신
  - `filtergateway.proto`: Filter Gateway 통신
  - `statemanager.proto`: State Manager 통신
  - `nodeagent.proto`: Node Agent 통신
  - `rocksdbservice.proto`: RocksDB 서비스 인터페이스

- **spec/artifact/**: Kubernetes 유사 리소스 정의
  - Scenario, Package, Model, Volume, Network, Node, Policy, Schedule

- **spec/k8s/**: Kubernetes Pod 호환 계층

- **저장소 추상화**:
  - 데이터 영속성을 위한 RocksDB 클라이언트
  - KV 연산 지원 (put, get, delete, 배치 연산)

---

### Tools 계층

#### pirictl (`src/tools/pirictl/`)

**목적**: Settings Service 관리를 위한 커맨드라인 인터페이스입니다.

**기능**:
- CLI에서 직접 설정 조회 및 업데이트
- 설정 이력 조회
- 설정 롤백 수행
- 디버깅 및 운영에 유용

#### rocksdb-inspector (`src/tools/rocksdb-inspector/`)

**목적**: RocksDB 저장소 검사 및 디버깅 도구입니다.

**기능**:
- RocksDB 키 및 값 탐색
- 분석을 위한 데이터 내보내기
- 데이터 무결성 검증
- 데이터 문제 트러블슈팅에 유용

#### idl2rs (`src/tools/idl2rs/`)

**목적**: DDS IDL 파일을 Rust 데이터 구조체로 변환합니다.

**기능**:
- DDS IDL(Interface Definition Language) 파싱
- Rust 구조체 정의 생성
- 차량 버스 메시지 처리 지원
- 일반 데이터 타입 및 중첩 구조체 지원

---

## 데이터 흐름

### 아티팩트 배포 흐름

```
1. 외부 시스템 / 클라우드
   ↓ (REST API - POST /api/artifact)
2. API Server (포트 47099)
   ↓ (YAML 파싱, RocksDB 저장)
3. RocksDB Service (포트 47007)
   ↓ (아티팩트 저장)
4. gRPC → Filter Gateway / Policy Manager
   ↓ (시나리오 및 정책 업데이트)
5. Player 컴포넌트 (Filter Gateway, State Manager, Action Controller)
```

### 시나리오 실행 흐름

```
1. Filter Gateway가 시나리오 수신
   ↓
2. 차량 상태 모니터링
   ↓
3. 조건 충족 → State Manager에 전송
   ↓
4. State Manager가 액션 평가 및 준비
   ↓
5. Action Controller에 배포 요청
   ↓
6. Action Controller가 Podman을 통해 컨테이너 생성
   ↓
7. State Manager에 완료 보고
   ↓
8. State Manager가 상태 업데이트 후 메트릭 보고
```

---

## 통신 프로토콜

### gRPC 서비스 (기본 포트)

| 서비스 | 포트 | 목적 |
|--------|------|------|
| RocksDB | 47007 | 저장소 및 메타데이터 |
| API Server | 내부 gRPC | 아티팩트 조정 |
| Filter Gateway | 47002 | 조건 모니터링 |
| Action Controller | 47001 | 워크로드 실행 |
| State Manager | 내부 gRPC | 오케스트레이션 |

### REST API

| 서비스 | 포트 | 목적 |
|--------|------|------|
| API Server | 47099 | 아티팩트 배포 |
| Settings Service | 8080 | 형상 관리 |

---

## 저장소 아키텍처

Pullpiri는 **RocksDB**를 주요 영구 저장 시스템으로 사용합니다. RocksDB는 다음을 제공합니다:

- **영구 저장소**: 모든 아티팩트 및 메타데이터 보존
- **빠른 키-값 연산**: 효율적인 데이터 접근
- **배치 연산**: 원자적 다중 항목 업데이트
- **확장성**: 대규모 배포 지원
- **gRPC 인터페이스**: 분산 접근 지원

키-값 예시:
- `Scenario/helloworld` → 시나리오 YAML 정의
- `Package/helloworld` → 패키지 YAML 정의
- `Policy/resource-limit` → 정책 정의
- `nodes/HPC` → 노드 IP 주소
- `node/HPC-192.168.1.100` → 상세 노드 메타데이터

---

## 빌드 아티팩트

### 컨테이너 이미지

- `localhost/pullpiri:latest` - 모든 서버 및 플레이어 컴포넌트를 포함한 메인 컨테이너
- `localhost/pullpiri-rocksdb:latest` - RocksDB 서비스 컨테이너

### 컴파일된 바이너리

- 다중 아키텍처 NodeAgent 바이너리 (x86_64, aarch64)
- CLI 도구 (pirictl, rocksdb-inspector, idl2rs)

---

## 개발 참고 사항

- 모든 컴포넌트는 async/await 패턴의 **Rust**로 구현됨
- 효율성과 타입 안전성을 위해 **gRPC** 통신 사용
- 경량 컨테이너화를 위해 **Podman** 컨테이너 관리 사용
- 사람이 읽기 쉬운 **YAML** 형식으로 설정 저장
- 안정성과 성능을 위해 **RocksDB**로 데이터 영속성 보장

<!-- markdownlint-disable-file MD033 no-inline-html -->
