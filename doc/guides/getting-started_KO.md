<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# 시작하기

## 시스템 요구사항

Pullpiri는 Ubuntu 24.04에서 테스트되었습니다.

[Podman](https://podman.io/)은 컨테이너 런타임으로 사용되므로 반드시 설치되어 있어야 합니다 (Pullpiri는 버전 4.0.0 이상의 Podman이 필요합니다).  
또한 컨테이너를 사용하지 않고 테스트하려면 [Rust](https://www.rust-lang.org)가 필요합니다.

## 사전 정보

### Pullpiri 설정

설정을 위한 `settings.yaml` 파일이 있습니다. 이 파일은 설치 중에 `/etc/piccolo/settings.yaml` 경로에 자동으로 생성됩니다.

```yaml
yaml_storage: /etc/piccolo/yaml
piccolo_cloud: http://0.0.0.0:41234
host:
  name: HPC
  ip: 0.0.0.0
  type: bluechi
guest:
#  - name: ZONE
#    ip: 192.168.0.1
#    type: nodeagent
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
  # Removed out_dir - will use Cargo's default OUT_DIR
```

- yaml_storage : Podman을 이용해 systemd 서비스를 생성하기 위해 `.kube` 및 `.yaml` 파일이 필요합니다.
- piccolo_cloud : `Packages`와 `scenarios`를 저장하는 저장소 주소입니다.
- host : `bluechi`를 통해 systemd 명령을 전달하기 위해 노드 이름이 필요합니다.
- guest : Bluechi 에이전트 노드 정보입니다.
- dds : 추후 업데이트 예정입니다.

### Pullpiri 모듈

Pullpiri는 여러 모듈로 구성되어 있습니다.
각 모듈에 대한 자세한 내용은 [Structure](/doc/guides/developments.md#structure)를 참고하세요.  
또한 [예제](/examples/README.md)가 도움이 될 것입니다.

## 제한 사항

- 멀티 노드 시스템 및 그에 따른 노드 셀렉터는 아직 완전히 고려되지 않았습니다.
- 원활한 운영을 위해 selinux permissive 모드의 `root` 사용자로 운영하는 것을 권장합니다.
- `/etc/containers/systemd` 폴더는 Pullpiri systemd 서비스 파일에 사용됩니다. 이 경로는 변경할 수 없습니다.
- 아직 초기 버전이므로 컨테이너 시작/중지/업데이트에 시간이 오래 걸릴 수 있습니다.
- 그 외 다른 문제가 발생할 수 있습니다.

## 설치

### 설치 전 준비사항

필요한 패키지 설치, 방화벽 비활성화, selinux permissive 설정이 필요합니다.

```bash
# 방화벽 비활성화
systemctl stop firewalld
systemctl disable firewalld
# 패키지 설치
dnf install git-all make gcc -y
# selinux permissive 설정
setenforce 0
```

설정 변경에 대한 자세한 내용은 [설정](#pullpiri-설정)을 참고하세요.

### 설치 과정

모든 Pullpiri 애플리케이션과 테스트 앱은 컨테이너 내에서 실행됩니다.
컨테이너에 익숙하다면 쉽게 사용할 수 있을 것입니다.
`Pullpiri`는 기본적으로 `podman play`를 사용합니다.
처음 사용하는 경우 [예제](/examples/README.md)를 먼저 따라해보는 것을 권장합니다.

시작하기 전에 Pullpiri 컨테이너 이미지를 빌드해야 합니다.

```sh
make builder
make image
```

*주의* - 성공적인 빌드를 위해서는 최소 20GB의 디스크 공간이 필요합니다.

`apt update` 중 오류가 발생하면 DNS 네임서버를 확인하세요.

시작하려면,

```sh
make install
```

중지하려면,

```sh
make uninstall
```

`podman ps` 명령으로 컨테이너 목록을 확인할 수 있습니다. (infra 컨테이너는 생략됩니다.)

```Text
[root@master pullpiri]# podman ps
CONTAINER ID  IMAGE                                 COMMAND               CREATED         STATUS         PORTS          NAMES
fd03b211e2ac  gcr.io/etcd-development/etcd:v3.5.24  --data-dir=/etcd-...  32 seconds ago  Up 32 seconds  2379-2380/tcp  piccolo-server-etcd
c6fbbb6feca5  localhost/pullpiri-server:latest                            32 seconds ago  Up 31 seconds                 piccolo-server-apiserver
341edada2c33  localhost/pullpiri-agent:latest                             31 seconds ago  Up 31 seconds                 piccolo-agent-nodeagent
eee2153bb581  localhost/pullpiri-player:latest                            31 seconds ago  Up 30 seconds                 piccolo-player-filtergateway
8d8011a24b43  localhost/pullpiri-player:latest                            31 seconds ago  Up 30 seconds                 piccolo-player-actioncontroller

[root@master images]# podman pod ps
POD ID        NAME            STATUS      CREATED             INFRA ID      # OF CONTAINERS
cc169812bd3e  piccolo-player  Degraded    About a minute ago  fb8974d9ba47  4
85eeff5e07cf  piccolo-agent   Running     About a minute ago  518c9482ae00  2
809508bfdc46  piccolo-server  Degraded    About a minute ago  1a738d6106f0  5
```

[Makefile](/Makefile)도 참고하세요.
