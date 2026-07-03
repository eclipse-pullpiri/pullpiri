<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Pullpiri REST API

Pullpiri REST API는 클라우드 또는 다른 시스템에서 차량 노드로 아티팩트(Artifact)를 배포하고 관리하기 위한 HTTP 기반 인터페이스를 제공합니다.

**API 서버 주소**: `http://<host>:47099`

## 개요

Pullpiri API는 다음과 같은 주요 기능을 제공합니다:

- **아티팩트 배포** (POST /api/artifact): Scenario, Package, Model 등의 아티팩트를 배포
- **아티팩트 철수** (DELETE /api/artifact): 배포된 아티팩트 제거
- **배포 알림** (GET /api/notify): 클라우드에서 새 아티팩트 릴리스 알림 수신

## 엔드포인트

### 1. 아티팩트 배포

새로운 아티팩트(Scenario, Package, Model, Volume, Network, Node, Schedule, Policy)를 배포합니다.

```
POST /api/artifact
```

#### 요청 헤더

| 헤더 | 값 |
|------|-----|
| Content-Type | text/plain (또는 application/x-yaml) |

#### 요청 본문

YAML 형식의 아티팩트 정의. 다중 아티팩트는 `---` 구분자로 분리됩니다.

##### 예시: Scenario 배포

```yaml
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition: ""
  action: update
  target: helloworld
```

##### 예시: Package와 Model 함께 배포

```yaml
apiVersion: v1
kind: Package
metadata:
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources: {}
      volume: {}
      network: {}
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld:latest
  terminationGracePeriodSeconds: 0
```

#### 응답

##### 성공 (200 OK)

```json
"Ok"
```

##### 실패 (405 또는 기타 오류)

```json
"Error message describing the failure"
```

#### 응답 상태 코드

| 코드 | 설명 |
|------|------|
| 200 | 아티팩트 배포 성공 |
| 405 | 잘못된 요청 또는 처리 오류 |

---

### 2. 아티팩트 철수

배포된 아티팩트를 제거합니다.

```
DELETE /api/artifact
```

#### 요청 헤더

| 헤더 | 값 |
|------|-----|
| Content-Type | text/plain |

#### 요청 본문

제거할 아티팩트의 이름 (문자열)

##### 예시

```
helloworld
```

#### 응답

##### 성공 (200 OK)

```json
"Ok"
```

##### 실패 (405 또는 기타 오류)

```json
"Error message describing the failure"
```

#### 응답 상태 코드

| 코드 | 설명 |
|------|------|
| 200 | 아티팩트 철수 성공 |
| 405 | 잘못된 요청 또는 처리 오류 |

---

### 3. 배포 알림

클라우드에서 새로운 아티팩트가 릴리스되었음을 API 서버에 알립니다.

```
GET /api/notify
```

#### 요청 파라미터

| 파라미터 | 타입 | 필수 | 설명 |
|---------|------|------|------|
| artifact_name | String | 예 | 새로 릴리스된 아티팩트의 이름 |

#### 예시

```
GET /api/notify?artifact_name=helloworld
```

#### 응답

##### 성공 (200 OK)

```json
"Ok"
```

##### 실패 (405 또는 기타 오류)

```json
"Error message describing the failure"
```

#### 응답 상태 코드

| 코드 | 설명 |
|------|------|
| 200 | 알림 수신 성공 |
| 405 | 잘못된 HTTP 메서드 |

---

## 아티팩트 종류

Pullpiri API가 지원하는 아티팩트 종류는 다음과 같습니다:

| Kind | 설명 |
|------|------|
| Scenario | 특정 조건에서 실행될 작업 정의 |
| Package | 배포할 애플리케이션 패키지 정의 |
| Model | Kubernetes Pod 사양으로 정의된 컨테이너 모델 |
| Volume | 저장소 볼륨 정의 |
| Network | 네트워크 구성 정의 |
| Node | 노드 정보 정의 |
| Schedule | 스케줄 기반 작업 정의 |
| Policy | 정책 규칙 정의 |

---

## 사용 예시

### cURL을 이용한 Scenario 배포

```bash
curl -X POST http://localhost:47099/api/artifact \
  -H "Content-Type: text/plain" \
  -d @helloworld_scenario.yaml
```

### cURL을 이용한 아티팩트 철수

```bash
curl -X DELETE http://localhost:47099/api/artifact \
  -H "Content-Type: text/plain" \
  -d "helloworld"
```

### cURL을 이용한 배포 알림

```bash
curl -X GET "http://localhost:47099/api/notify?artifact_name=helloworld"
```

---

## 오류 처리

모든 API 응답은 성공/실패 여부를 HTTP 상태 코드로 표시합니다:

- **200 OK**: 요청이 성공적으로 처리됨
- **405 Method Not Allowed**: 요청의 HTTP 메서드가 허용되지 않거나 처리 중 오류 발생

오류 응답의 본문에는 상세한 오류 메시지가 포함됩니다.

---

## 참고 사항

- 모든 요청은 유효한 YAML 또는 문자열 형식이어야 합니다.
- 다중 아티팩트 배포 시 YAML 구분자(`---`)로 각 아티팩트를 분리합니다.
- API 서버는 수신한 아티팩트를 RocksDB에 저장하고 필터게이트웨이 등 다른 컴포넌트에 전달합니다.