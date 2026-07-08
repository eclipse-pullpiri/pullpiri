<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# 시작하기

Pullpiri는 Rust 기반의 차량 서비스 오케스트레이터 프레임워크로, 차량 내 클라우드 네이티브 서비스를 효율적으로 배포하고 관리할 수 있게 합니다. 서버, 에이전트, 플레이어 컴포넌트로 구성된 마이크로서비스 아키텍처를 사용하여 차량 시스템에서 컨테이너화된 워크로드를 오케스트레이션합니다.

이 가이드는 Pullpiri를 실행하는 세 가지 방법을 안내합니다:

| 방법 | 설명 |
|------|------|
| [빠른 시작](#빠른-시작) | GitHub Container Registry의 사전 빌드된 컨테이너 이미지 배포 |
| [소스 빌드](./build.md) | 소스 코드에서 Docker 이미지 및 바이너리 빌드 |
| [튜토리얼](./tutorial.md) | 내장 예제 시나리오를 처음부터 끝까지 실행 |

---

## 시스템 요구사항

Pullpiri는 다음 Linux 배포판에서 테스트되었습니다:

- Ubuntu 24.04 LTS
- CentOS Stream 9

### 최소 하드웨어 요구사항

| 리소스 | 최소 사양 |
|--------|---------|
| CPU | 2코어 |
| RAM | 512 MB |
| 디스크 | 20 GB |
| 아키텍처 | `x86_64` 또는 `aarch64` |

### 소프트웨어 사전 요구사항

| 소프트웨어 | 버전 | 비고 |
|-----------|------|------|
| [Podman](https://podman.io/) | ≥ 4.0.0 | 필수 컨테이너 런타임 |
| Linux 커널 | ≥ 5.10 | 호스트 네트워킹 지원 |

---

## 빠른 시작

GitHub Container Registry에 게시된 사전 빌드된 컨테이너 이미지를 사용하는 것이 가장 빠른 방법입니다.

### Step 1: 사전 요구사항 설치

#### Podman 설치

**Ubuntu 22.04 / 24.04:**

```bash
sudo apt update
sudo apt install -y podman
```

**CentOS Stream 9 / RHEL:**

```bash
sudo dnf install -y podman
```

Podman 설치 확인:

```bash
podman --version
# podman version 4.x.x or higher
```

#### 시스템 준비

```bash
# 필수 디렉토리 생성
sudo mkdir -p /etc/pullpiri
sudo mkdir -p /run/pullpirilog
```

#### 필요 포트 개방

Pullpiri는 다음 TCP 포트를 사용합니다. 방화벽에서 해당 포트를 허용하도록 설정하세요:

| 포트 | 서비스 |
|------|--------|
| 8080 | Settings Service (REST) |
| 47001 | Action Controller (gRPC) |
| 47002 | Filter Gateway (gRPC) |
| 47003 | Monitoring Server (gRPC) |
| 47004 | NodeAgent (gRPC) |
| 47005 | Policy Manager (gRPC) |
| 47006 | State Manager (gRPC) |
| 47007 | RocksDB Service (gRPC) |
| 47098 | API Server (gRPC) |
| 47099 | API Server (REST) |

**Ubuntu (ufw):**

먼저 ufw 활성화 여부를 확인합니다:

```bash
sudo ufw status
# Status: active   → 아래 명령어 실행
# Status: inactive → 방화벽이 비활성화 상태, 포트가 이미 열려 있으므로 이 단계 생략
```

ufw가 활성화된 경우 필요한 포트를 개방합니다:

```bash
sudo ufw allow 8080/tcp
sudo ufw allow 47001:47007/tcp
sudo ufw allow 47098:47099/tcp
sudo ufw reload
sudo ufw status numbered
```

> **참고:** ufw가 비활성화 상태인 경우(Ubuntu 기본 설치 시 비활성), 모든 포트가 이미 접근 가능하므로 방화벽 설정이 필요하지 않습니다.

**CentOS Stream 9 / RHEL (firewalld):**

먼저 firewalld 실행 여부를 확인합니다:

```bash
sudo systemctl is-active firewalld
# active   → 아래 명령어 실행
# inactive → 방화벽이 비활성화 상태, 포트가 이미 열려 있으므로 이 단계 생략
```

firewalld가 활성화된 경우 필요한 포트를 개방합니다:

```bash
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --permanent --add-port=47001-47007/tcp
sudo firewall-cmd --permanent --add-port=47098-47099/tcp
sudo firewall-cmd --reload
sudo firewall-cmd --list-ports
```

### Step 2: 저장소 클론

설치 스크립트는 저장소에 포함되어 있습니다:

```bash
git clone https://github.com/eclipse-pullpiri/pullpiri.git
cd pullpiri
```

### Step 3: Pullpiri 배포

설치 스크립트는 GitHub Container Registry에서 컨테이너 이미지를 자동으로 받아 Podman 파드로 배포합니다.

```bash
# root 권한으로 실행 (Podman 파드 및 시스템 설정에 필요)
sudo bash containers/install-pullpiri.sh
```

스크립트 실행 내용:
1. `/etc/pullpiri/settings.yaml` 설정 파일 생성
2. `ghcr.io/eclipse-pullpiri/pullpiri:latest` 이미지 풀 (서버 및 플레이어 컴포넌트)
3. `ghcr.io/mco-piccolo/pullpiri-rocksdb:v11.18.0` 이미지 풀 (RocksDB 스토리지 서비스)
4. `pullpiri-server` Podman 파드 시작 (RocksDB, APIServer, PolicyManager, MonitoringServer, LogService, SettingsService)
5. `pullpiri-player` Podman 파드 시작 (FilterGateway, ActionController, StateManager)
6. GitHub Releases에서 `nodeagent` 바이너리를 다운로드하여 **systemd 서비스** (`nodeagent.service`)로 등록

> **참고:** 컨테이너 이미지를 다운로드하는 첫 실행은 몇 분 정도 소요될 수 있습니다.

### Step 4: 설치 확인

모든 컨테이너가 실행 중인지 확인합니다:

```bash
podman pod ps
# NAME              STATUS    ...
# pullpiri-server   Running   ...
# pullpiri-player   Running   ...

podman ps
# pullpiri-rocksdbservice   Running ...
# pullpiri-apiserver        Running ...
# pullpiri-policymanager    Running ...
# pullpiri-monitoringserver Running ...
# pullpiri-logservice       Running ...
# pullpiri-settingsservice  Running ...
# pullpiri-filtergateway    Running ...
# pullpiri-actioncontroller Running ...
# pullpiri-statemanager     Running ...
```

nodeagent systemd 서비스가 실행 중인지 확인합니다:

```bash
systemctl status nodeagent.service
# ● nodeagent.service - Pullpiri NodeAgent Service
#    Active: active (running) ...
```

API 서버가 수신 대기 중인지 확인합니다:

```bash
HOST_IP=$(hostname -I | awk '{print $1}')
curl -X GET "http://${HOST_IP}:47099/api/notify"
```

### 서비스 포트

| 서비스 | 포트 | 프로토콜 |
|--------|------|---------|
| API Server (REST) | 47099 | REST (HTTP) |
| API Server (gRPC) | 47098 | gRPC |
| Action Controller | 47001 | gRPC |
| Filter Gateway | 47002 | gRPC |
| Monitoring Server | 47003 | gRPC |
| NodeAgent | 47004 | gRPC |
| Policy Manager | 47005 | gRPC |
| State Manager | 47006 | gRPC |
| RocksDB Service | 47007 | gRPC |
| Settings Service | 8080 | REST (HTTP) |

### 제거

모든 Pullpiri 컨테이너와 파드를 중지하고 제거하려면:

```bash
sudo make uninstall
# 동일한 명령: sudo bash containers/uninstall-pullpiri.sh
```

---

## 소스 빌드

소스 코드에서 Pullpiri를 빌드해야 하는 경우(예: 커스텀 수정 또는 특정 아키텍처), 자세한 **[빌드 가이드](./build.md)**를 참고하세요.

빌드 가이드에서 다루는 내용:
- 프로젝트 `Dockerfile`로 Docker/Podman 컨테이너 이미지 빌드
- 컨테이너 빌드 스테이지 내에서 모든 Rust 바이너리 컴파일
- 빌드된 이미지를 대상 시스템에 설치

---

## 튜토리얼

Pullpiri를 설치하고 실행한 후, **[튜토리얼](./tutorial.md)**을 따라 첫 번째 차량 서비스 시나리오를 배포해 보세요.

튜토리얼 내용:
- `examples/` 디렉토리의 내장 `helloworld` 시나리오 실행
- 배포된 컨테이너 워크로드 확인
- Scenario → Package → Model 리소스 모델 이해

---

## 설정 참조

주요 설정 파일은 설치 중에 `/etc/pullpiri/settings.yaml` 경로에 자동으로 생성됩니다:

```yaml
host:
  name: <hostname>        # 노드 호스트명 (자동 감지)
  ip: <host-ip>           # 호스트 IP 주소 (자동 감지)
  type: vehicle           # 노드 타입
  role: master            # 노드 역할 (master 또는 nodeagent)
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
```

원격 게스트 노드를 추가하려면(멀티 노드 구성), `/etc/pullpiri/settings.yaml`을 편집합니다:

```yaml
host:
  name: HPC
  ip: 192.168.0.100
  type: vehicle
  role: master
guest:
  - name: ZONE1
    ip: 192.168.0.101
    type: vehicle
    role: nodeagent
```

---

## 추가 자료

- [프로젝트 구조](./structure.md)
- [API 참조](./pullpiri-apis.md)
- [개발 가이드](./developments.md)
- [릴리즈 노트](./release.md)
