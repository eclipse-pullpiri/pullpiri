<!--
SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# scaling_client (한글 가이드)

`scaling_client`는 Dynamic Resource Scaling 기능(#514 / #526)을 위한 최소한의
커맨드라인 클라이언트입니다. 실행 중인 ActionController로
`RequestResourceScaling` RPC를 한 번 전송하며, ActionController가
ResourceManager와 대상 NodeAgent를 거쳐 실행 중인 컨테이너의 CPU / 메모리
제한을 런타임에 변경합니다.

이 클라이언트는 `actioncontroller` 크레이트의 Cargo *example*
(`src/player/actioncontroller/examples/scaling_client.rs`)로 제공되므로,
빌드를 위해 별도의 크레이트나 매니페스트 항목이 필요하지 않습니다.

## 바이너리 빌드

클라이언트는 `src/` 워크스페이스에서 빌드합니다.

```bash
cd src

# 디버그 빌드(컴파일이 빠름):
cargo build -p actioncontroller --example scaling_client

# 릴리스 빌드(권장):
cargo build -p actioncontroller --example scaling_client --release
```

결과 바이너리는 아래 경로에 생성됩니다.

```
src/target/debug/examples/scaling_client      # 디버그 빌드
src/target/release/examples/scaling_client     # 릴리스 빌드
```

편의를 위해 `PATH`가 걸린 위치로 복사해 사용할 수 있습니다.

```bash
cp src/target/release/examples/scaling_client /usr/local/bin/scaling_client
```

> 참고: 컴파일된 바이너리는 빌드 산출물이며 저장소에 커밋하지 않습니다.
> 소스가 변경될 때마다 위 명령으로 다시 빌드하세요.

### 별도 빌드 없이 실행하기

한 번만 실행할 때는 Cargo로 빌드와 실행을 한 번에 할 수 있습니다.

```bash
cd src
cargo run -p actioncontroller --example scaling_client -- <인자들...>
```

## 사용법

```
scaling_client <workload_id> <cpu> <mem_mib> [node] [endpoint]
```

| 인자          | 필수 | 설명                                                                    |
| ------------- | ---- | ----------------------------------------------------------------------- |
| `workload_id` | 예   | 크기를 조정할 컨테이너 이름 또는 id (예: `helloworld_helloworld`).       |
| `cpu`         | 예   | 목표 CPU 제한. 아래 "CPU 단위" 참고.                                     |
| `mem_mib`     | 예   | 목표 메모리 제한(MiB 단위 정수).                                         |
| `node`        | 아니오 | 대상 노드. 호스트명(예: `HPC`) 또는 IP 주소. 아래 "노드 지정" 참고. 기본값은 로컬 노드. |
| `endpoint`    | 아니오 | ActionController gRPC 엔드포인트. 기본값 `http://<host.ip>:47001` (로컬 `/etc/pullpiri/settings.yaml`의 `host.ip`를 사용). |

### CPU 단위

CPU는 밀리코어(millicore)로 표현하며 `1000m == 1 코어`입니다. `cpu` 인자는
정수/소수 코어 또는 쿠버네티스 스타일 밀리코어 접미사를 모두 허용합니다.

| 입력    | 의미           |
| ------- | -------------- |
| `1`     | 1 코어 (1000m) |
| `0.5`   | 500m           |
| `500m`  | 500m           |
| `2000m` | 2 코어         |
| `2`     | 2 코어         |

### 노드 지정

`node` 인자에는 IP 주소 또는 노드 호스트명을 넣을 수 있습니다. 해석은
ActionController(서버측)에서 수행합니다.

- 비어 있으면 -> 로컬 노드(`127.0.0.1`).
- IP 리터럴(예: `10.231.176.123`) -> 그대로 사용.
- 호스트명(예: `HPC`) -> 클러스터 노드 레지스트리(키-값 저장소의
  `cluster/nodes/`)에서 조회해 IP로 변환. 이를 통해 원격 노드에서 실행 중인
  워크로드도 노드 이름으로 지정할 수 있습니다.

호스트명을 해석할 수 없으면(레지스트리에 접근 불가하거나 일치하는 노드가 없음)
입력값을 그대로 전달하므로, 이후 발생하는 연결 오류가 의미 있게 표시됩니다.

### 스케일 방향

`up` / `down` 인자는 없습니다. 스케일 방향은 전적으로 서버측에서 결정됩니다.
ResourceManager가 요청값을 현재 desired 상태와 자원별로 비교하여, 증가하는
자원만 용량을 검사합니다. 값이 같거나 줄어드는 자원은 거부되지 않으며,
호출자가 요청을 잘못 표기해 용량 검사를 우회할 수 없습니다.

### ActionController 엔드포인트 선택

클라이언트는 항상 로컬(마스터) 노드에서 실행되므로, ActionController
엔드포인트는 보통 지정할 필요가 없습니다. `endpoint`를 생략하면 로컬
`/etc/pullpiri/settings.yaml`의 `host.ip`를 읽어 `http://<host.ip>:47001`로
구성합니다(값이 없거나 `0.0.0.0`이면 `127.0.0.1`로 대체). 로컬이 아닌
ActionController를 대상으로 할 때만 `endpoint`를 명시합니다.

참고로 `endpoint`(클라이언트가 접속하는 ActionController)와 `node`(워크로드
크기를 조정할 대상 노드)는 별개입니다. 클라이언트는 항상 하나의
ActionController에 접속하고, 그 ActionController가 대상 노드의 NodeAgent로
런타임 변경을 라우팅합니다.

## 예시

```bash
# 컨테이너 "helloworld_helloworld"를 노드 "HPC"에서 0.5 코어 / 64 MiB로 조정.
# 엔드포인트는 로컬 설정 파일에서 가져옵니다.
scaling_client helloworld_helloworld 0.5 64 HPC

# 밀리코어 접미사로 동일한 요청.
scaling_client helloworld_helloworld 500m 64 HPC

# 호스트명 대신 IP로 노드 지정.
scaling_client my-workload 2 512 192.168.0.10

# ActionController 엔드포인트를 명시적으로 지정(로컬이 아닌 마스터).
scaling_client my-workload 2 512 192.168.0.10 http://192.168.0.20:47001
```

성공 시 출력 예:

```
-> RequestResourceScaling endpoint=http://10.231.176.123:47001 node='HPC' workload='helloworld_helloworld' cpu=500m mem=64MiB
<- success        : true
<- message        : scaling applied and synchronized
<- sync_state     : 3
<- actual_cpu     : 500m
<- actual_memory  : 64 MiB
```

## 사전 준비

클라이언트를 호출하기 전에 아래 서비스들이 실행 중이고 접근 가능해야 합니다.

- ActionController(기본 gRPC 포트 `47001`) - 이 클라이언트가 연결하는 엔드포인트.
- ResourceManager(기본 gRPC 포트 `47008`) - 가용성 검증을 수행하고 desired 상태를 소유.
- NodeAgent(기본 gRPC 포트 `47004`) - Podman REST API를 통해 런타임 변경을 적용.

NodeAgent는 대상 컨테이너를 소유한 런타임의 Podman 소켓에 접근할 수 있어야
합니다. root Podman 저장소에서 실행되는 컨테이너라면, NodeAgent가
`/run/podman/podman.sock`에 접근할 수 있도록(예: root 권한으로) 실행해야 합니다.

## 적용된 제한 확인

Podman은 런타임 `update` 결과를 `inspect`에 반영하지 않습니다. 실제 적용된
값은 컨테이너 cgroup에 있습니다. cgroup v2에서 root Podman 저장소의 컨테이너
기준 예시:

```bash
# 컨테이너 내부에서 실행 중인 프로세스로부터 cgroup scope를 찾은 뒤 제한값을
# 읽습니다 (1 코어 == "100000 100000", 64 MiB == 67108864 바이트).
scope=/sys/fs/cgroup/machine.slice/libpod-<CONTAINER_ID>.scope
cat "$scope/cpu.max"      # 예: "50000 100000"  -> 0.5 코어
cat "$scope/memory.max"   # 예: "67108864"      -> 64 MiB
```

## 워크로드 매니페스트에서 초기 제한 지정

`scaling_client`는 *이미 실행 중인* 컨테이너의 제한을 런타임에 변경합니다.
컨테이너가 생성되는 시점부터 자원 제한을 두려면, 대신 워크로드 매니페스트에
선언합니다. NodeAgent는 컨테이너를 생성할 때
`spec.containers[].resources.limits`를 읽어 Podman API를 통해 CPU / 메모리
제한을 적용합니다.

`Model` 문서의 컨테이너 항목에 `resources.limits` 블록을 추가합니다.
`examples/resources/helloworld_no_condition.yaml`을 예로 들면 `Model` 부분은
다음과 같습니다.

```yaml
apiVersion: v1
kind: Model
metadata:
  name: helloworld
  annotations:
    io.pullpiri.annotations.package-type: helloworld
    io.pullpiri.annotations.package-name: helloworld
    io.pullpiri.annotations.package-network: default
  labels:
    app: helloworld
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: quay.io/podman/hello:latest
      resources:              # 컨테이너 생성 시 적용되는 초기 제한
        limits:
          cpu: "0.2"          # 코어(문자열): "0.2" -> 0.2 코어
          memory: "4Mi"       # Ki / Mi / Gi (x1024) 또는 K / M / G (x1000)
  terminationGracePeriodSeconds: 0
```

`resources.limits` 필드 설명:

| 필드     | 형식                                                                     | 예시     |
| -------- | ------------------------------------------------------------------------ | -------- |
| `cpu`    | 문자열, 정수/소수 코어. `f64`로 파싱되어 `NanoCpus`로 적용.               | `"0.2"`, `"1"`, `"2"` |
| `memory` | 문자열, 순수 바이트 또는 `Ki`/`Mi`/`Gi`(x1024), `K`/`M`/`G`(x1000) 접미사. | `"4Mi"`, `"512Mi"`, `"1Gi"` |

참고:

- 두 값 모두 문자열입니다(따옴표로 감쌉니다).
- 여기서 `cpu`는 코어 단위입니다(`scaling_client`에서 쓰는 밀리코어 형식이
  아님). `"0.5"`는 0.5 코어를 의미합니다.
- 이 제한들은 컨테이너 생성 시점에만 적용됩니다. 실행 중인 컨테이너의 값을
  바꾸려면 워크로드를 재생성하거나 `scaling_client`를 사용하세요.
