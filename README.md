# For timpani

혹시 궁금하신 점 있으시면 바로 팀즈로 연락 주세요

## 사용법

### 소스 받기

```sh
git clone -b timpani https://github.com/eclipse-pullpiri/pullpiri.git
```

### 노드 설정 - `src/settings.yaml`

아래에서 Host 와 Guest 의 노드 이름과 IP 정보를 맞춰줍니다. 블루치 설정과 동일하게 해주면 됩니다.
```yaml
# SPDX-License-Identifier: Apache-2.0

yaml_storage: /etc/piccolo/yaml
piccolo_cloud: http://0.0.0.0:41234
host:
  name: HPC
  ip: 0.0.0.0
  type: bluechi
guest:
  - name: ZONE
    ip: 192.168.0.1
    type: bluechi
dds:
  idl_path: src/vehicle/dds/idl
  domain_id: 100
  # Removed out_dir - will use Cargo's default OUT_DIR
```

### 팀파니 예제 컨테이너
`examples/resources/timpani_test.yaml` 참조

전에 공유해주신 예제를 `localhost/timpani_test:latest` 라는 이름으로 컨테이너 이미지 빌드햇습니다.

예제 README 의 `docker run` 옵션 항목을 참조하여 최대한 비슷하게 Pod spec 에 맞췄습니다만, 예상과 다르게 동작할 수도 있습니다.

```yaml
apiVersion: v1
kind: Model
metadata:
  name: timpani_test
  annotations:
    io.piccolo.annotations.package-type: timpani_test
    io.piccolo.annotations.package-name: timpani_test
    io.piccolo.annotations.package-network: default
  labels:
    app: timpani_test
spec:
  hostNetwork: true
  containers:
    - name: timpani_test
      image: localhost/timpani_test:latest
      command: ["./sample_apps", "-t", "-p", "50", "-d", "45", "-P", "70", "-a", "2", "-l", "5", "container_task"]
      securityContext:
        privileged: true
        capabilities:
          add: ["SYS_NICE"]
      resources:
        limits:
          cpu: "3"
  terminationGracePeriodSeconds: 0
```

### gRPC command 점검
`src/player/actioncontroller/src/grpc/sender/timpani.rs`

위 코드의 Line 19 부터 아래 내용이 있습니다. 나름대로 예제 참조하여 적당히 작성하였는데, 수정할 부분이 있을지 모르겠습니다.

물론 지금은 급한대로 동작여부 확인용으로 급하게 구성한 것이고, 나중에는 구조에 변화가 있을 것입니다.

```rust
    let request = SchedInfo {
        workload_id: String::from("timpani_test"),
        tasks: vec![TaskInfo {
            name: String::from("sample_apps"),
            priority: 50,
            policy: SchedPolicy::Normal as i32,
            cpu_affinity: 0,
            period: 1000000,
            release_time: 0,
            runtime: 100000,
            deadline: 900000,
            node_id: String::from("HPC"),
            max_dmiss: 3,
        }],
    };
```

### 컨테이너 이미지 만들기
위 사항 모두 확인하셨으면 컨테이너 이미지 만들기

최상위 `src` 폴더에서
```sh
make image
```

### 피콜로 실행
최상위 `src` 폴더에서
```sh
make install
```

### 예제 Pod 실행
```sh
make test
```
이제 팀파니 예제가 실행되고 `podman ps` 를 통해 아래와 같이 확인 가능합니다.
```text
NAME
// .... 중략
timpani_test-timpani_test
```
해당 pod 의 로그는 `podman logs -f timpani_test-timpani_test` 로 확인 가능합니다.
뭐가 뜨긴 하는데 해석하는 법을 모르겠습니다.

### 테스트 진행
.....

### 피콜로 종료
테스트 종료 후 피콜로 종료
```sh
make uninstall
```
만들어진 timpani pod 은 별도로 삭제해줘야 합니다.
```sh
podman pod rm -f timpani_test
```
