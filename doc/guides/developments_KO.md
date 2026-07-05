<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Pullpiri

이 문서는 개발, 테스트, 정적 분석에 관한 정보를 담고 있습니다.

## 개발

### 환경 설정

자세한 내용은 [설치 가이드](/doc/guides/getting-started.md#installation)를 참고하세요.

### 빌드

컨테이너 사용을 우선으로 하지만, 직접 빌드도 가능합니다. (단, 시스템에 따라 빌드 오류가 발생할 수 있습니다.)  
본 프로젝트는 [cargo](https://doc.rust-lang.org/cargo/)를 빌드 시스템으로 사용하며, 명령어는 [Makefile](/Makefile)로 래핑되어 있습니다.

바이너리 및 기타 아티팩트(매뉴얼 페이지 등)는 다음 명령으로 빌드할 수 있습니다:

```bash
# src 디렉토리에서
make build
# 아래와 동일
cd src/agent && cargo build
cd src/player && cargo build
cd src/server && cargo build
```

빌드가 성공하면 바이너리는 다음 위치에 생성됩니다:

```bash
# 바이너리만 표시
[root@HPC src]# ls agent/target/debug/
nodeagent
[root@HPC src]# ls player/target/debug/
actioncontroller    statemanager    filtergateway
[root@HPC src]# ls server/target/debug/
apiserver    monitoringserver    policymanager
```

직접 실행도 가능하지만, 컨테이너 사용을 권장합니다.

```bash
# 루트 디렉토리에서
make image
make install
```

자세한 내용은 [시작하기](/doc/guides/getting-started.md)를 참고하세요.

## 정적 분석

### [rustfmt](https://github.com/rust-lang/rustfmt)

Rustfmt는 공식 Rust 스타일 가이드에 따라 Rust 코드를 포맷합니다.
코드베이스 전반의 일관성을 유지하고 코드를 읽고 리뷰하기 쉽게 만들어 줍니다.

#### rustfmt 사용법

1단계: Rustfmt 설치.

```bash
rustup component add rustfmt 
```

2단계: 코드 포맷: 프로젝트 내 모든 `.rs` 파일을 재귀적으로 포맷합니다.

```bash
# src 디렉토리에서
make fmt
```

### [clippy](https://doc.rust-lang.org/nightly/clippy/)

Clippy는 일반적인 실수를 잡아내고 Rust 코드를 개선하기 위한 lint 모음입니다.
소스 코드를 분석하여 관용적인 Rust 작성법, 성능 개선, 잠재적 버그에 대한 제안을 제공합니다.

#### clippy 사용법

1단계: Clippy 설치.

```bash
rustup component add clippy
```

2단계: Clippy 실행: 프로젝트에 대해 린터를 실행합니다.
경고와 코드 개선 제안을 보고합니다.

```bash
# src 디렉토리에서
make clippy
```

*선택사항*: 경고 자동 수정: clippy 경고에 대해 안전한 자동 수정을 적용합니다.

```bash
# `Cargo.toml`이 위치한 디렉토리 (예: `src/server/apiserver`)
cargo clippy --fix
```

### [cargo audit](https://crates.io/crates/cargo-audit) - 보안 취약점 스캐너

Cargo Audit은 `Cargo.lock` 파일을 RustSec Advisory Database와 대조하여
알려진 보안 취약점이 있는 크레이트를 검사합니다.
의존성 보안을 확보하는 데 도움을 줍니다.

#### cargo-audit 사용법

1단계: Cargo Audit 설치.

```bash
cargo install cargo-audit
```

2단계: 실행: 의존성 트리에서 알려진 취약점 및 오래된 크레이트를 검사합니다.

```bash
# `Cargo.lock`이 위치한 디렉토리 (예: `src/server`, `src/player`, `src/agent`)
cargo audit
```

### [cargo deny](https://crates.io/crates/cargo-deny) - 의존성 & 라이선스 검사기

Cargo Deny는 프로젝트 의존성에 대한 정책을 적용하는 데 사용되며, 다음 문제를 검사합니다:

- 중복 크레이트
- 허용되지 않는 라이선스
- 보안 취약점
- 유지보수되지 않는 크레이트

#### cargo-deny 사용법

1단계: Cargo Deny 설치

```bash
cargo install cargo-deny
```

2단계: 설정 초기화 (신규 컴포넌트에만 해당) -
라이선스 정책, 금지 항목, 예외를 설정할 수 있는 기본 deny.toml 파일을 생성합니다.

```bash
# `Cargo.toml`이 위치한 디렉토리 (예: `src/server/apiserver`)
# 대부분의 크레이트는 이미 완료되어 있습니다.
cargo deny init
```

3단계: 검사 실행 -
의존성 메타데이터를 분석하여 라이선스, 보안 권고, 중복 관련 문제를 보고합니다.

```bash
# `Cargo.toml`이 위치한 디렉토리 (예: `src/server/apiserver`)
cargo deny check
```

### [cargo udeps](https://crates.io/crates/cargo-udeps) - 미사용 의존성 감지기

Cargo Udeps는 `Cargo.toml`에서 사용되지 않는 의존성을 식별합니다. 미사용 의존성은 프로젝트를 비대하게 만들고 불필요한 취약점에 노출시킬 수 있습니다. 이 도구는 Nightly Rust가 필요합니다.

*주의* : rustc 1.86.0 이상이 필요합니다. (2025년 7월 18일 기준)

#### cargo-udeps 사용법

1단계: Cargo Udeps 설치

```bash
cargo install cargo-udeps
rustup install nightly
```

2단계: Nightly 실행 -
미사용 의존성 검사기를 실행하여 사용되지 않는 패키지 목록을 출력합니다.

```bash
cargo +nightly udeps
```

## 단위 테스트

단위 테스트는 다음 명령으로 실행할 수 있습니다:

```bash
# `Cargo.toml`이 포함된 임의의 디렉토리에서
cargo test
```

## 통합 테스트

일부 `Pullpiri` 모듈에는 `tests` 폴더가 있습니다.

```bash
# src/server/apiserver 디렉토리에서
[root@HPC apiserver]# tree
.
├── apiserver.md
├── Cargo.toml
├── src
......
└── tests
    ├── api_integration.rs
    ├── apiserver_init.rs
    ├── filtergateway_integration.rs
    └── manager_integration.rs
```

통합 테스트에 대한 일반적인 정보는 [rust doc](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html)을 참고하세요.

## [cargo tarpaulin](https://crates.io/crates/cargo-tarpaulin) - 코드 커버리지

cargo-tarpaulin은 Rust 프로젝트를 위한 코드 커버리지 도구입니다.
Rust 코드를 계측하고, 테스트를 실행한 후 테스트 중 어떤 코드 라인이 실행되었는지 보여주는 상세 보고서를 생성합니다.
단위 테스트와 통합 테스트를 포함한 다양한 Rust 테스트 전략을 지원합니다.

### 설치 및 실행

cargo-tarpaulin을 사용하려면 먼저 설치가 필요합니다.
cargo를 통해 간단히 설치할 수 있습니다:

```bash
cargo install cargo-tarpaulin

# 설치 후 다음 명령으로 프로젝트에서 도구를 실행할 수 있습니다:
# `Cargo.toml`이 포함된 임의의 디렉토리에서
cargo tarpaulin
```

## 포트 사용

`Pullpiri`는 47001 ~ 47099 범위의 포트를 사용합니다.

```Text
gRPC용
47001~

REST용
~47099

etcd (기본값)
2379, 2380
```

## 기타 문서

이 프로젝트의 문서 파일은 [doc](/doc/) 디렉토리에 위치하며, 다음을 포함합니다:

- [시작하기](/doc/guides/getting-started.md): 실행 방법
- [예제](/examples/README.md): 예제 수행을 위한 모든 파일과 가이드가 포함된 디렉토리
- (Deprecated) ~~[pullpiri.drawio](/doc/images/pullpiri.drawio):
Pullpiri에 사용된 모든 다이어그램이 포함된 파일~~
