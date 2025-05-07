# API Server

## 1. Introduction

### Major features

API Server는 내,외부 API를 제공하며 Piccolo Artifact 등록 및 준비 작업을 수행 한다.

1. REST API 오픈하여 Piccolo Cloud 와 통신 또는 direct 로 artifact 를 받는 역할을 한다.
1. yaml 형식 string 으로 들어온 artifact를 파싱한다.
1. common 에 정의된 scenario, package 모듈등으로 파싱하여 struct로 생성한다.
1. 파싱 결과를 etcd 에 저장하고 grpc를 통해 filtergateway 로 전달한다.
1. 만약 Bluechi 를 사용할 경우 bluechi 동작에 필요한 파일 생성 후 전파한다.

### Main Dataflow

1. REST API 오픈하여 외부에서 GET 메소드로 scenario 를 비롯한
Piccolo artifact 를 수신한다.
1. yaml 형식 string 으로 들어온 artifact 를 종류 별로 파싱하여 etcd 에 저장한다.
파싱하는 struct 는 common/src/spec/artifact 아래에 정의되어 있다.
1. (선택사항) Bluechi 를 사용할 경우 `.kube`, `.yaml` 파일을 생성 후 각 노드에 전파한다.
1. artifact 중 scenario 는 gRPC 를 통해 filtergateway 로 전달한다.

## 2. File information

```text
apiserver
├── apiserver.md
├── Cargo.toml
└── src
    ├── artifact
    │   ├── data.rs
    │   └── mod.rs
    ├── bluechi
    │   ├── filemaker.rs
    │   ├── mod.rs
    │   └── parser.rs
    ├── grpc
    │   ├── mod.rs
    │   └── sender
    │       ├── filtergateway.rs
    │       └── mod.rs
    ├── main.rs
    ├── manager.rs
    └── route
        ├── api.rs
        └── mod.rs
```

- **main.rs** - manager initialize 수행
- **manager.rs** - 각 모듈 간의 data 흐름을 제어
- **grpc/mod.rs** - gRPC 메시지 송수신 담당
- **grpc/sender/mod.rs** - gRPC 메시지 송신 담당
- **grpc/sender/filtergateway.rs** - filtergateway 로 gRPC 메시지 전달
- **artifact/mod.rs** - string type artifact 를 struct 로 변환하고 etcd에 저장
- **artifact/data.rs** - etcd에 파싱된 결과를 저장하거나 불러오르는 함수 구현
- **route/mod.rs** - Piccolo REST API 의 access point
- **route/api.rs** - Piccolo REST API handler function 모임
- **bluechi/mod.rs** - Bluechi 통합에 필요한 사전 작업 진행
- **bluechi/filemaker.rs** - Bluechi 동작에 필요한 파일 생성 및 다른 node 에 전파
- **bluechi/parser.rs** - 주어진 Package 정보로부터 Model artifact 생성

## 3. Function information

본 문단의 function 정보는 rustdoc 에서 사용하는 주석 형태로 작성되었다.
이에 대해서는 [링크](https://doc.rust-lang.org/stable/rustdoc/index.html) 를 참조하라.

### `main.rs`

```rust
/// Main function of Piccolo API Server
#[tokio::main]
async fn main() {}
```

### `manager.rs`

```rust
/// Launch REST API listener and reload scenario data in etcd
pub async fn initialize() {}

/// Reload all scenario data in etcd
///
/// ### Parametets
/// * None
/// ### Description
/// This function is called once when the apiserver starts.
async fn reload() {}

/// Apply downloaded artifact
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Description
/// write artifact in etcd  
/// (optional) make yaml, kube files for Bluechi  
/// send a gRPC message to gateway
pub async fn apply_artifact(body: &str) -> common::Result<()> {}

/// Withdraw downloaded artifact
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Description
/// delete artifact in etcd  
/// (optional) delete yaml, kube files for Bluechi  
/// send a gRPC message to gateway
pub async fn withdraw_artifact(body: &str) -> common::Result<()> {}
```

### `artifact/mod.rs`

```rust
/// Apply downloaded artifact to etcd
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Returns
/// * `Result(String, String)` - scenario and package yaml in downloaded artifact
/// ### Description
/// Write artifact in etcd
pub async fn apply(body: &str) -> common::Result<(String, String)> {}

/// Delete downloaded artifact to etcd
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Returns
/// * `Result(String)` - scenario yaml in downloaded artifact
/// ### Description
/// Delete scenario yaml only, because other scenario can use a package with same name
pub async fn withdraw(body: &str) -> common::Result<String> {}
```

### `artifact/data.rs`

```rust
/// Read yaml string of artifacts from etcd
///
/// ### Parameters
/// * `artifact_name: &str` - name of the newly released artifact
/// ### Return
/// * `Result<(String)>` - `Ok()` contains yaml string if success
pub async fn read_from_etcd(artifact_name: &str) -> common::Result<String> {}

/// Read all scenario yaml string in etcd
///
/// ### Parameters
/// * None
/// ### Return
/// * `Result<Vec<String>>` - `Ok(_)` contains scenario yaml string vector
pub async fn read_all_scenario_from_etcd() -> common::Result<Vec<String>> {}

/// Write yaml string of artifacts to etcd
///
/// ### Parameters
/// * `key: &str, artifact_name: &str` - etcd key and the name of the newly released artifact
/// ### Return
/// * `Result<()>` - `Ok` if success, `Err` otherwise
pub async fn write_to_etcd(key: &str, artifact_str: &str) -> common::Result<()> {}

/// Write yaml string of artifacts to etcd
///
/// ### Parameters
/// * `key: &str` - data key to delete from etcd
/// ### Return
/// * `Result<()>` - `Ok` if success, `Err` otherwise
pub async fn delete_at_etcd(key: &str) -> common::Result<()> {}
```

### `route/mod.rs`

```rust
/// Serve Piccolo HTTP API service
///
/// ### Parametets
/// None
/// ### Description
/// CORS layer needs to be considerd.
pub async fn launch_tcp_listener() {}

/// Generate appropriate API response based on handler execution result
///
/// ### Parametets
/// * `result: Result<()>` - result of API handler logic
/// ### Description
/// Additional StatusCode may be added depending on the error.
pub fn status(result: common::Result<()>) -> Response {}
```

### `route/api.rs`

```rust
/// Make router type for composing handler and Piccolo service
///
/// ### Parametets
/// None
pub fn router() -> Router {}

/// Notify of new artifact release in the cloud
///
/// ### Parametets
/// * `artifact_name: String` - name of the newly released artifact
async fn notify(artifact_name: String) -> Response {}

/// Apply the new artifacts (scenario, package, etc...)
///
/// ### Parameters
/// * `body: String` - the string in yaml format
async fn apply_artifact(body: String) -> Response {}

/// Withdraw the applied scenario
///
/// ### Parameters
/// * `body: String` - name of the artifact to be deleted
async fn withdraw_artifact(body: String) -> Response {}
```

### `grpc/sender/filtergateway.rs` : Predefined code

```rust
//! Running gRPC message sending to filtergateway

use common::filtergateway::{
    connect_server, filter_gateway_connection_client::FilterGatewayConnectionClient,
    HandleScenarioRequest, HandleScenarioResponse,
};
use tonic::{Request, Response, Status};

/// Send scenario information to filtergateway via gRPC
///
/// ### Parametets
/// * `scenario: HandleScenarioRequest` - wrapped scenario information
/// ### Description
/// This is generated almost automatically by `tonic_build`, so you
/// don't need to modify it separately.
pub async fn send(
    scenario: HandleScenarioRequest,
) -> Result<Response<HandleScenarioResponse>, Status> {
    let mut client = FilterGatewayConnectionClient::connect(connect_server())
        .await
        .unwrap();
    client.handle_scenario(Request::new(scenario)).await
}
```

### `bluechi/mod.rs`

```rust
/// Parsing model artifacts and make files about bluechi
///
/// ### Parametets
/// * `package_str` - whole yaml string of package artifact
/// ### Description
/// Get base `Model` information from package spec  
/// Combine `Network`, `Volume`, parsed `Model` information  
/// Convert `Model` to `Pod`  
/// Make `.kube`, `.yaml` files for bluechi  
/// Copy files to the guest node running Bluechi
pub async fn parse(package_str: String) -> common::Result<()> {}
```

### `bluechi/parser.rs`

```rust
/// Get combined `Network`, `Volume`, parsed `Model` information
///
/// ### Parametets
/// * `p: Package` - Package artifact
/// ### Description
/// Get base `Model` information from package spec  
/// Combine `Network`, `Volume`, parsed `Model` information
pub async fn get_complete_model(p: Package) -> common::Result<Vec<Model>> {}
```

### `bluechi/filemaker.rs`

```rust
/// Make files about bluechi for Pod
///
/// ### Parametets
/// * `pods: Vec<Pod>` - Vector of pods
/// ### Description
/// Make `.kube`, `.yaml` files for bluechi
pub async fn make_files_from_pod(pods: Vec<Pod>) -> common::Result<Vec<String>> {}

/// Make .kube files for Pod
///
/// ### Parametets
/// * `dir: &str, pod_name: &str` - Piccolo yaml directory path and pod name
/// ### Description
/// Make .kube files for Pod
fn make_kube_file(dir: &str, pod_name: &str) -> common::Result<()> {}

/// Make .yaml files for Pod
///
/// ### Parametets
/// * `dir: &str, pod: Pod` - Piccolo yaml directory path and Pod structure
/// ### Description
/// Make .yaml files for Pod
fn make_yaml_file(dir: &str, pod: Pod) -> common::Result<()> {}

/// (under construction) Copy Bluechi files to other nodes
///
/// ### Parametets
/// TBD
/// ### Description
/// TBD
pub fn copy_to_remote_node(file_names: Vec<String>) -> common::Result<()> {}
```

## 4.참고(참조파일 먼저 입력필요)

- `src/common/src/spec/artifact` 아래 위치한 module 과 struct 를 사용한다. (변경 불가)
- gRPC 폴더는 사전 작성된 내용이 있으니 이를 활용하면 된다.
- REST API 로 들어오는 artifact 예제는 `examples/resources` 아래 yaml 파일을 사용한다.

## 5.코드 생성 이후 추가 요구사항

## 6. unittesting
