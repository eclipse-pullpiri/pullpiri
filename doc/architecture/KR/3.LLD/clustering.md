# PICCOLO 클러스터링 시스템

**문서 번호**: PICCOLO-CLUSTERING-LLD-2025-001  
**버전**: 1.0  
**날짜**: 2025-09-04  
**작성자**: PICCOLO 팀  
**분류**: LLD (Low-Level Design)

## 1. 개요

PICCOLO 프레임워크는 마스터 노드와 서브 노드(워커 노드) 간의 효율적인 클러스터링 메커니즘을 구현하여 분산 환경에서의 모니터링 및 관리를 가능하게 합니다. 본 클러스터링 시스템은 임베디드 환경의 특수성을 고려하여 설계되었으며, 제한된 리소스, 클라우드 연결성, 소규모 클러스터에 최적화되어 있습니다.

### 1.1 목적 및 범위

본 문서는 다음 내용을 포함합니다:
- PICCOLO 클러스터링 아키텍처 상세 설명
- API Server와 NodeAgent의 클러스터링 관련 기능 및 인터페이스
- 클러스터 설정, 배포, 관리 프로세스
- 임베디드 환경에 최적화된 특수 구현 사항

### 1.2 클러스터링 목적 및 원칙

1. **최소화된 아키텍처**
   - 임베디드 환경에 최적화된 경량 설계
   - 2-10개 노드 규모의 소규모 클러스터 지원
   - 리더 선출(Leader Election) 없이 단순화된 마스터-서브 노드 구조
   - 서브 노드에는 NodeAgent만 실행하여 리소스 부담 최소화

2. **하이브리드 연결성**
   - 임베디드 노드와 클라우드 노드 간 연결 지원
   - 다양한 네트워크 조건(불안정한 연결, 제한된 대역폭)에서 동작
   - 오프라인 상태에서의 로컬 운영 및 재연결 시 동기화

3. **중앙화된 상태 관리**
   - etcd에 컨테이너 모니터링 데이터 저장
   - 상태 변경 사항을 StateManager에 효율적으로 전달
   - 마스터 노드에 API Server, FilterGateway, ActionController, StateManager, MonitoringServer 집중 배치

4. **리소스 효율성**
   - Podman 기반 컨테이너 관리로 데몬리스 아키텍처 활용
   - 제한된 하드웨어 사양에서 최소한의 오버헤드
   - 메모리 사용량 최적화된 에이전트 설계

## 2. API Server 클러스터링 기능

API Server는 PICCOLO 클러스터에서 마스터 노드의 핵심 컴포넌트로, 클러스터 구성, 노드 관리, 아티팩트 배포 등을 담당합니다.

### 2.1 주요 기능

1. **노드 관리**
   - 노드 등록 및 인증 처리
   - 클러스터 구성 정보 유지 및 갱신
   - 노드 상태 모니터링 및 활성 확인

2. **아티팩트 배포**
   - 서브 노드에 배포할 아티팩트 관리
   - NodeAgent에 아티팩트 정보 전송
   - 배포 상태 추적 및 보고

3. **클러스터 구성 관리**
   - 클러스터 토폴로지 정보 관리
   - 마스터-서브 노드 관계 설정
   - 노드 역할 및 권한 관리

### 2.2 시스템 아키텍처

PICCOLO 클러스터링 시스템의 아키텍처는 다음과 같이 구성되어 있습니다:

```text
common/
└── src
    └── spec
        └── artifact
            └── node.rs          # 클러스터링 공통 데이터 구조

apiserver/
└── src
    ├── cluster
    │   ├── mod.rs               # 공통 구조 재사용
    │   └── registry.rs          # 노드 레지스트리 구현
    ├── grpc
    │   ├── server.rs            # API Server gRPC 서비스 (수신자)
    │   └── sender
    │       └── nodeagent.rs     # NodeAgent gRPC 클라이언트 (송신자)
    └── manager.rs               # 서비스 관리자

nodeagent/
└── src
    ├── clustering.rs            # API Server gRPC 클라이언트 (송신자)
    └── grpc
        └── receiver.rs          # NodeAgent gRPC 서비스 (수신자)
```

#### 2.2.1 공통 데이터 구조 (`common/spec/artifact/node.rs`)

클러스터링에 사용되는 모든 데이터 구조는 공통 모듈에 정의되어 있습니다:

```rust
/// 노드 사양 (YAML 아티팩트용)
pub struct NodeSpec {
    pub node_info: Option<NodeInfo>,
    pub cluster_config: Option<ClusterConfig>,
}

/// 클러스터 내 노드 정보
pub struct NodeInfo {
    pub node_id: String,
    pub node_name: String,
    pub ip_address: String,
    pub role: NodeRole,
    pub status: NodeStatus,
    pub resources: NodeResources,
    pub labels: HashMap<String, String>,
    pub created_at: i64,
    pub last_heartbeat: i64,
}

/// 노드 클러스터 구성
pub struct ClusterConfig {
    pub cluster_id: String,
    pub master_endpoint: String,
    pub heartbeat_interval: u64,
    pub config: HashMap<String, String>,
}
```

#### 2.2.2 gRPC 통신 패턴

gRPC 서비스는 **수신자 쪽에서 정의**되며, **송신자가 수신자의 API를 호출**하는 패턴을 따릅니다:

1. **API Server → NodeAgent 호출**:
   - API Server는 NodeAgent의 `NodeAgentService`를 호출
   - `apiserver/grpc/sender/nodeagent.rs`에서 클라이언트 구현
   - NodeAgent는 `nodeagent/grpc/receiver.rs`에서 서비스 구현

2. **NodeAgent → API Server 호출**:
   - NodeAgent는 API Server의 `ApiServerService`를 호출
   - `nodeagent/clustering.rs`에서 클라이언트 구현
   - API Server는 `apiserver/grpc/server.rs`에서 서비스 구현

### 2.3 노드 명세 (node.yaml)

클러스터의 노드는 YAML 아티팩트로 정의됩니다. 이는 기존 PICCOLO 아티팩트 시스템과 일관성을 유지하며, 다음과 같은 구조를 가집니다:

```yaml
apiVersion: v1
kind: Node
metadata:
  name: sample-node
  labels:
    node-type: "embedded"
    region: "vehicle-cluster"
    zone: "ecu-1"
spec:
  node_info:
    node_id: "node-12345"
    node_name: "sample-node"
    ip_address: "192.168.1.100"
    role: "Sub"                    # Master 또는 Sub
    status: "Online"               # Online, Offline, Initializing, Error, Maintenance
    resources:
      cpu_cores: 4
      memory_mb: 2048
      disk_gb: 32
      cpu_usage: 15.5
      memory_usage: 45.2
    labels:
      hardware_type: "arm64"
      vehicle_model: "ev-2024"
      ecu_type: "infotainment"
    created_at: 1704063600
    last_heartbeat: 1704067200
  cluster_config:
    cluster_id: "piccolo-cluster-001"
    master_endpoint: "192.168.1.10:50051"
    heartbeat_interval: 30
    config:
      etcd_endpoint: "192.168.1.10:2379"
      log_level: "info"
      monitoring_enabled: "true"
      backup_enabled: "false"
```

이 구조는 `common/spec/artifact/node.rs`에 정의된 데이터 구조와 직접 매핑됩니다.

### 2.4 핵심 인터페이스

#### 2.4.1 NodeAgent와의 gRPC 통신

API Server는 gRPC를 통해 NodeAgent와 통신하여 아티팩트 정보 전달, 노드 상태 확인 등을 수행합니다.

```rust
/// Send artifact information to NodeAgent via gRPC
///
/// ### Parameters
/// * `artifact: ArtifactInfo` - Artifact information
/// * `metadata: Option<Metadata>` - Optional request metadata
/// ### Returns
/// * `Result<Response<ArtifactResponse>, Status>` - Response from NodeAgent
/// ### Description
/// Sends artifact information to NodeAgent using the gRPC client
/// Handles connection management and retries automatically
/// Includes security context and tracing information when available
pub async fn send_artifact(
    artifact: ArtifactInfo,
    metadata: Option<Metadata>
) -> Result<Response<ArtifactResponse>, Status> {
    let mut client = NodeAgentClient::connect(connect_nodeagent())
        .await?;
    
    let request = if let Some(md) = metadata {
        Request::from_parts(md, artifact)
    } else {
        Request::new(artifact)
    };
    
    client.handle_artifact(request).await
}

/// Notify NodeAgent of artifact removal
///
/// ### Parameters
/// * `artifact_id: String` - ID of the artifact to remove
/// ### Returns
/// * `Result<Response<RemoveResponse>, Status>` - Response from NodeAgent
/// ### Description
/// Notifies NodeAgent that an artifact has been removed
pub async fn notify_artifact_removal(
    artifact_id: String
) -> Result<Response<RemoveResponse>, Status> {
    let mut client = NodeAgentClient::connect(connect_nodeagent())
        .await?;
    client.remove_artifact(Request::new(RemoveRequest { artifact_id })).await
}

/// Check NodeAgent connection health
///
/// ### Returns
/// * `bool` - Whether connection is healthy
/// ### Description
/// Verifies connection to NodeAgent is working properly
pub async fn check_nodeagent_connection() -> bool {
    if let Ok(mut client) = NodeAgentClient::connect(connect_nodeagent()).await {
        client.health_check(Request::new(HealthCheckRequest {})).await.is_ok()
    } else {
        false
    }
}
```

#### 2.4.2 노드 등록 및 관리

API Server는 클러스터의 노드 구성을 관리하고, 새로운 노드의 등록 요청을 처리합니다.

```rust
/// Register a new node in the cluster
///
/// ### Parameters
/// * `node_info: NodeRegistrationRequest` - Node information and credentials
/// ### Returns
/// * `Result<NodeRegistrationResponse, NodeRegistrationError>` - Registration result
pub async fn register_node(
    node_info: NodeRegistrationRequest
) -> Result<NodeRegistrationResponse, NodeRegistrationError> {
    // 노드 정보 검증
    validate_node_info(&node_info)?;
    
    // 인증 정보 확인
    authenticate_node(&node_info.credentials)?;
    
    // 노드 정보 저장
    let node_id = store_node_info(&node_info).await?;
    
    // 클러스터 구성 업데이트
    update_cluster_topology(node_id, &node_info.role).await?;
    
    // 응답 생성
    Ok(NodeRegistrationResponse {
        node_id,
        cluster_info: get_cluster_info().await?,
        status: NodeStatus::Registered,
    })
}
```

## 3. NodeAgent 클러스터링 기능

NodeAgent는 각 서브 노드에서 실행되는 에이전트로, 마스터 노드와의 통신 및 로컬 노드 관리를 담당합니다.

### 3.1 주요 기능

1. **노드 식별 및 등록**
   - 노드 고유 번호 할당 및 관리
   - 시스템 정보 수집 및 보고
   - API Server에 노드 등록 요청

2. **클러스터 연결 관리**
   - 마스터 노드와의 연결 유지
   - 네트워크 장애 시 재연결 시도
   - 하트비트 메커니즘을 통한 활성 상태 보고

3. **시스템 상태 체크**
   - 클러스터 연결 전 시스템 준비 상태 확인
   - 하드웨어 리소스, 필수 서비스, 네트워크 가용성 점검
   - 임베디드 환경에 최적화된 경량 체크 수행

### 3.2 클러스터링 프로세스

#### 3.2.1 노드 발견 단계

1. **마스터 노드 구성**
   - 마스터 노드의 API Server는 `node.yaml` 구성 파일을 읽어 관리할 서브 노드 목록 확인
   - 구성 파일에는 각 노드의 호스트명, IP 주소, 역할(임베디드/클라우드), 접근 자격 증명 정보 포함
   - 정적 구성을 기본으로 하되, 클라우드 노드는 동적 발견 지원
   - 임베디드 시스템 시동 시 자동 서브 노드 등록 프로세스

#### 3.2.2 NodeAgent 배포 단계

NodeAgent는 서브 노드에 다음과 같은 설치 스크립트를 통해 배포됩니다:

```bash
#!/bin/bash
# install_nodeagent.sh - NodeAgent 설치 스크립트
# 사용법: ./install_nodeagent.sh <마스터노드IP> [노드타입]

# 매개변수 확인
if [ $# -lt 1 ]; then
    echo "사용법: $0 <마스터노드IP> [노드타입(sub|master)]"
    exit 1
fi

# 매개변수 설정
MASTER_IP=$1
NODE_TYPE=${2:-"sub"}
DOWNLOAD_URL="http://${MASTER_IP}:8080/api/v1/nodeagent/download"
INSTALL_DIR="/opt/piccolo"
CONFIG_DIR="/etc/piccolo"
BINARY_NAME="nodeagent"
LOG_DIR="/var/log/piccolo"
DATA_DIR="/var/lib/piccolo"
RUN_DIR="/var/run/piccolo"

# 필요한 디렉토리 생성
echo "필요한 디렉토리 생성 중..."
mkdir -p ${INSTALL_DIR} ${CONFIG_DIR} ${LOG_DIR} ${DATA_DIR} ${RUN_DIR}

# NodeAgent 바이너리 다운로드
echo "NodeAgent 바이너리 다운로드 중... (${DOWNLOAD_URL})"
if command -v curl &> /dev/null; then
    curl -L ${DOWNLOAD_URL} -o ${INSTALL_DIR}/${BINARY_NAME}
elif command -v wget &> /dev/null; then
    wget ${DOWNLOAD_URL} -O ${INSTALL_DIR}/${BINARY_NAME}
else
    echo "오류: curl 또는 wget이 설치되어 있지 않습니다."
    exit 1
fi

# 실행 권한 부여
chmod +x ${INSTALL_DIR}/${BINARY_NAME}

# 시스템 체크 스크립트 다운로드
echo "시스템 체크 스크립트 다운로드 중..."
if command -v curl &> /dev/null; then
    curl -L "http://${MASTER_IP}:8080/api/v1/nodeagent/scripts/node_ready_check.sh" -o /usr/local/bin/node_ready_check.sh
elif command -v wget &> /dev/null; then
    wget "http://${MASTER_IP}:8080/api/v1/nodeagent/scripts/node_ready_check.sh" -O /usr/local/bin/node_ready_check.sh
fi
chmod +x /usr/local/bin/node_ready_check.sh

# 구성 파일 생성
echo "구성 파일 생성 중..."
cat > ${CONFIG_DIR}/nodeagent.yaml << EOF
nodeagent:
  node_type: "${NODE_TYPE}"
  master_ip: "${MASTER_IP}"
  api_port: 8080
  log_level: "info"
  metrics:
    collection_interval: 5
    batch_size: 50
  etcd:
    endpoint: "${MASTER_IP}:2379"
EOF

# systemd 서비스 파일 생성
echo "systemd 서비스 파일 생성 중..."
cat > /etc/systemd/system/nodeagent.service << EOF
[Unit]
Description=PICCOLO NodeAgent Service
After=network.target

[Service]
Type=simple
ExecStartPre=/usr/local/bin/node_ready_check.sh ${NODE_TYPE}
ExecStart=${INSTALL_DIR}/${BINARY_NAME} --config ${CONFIG_DIR}/nodeagent.yaml
Restart=on-failure
RestartSec=10
Environment=RUST_LOG=info
Environment=MASTER_NODE_IP=${MASTER_IP}

[Install]
WantedBy=multi-user.target
EOF

# 서비스 활성화 및 시작
echo "NodeAgent 서비스 활성화 및 시작 중..."
systemctl daemon-reload
systemctl enable nodeagent.service
systemctl start nodeagent.service

# 설치 결과 확인
if systemctl is-active --quiet nodeagent.service; then
    echo "NodeAgent 설치 및 시작 성공!"
else
    echo "경고: NodeAgent 서비스가 시작되지 않았습니다. 로그를 확인하세요: journalctl -u nodeagent.service"
fi

echo "설치 완료: NodeAgent (마스터 IP: ${MASTER_IP}, 노드 타입: ${NODE_TYPE})"
```

#### 3.2.3 시스템 준비 상태 체크 단계

클러스터에 참여하기 전 노드의 상태를 확인하는 시스템 체크 스크립트:

```bash
#!/bin/bash
# node_ready_check.sh - 노드 클러스터링 전 상태 체크 스크립트
# 임베디드 환경에 최적화된 시스템 체크 수행

NODE_TYPE=${1:-"sub"}  # 기본값은 "sub" 노드
LOG_FILE="/var/log/piccolo/system_check.log"
RESULT_FILE="/var/run/piccolo/node_status"

# 로그 디렉토리 생성
mkdir -p $(dirname $LOG_FILE) $(dirname $RESULT_FILE)

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a $LOG_FILE
}

log "시스템 준비 상태 체크 시작 (노드 타입: $NODE_TYPE)"

# 결과 초기화
echo "status=checking" > $RESULT_FILE

# 1. 기본 시스템 리소스 확인
log "기본 시스템 리소스 확인 중..."

# CPU 로드 확인
CPU_LOAD=$(cat /proc/loadavg | awk '{print $1}')
CPU_CORES=$(grep -c ^processor /proc/cpuinfo)
CPU_LOAD_PER_CORE=$(echo "$CPU_LOAD / $CPU_CORES" | bc -l)

if (( $(echo "$CPU_LOAD_PER_CORE > 0.8" | bc -l) )); then
    log "경고: CPU 부하가 높습니다: $CPU_LOAD (코어당 $(printf "%.2f" $CPU_LOAD_PER_CORE))"
    WARNING_COUNT=$((WARNING_COUNT+1))
else
    log "CPU 부하 정상: $CPU_LOAD (코어당 $(printf "%.2f" $CPU_LOAD_PER_CORE))"
fi

# 메모리 확인
MEM_TOTAL=$(grep MemTotal /proc/meminfo | awk '{print $2}')
MEM_FREE=$(grep MemAvailable /proc/meminfo | awk '{print $2}')
MEM_PERCENT_FREE=$(echo "scale=2; $MEM_FREE * 100 / $MEM_TOTAL" | bc)

if (( $(echo "$MEM_PERCENT_FREE < 20" | bc -l) )); then
    log "경고: 가용 메모리가 부족합니다: ${MEM_PERCENT_FREE}% 남음"
    WARNING_COUNT=$((WARNING_COUNT+1))
else
    log "메모리 상태 정상: ${MEM_PERCENT_FREE}% 가용"
fi

# 2. 필수 서비스 확인
log "필수 서비스 확인 중..."

# Podman 확인
if ! command -v podman &> /dev/null; then
    log "오류: Podman이 설치되어 있지 않습니다."
    ERROR_COUNT=$((ERROR_COUNT+1))
else
    PODMAN_VERSION=$(podman --version | awk '{print $3}')
    log "Podman 설치됨: 버전 $PODMAN_VERSION"
    
    # Podman 서비스 확인
    if ! systemctl is-active --quiet podman.socket 2>/dev/null; then
        log "경고: podman.socket 서비스가 실행 중이 아닙니다."
        WARNING_COUNT=$((WARNING_COUNT+1))
    else
        log "podman.socket 서비스 실행 중"
    fi
fi

# 3. 네트워크 연결 확인
log "네트워크 연결 확인 중..."

# 마스터 노드 연결 확인
MASTER_IP=${MASTER_NODE_IP:-"127.0.0.1"}
if ping -c 1 -W 2 $MASTER_IP &> /dev/null; then
    log "마스터 노드 연결 가능: $MASTER_IP"
    
    # API 서버 포트 확인
    if nc -z -w 2 $MASTER_IP 8080 &> /dev/null; then
        log "API 서버 포트 접속 가능: $MASTER_IP:8080"
    else
        log "오류: API 서버 포트에 접속할 수 없습니다: $MASTER_IP:8080"
        ERROR_COUNT=$((ERROR_COUNT+1))
    fi
    
    # ETCD 포트 확인
    if nc -z -w 2 $MASTER_IP 2379 &> /dev/null; then
        log "ETCD 포트 접속 가능: $MASTER_IP:2379"
    else
        log "오류: ETCD 포트에 접속할 수 없습니다: $MASTER_IP:2379"
        ERROR_COUNT=$((ERROR_COUNT+1))
    fi
else
    log "오류: 마스터 노드에 접속할 수 없습니다: $MASTER_IP"
    ERROR_COUNT=$((ERROR_COUNT+1))
fi

# 4. 상태 평가 및 결과 출력
log "시스템 체크 완료: 오류($ERROR_COUNT), 경고($WARNING_COUNT)"

if [ $ERROR_COUNT -gt 0 ]; then
    log "시스템 준비 상태: 실패 (치명적 오류 발생)"
    echo "status=failed" > $RESULT_FILE
    exit 1
elif [ $WARNING_COUNT -gt 0 ]; then
    log "시스템 준비 상태: 경고 (비치명적 문제 발견)"
    echo "status=warning" > $RESULT_FILE
    exit 0
else
    log "시스템 준비 상태: 양호 (모든 체크 통과)"
    echo "status=ready" > $RESULT_FILE
    exit 0
fi
```

#### 3.2.4 노드 연결 및 인증 단계

NodeAgent는 다음과 같은 Rust 코드를 통해 마스터 노드와 연결합니다:

```rust
/// Connect to master node API server
pub async fn connect_to_master(config: &NodeConfig) -> Result<(), ConnectionError> {
    let master_url = format!("http://{}:{}/api/v1/nodes/register", 
                             config.master_ip, config.api_port);
    
    let node_info = collect_node_info().await?;
    let credentials = generate_credentials(&config)?;
    
    let request = NodeRegistrationRequest {
        node_info,
        credentials,
        node_type: config.node_type.clone(),
    };
    
    let client = Client::new();
    let response = client.post(&master_url)
        .json(&request)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    
    if response.status().is_success() {
        let reg_response: NodeRegistrationResponse = response.json().await?;
        save_node_id(&reg_response.node_id)?;
        save_cluster_info(&reg_response.cluster_info)?;
        
        // 연결 성공 상태 설정
        CONNECTED.store(true, Ordering::SeqCst);
        
        Ok(())
    } else {
        let error_msg = response.text().await?;
        Err(ConnectionError::RegistrationFailed(error_msg))
    }
}

/// Maintain connection with master node
pub async fn maintain_master_connection(config: &NodeConfig) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        // 마스터 노드 연결 상태 확인
        if !CONNECTED.load(Ordering::SeqCst) {
            match connect_to_master(config).await {
                Ok(_) => log::info!("Successfully reconnected to master node"),
                Err(e) => log::error!("Failed to reconnect to master node: {}", e),
            }
            continue;
        }
        
        // 하트비트 전송
        match send_heartbeat().await {
            Ok(_) => log::debug!("Heartbeat sent successfully"),
            Err(e) => {
                log::warn!("Failed to send heartbeat: {}", e);
                CONNECTED.store(false, Ordering::SeqCst);
                break;
            }
        }
    }
}
```

### 3.3 클러스터링 아키텍처

PICCOLO의 클러스터링 아키텍처는 임베디드 환경에 최적화된 소규모 클러스터를 위해 설계되었습니다:

1. **단순화된 마스터-서브 구조**
   - 단일 마스터 노드에 모든 코어 서비스 집중(API Server, FilterGateway, ActionController, StateManager, MonitoringServer)
   - 서브 노드에는 NodeAgent만 실행하여 리소스 부담 최소화
   - 리더 선출 없이 사전 정의된 마스터 노드 사용
   - 임베디드 환경에 최적화된 경량 상태 관리

2. **Podman 기반 컨테이너 관리**
   - 데몬리스 아키텍처로 리소스 사용 최소화
   - rootless 모드 지원으로 보안 강화
   - 임베디드 장치에 적합한 경량 컨테이너 런타임
   - OCI 표준 호환성 유지

3. **etcd 기반 상태 저장**
   - 컨테이너 모니터링 데이터를 etcd에 저장
   - 분산 키-값 저장소로 데이터 일관성 보장
   - 임베디드 환경에 최적화된 경량 구성 사용
   - 제한된 스토리지를 고려한 데이터 보존 정책

4. **하이브리드 연결 모델**
   - 임베디드 노드와 클라우드 노드 간 통합 구조
   - 다양한 네트워크 환경(유선, 무선, 셀룰러)에서 작동
   - 간헐적 연결에서도 강건한 동기화 메커니즘
   - 클라우드 연결성을 활용한 확장 기능

### 3.4 클러스터 토폴로지 유형

PICCOLO는 임베디드 환경과 클라우드 연계를 위한 최적화된 클러스터 토폴로지를 지원합니다:

1. **기본 임베디드 토폴로지**
   - 단일 마스터 노드와 소수의 서브 노드로 구성
   - 마스터 노드에 모든 코어 서비스 집중 배치
   - 간단한 구조로 임베디드 환경의 제한된 리소스에 최적화
   - 2-5개 노드 규모의 소형 시스템에 적합

2. **에지-클라우드 하이브리드 토폴로지**
   - 로컬 임베디드 클러스터와 클라우드 노드 연결
   - 에지에서의 빠른 처리와 클라우드의 확장성 결합
   - 간헐적 클라우드 연결에서도 로컬 운영 가능
   - 데이터 처리 부하를 클라우드와 분담

3. **다중 임베디드 클러스터 토폴로지**
   - 여러 임베디드 클러스터가 상위 마스터 노드에 연결
   - 각 클러스터는 독립적으로 운영 가능
   - 분산 환경에서의 계층적 관리
   - 클러스터 간 격리 및 자원 관리

4. **지역 분산 토폴로지**
   - 지리적으로 분산된 임베디드 시스템 통합
   - 연결 상태에 따른 동적 구성 변경
   - 로컬 클러스터의 자율성 보장
   - 중앙 관리와 분산 처리의 균형

## 4. 클러스터 상태 관리

### 4.1 노드 상태 모니터링

1. **하트비트 메커니즘**
   - 정기적인 하트비트 체크를 통한 노드 활성 상태 확인
   - 응답 없는 노드 감지 및 상태 업데이트
   - 재연결 시 자동 복구 절차 수행

2. **리소스 모니터링**
   - Podman 컨테이너 상태 모니터링 및 etcd에 저장
   - 임베디드 장치의 자원 상태(CPU, 메모리, 디스크, 전원) 모니터링
   - 자원 제약 조건 설정 및 알림 기능

3. **상태 변경 통지**
   - 중요 상태 변경 감지 시 StateManager에 즉시 전달
   - 노드 연결 해제/재연결 시 자동 복구 메커니즘
   - 상태 이벤트 로깅 및 분석

### 4.2 클러스터 구성 동기화

1. **구성 배포**
   - 마스터 노드에서 서브 노드로 구성 변경 사항 전파
   - 부분 업데이트 지원으로 네트워크 트래픽 최소화
   - 구성 버전 관리 및 충돌 해결

2. **정책 동기화**
   - 보안 정책, 모니터링 설정, 리소스 제약 조건 등 동기화
   - 노드 유형 및 역할에 따른 차등 정책 적용
   - 정책 적용 상태 확인 및 보고

## 5. 배포 및 운영

### 5.1 클러스터 초기 설정

1. **마스터 노드 구성**
   - API Server, FilterGateway, ActionController, StateManager, MonitoringServer 설치 및 구성
   - etcd 설정 및 초기화
   - 클러스터 구성 파일 생성

2. **서브 노드 등록**
   - NodeAgent 설치 스크립트 실행
   - 마스터 노드 IP 및 노드 유형 지정
   - 자동 서비스 등록 및 시작

### 5.2 클러스터 확장

1. **신규 노드 추가**
   - 마스터 노드에 노드 정보 사전 등록 또는 동적 발견 설정
   - NodeAgent 설치 및 등록 프로세스 수행
   - 클러스터 토폴로지 자동 업데이트

2. **노드 제거**
   - 안전한 노드 중지 및 제거 절차
   - 클러스터 구성 정보 업데이트
   - 노드 자원 해제 및 정리

### 5.3 장애 복구

1. **노드 장애 감지**
   - 하트비트 실패 시 노드 상태 업데이트
   - 장애 로깅 및 알림 생성
   - 자동 복구 절차 시작

2. **복구 절차**
   - 노드 상태 확인 및 재시작 시도
   - 영구 장애 시 노드 격리 및 관리자 알림
   - 클러스터 재구성 및 워크로드 재배치

## 6. 보안

### 6.1 노드 인증

1. **초기 인증**
   - TLS 기반 노드 인증서 사용
   - 마스터 노드의 인증 기관 활용
   - 안전한 초기 인증 절차

2. **지속적 인증**
   - 주기적 인증서 갱신
   - 토큰 기반 세션 유지
   - 의심스러운 활동 감지 및 차단

### 6.2 통신 보안

1. **암호화 통신**
   - gRPC 통신의 TLS 암호화
   - API 엔드포인트 보안
   - 데이터 무결성 검증

2. **접근 제어**
   - 노드 역할 기반 권한 부여
   - 최소 권한 원칙 적용
   - 자원 접근 제한 및 모니터링

## 7. 참조 및 부록

### 7.1 관련 문서

- HLD/base/piccolo_network(pharos).md - PICCOLO 네트워크 아키텍처
- 3.LLD/apiserver.md - API Server 상세 설계
- 3.LLD/nodeagent.md - NodeAgent 상세 설계

### 7.2 용어 정의

| 용어 | 정의 |
|------|------|
| 마스터 노드 | API Server, FilterGateway, ActionController 등 핵심 서비스를 실행하는 중앙 관리 노드 |
| 서브 노드 | NodeAgent만 실행하는 워커 노드로, 마스터 노드의 관리 대상 |
| NodeAgent | 각 노드에서 실행되는 에이전트로, 노드 상태 모니터링 및 마스터 노드와의 통신 담당 |
| 임베디드 환경 | 제한된 자원(CPU, 메모리, 스토리지)을 가진 장치 환경 |
| Podman | 데몬리스 컨테이너 관리 도구로, Docker 대체제로 사용 |
| etcd | 분산 키-값 저장소로, 클러스터 상태 정보 저장에 사용 |
