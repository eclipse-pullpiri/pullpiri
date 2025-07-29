<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Pullpiri

This documents contains developments, testings, static analysis informations.

## Development

### Environment Setup

For detail, refer to [installation](/doc/docs/getting-started.md#installation).

### Build

The first priority is to use Containers, but direct build is also possible. (However, build errors may occur depending on the system.)  
The project is using [cargo](https://doc.rust-lang.org/cargo/) as its build system and command is wrapped in [Makefile](/Makefile).

The binaries and other artifacts (such as the man pages) can be built via:

```bash
# in src directory
make build
# same as
cd src/agent && cargo build
cd src/player && cargo build
cd src/server && cargo build
```

After successfully compiling the binaries, they will be in:

```bash
# only binaries appear in below
[root@HPC src]# ls agent/target/debug/
nodeagent
[root@HPC src]# ls player/target/debug/
actioncontroller    statemanager    filtergateway
[root@HPC src]# ls server/target/debug/
apiserver    monitoringserver    policymanager
```

You can use these directly, but it is recommended to use containers.

```bash
# in root directory
make image
make install
```

For more details, refer to the [Getting started](/doc/docs/getting-started.md)

## Static analysis

### [rustfmt](https://github.com/rust-lang/rustfmt)

Rustfmt formats Rust code according to the official rust style guide.
It enforces consistency across codebases and making code easier to read and review.

#### Using rustfmt

Step 1: Install Rustfmt.

```bash
rustup component add rustfmt 
```

Step 2: Format Your Code: Recursively formats all `.rs` files in the project.

```bash
# in src directory
make fmt
```

### [clippy](https://doc.rust-lang.org/nightly/clippy/)

Clippy is a collection of lints to catch common mistakes and improve Rust code.
It analyzes source code and provides suggestions for idiomatic Rust practices
to performance improvements and potential bugs.

#### Using clippy

Step 1: Install Clippy.

```bash
rustup component add clippy
```

Step 2: Run Clippy: Runs the linter on your project.
It reports warnings and suggestions for your code.

```bash
# in src directory
make clippy
```

*Optional*: Automatically fix warnings: Applies safe automatic fixes to clippy warnings.

```bash
# directory located `Cargo.toml` like `src/server/apiserver`
cargo clippy --fix
```

### [cargo audit](https://crates.io/crates/cargo-audit) - Security vulnerability scanner

Cargo Audit scans your `Cargo.lock` file for crates with known security
vulnerabilities using the RustSec Advisory Database.
It helps you ensure your dependencies are secure.

#### Using cargo-audit

Step 1: Install Cargo Audit.

```bash
cargo install cargo-audit
```

Step 2: Run: Scans the dependency tree for known vulnerabilities and outdated crates.

```bash
# directory located `Cargo.lock` like `src/server`, `src/player`, `src/agent`
cargo audit
```

### [cargo deny](https://crates.io/crates/cargo-deny) - Dependency & License checker

Cargo Deny checks for issues including:It’s used to enforce policies for project dependencies.

- Duplicate crates
- Insecure or unwanted licenses
- Vulnerabilities
- Unmaintained crates

#### Using cargo-deny

Step 1: Install Cargo Deny

```bash
cargo install cargo-deny
```

Step 2: Initialize Configuration (for New components only) -
Creates a default deny.toml file where you can configure license policies, bans and exceptions.

```bash
# directory located `Cargo.toml` like `src/server/apiserver`
# almost all crates already done.
cargo deny init
```

Step 3: Run Check -
Analyzes dependency metadata and reports issues related to licenses, advisories and duplicates.

```bash
# directory located `Cargo.toml` like `src/server/apiserver`
cargo deny check
```

### [cargo udeps](https://crates.io/crates/cargo-udeps) - Unused Dependency Detector

Cargo Udeps identifies unused dependencies in your `Cargo.toml` Keeping unused dependencies can bloat the project and expose unnecessary vulnerabilities."For this requires Nightly Rust to run".

*Caution* : rustc 1.86.0 or above is required. (on July 18th, 2025)

#### Using cargo-udeps

Step 1: Install Cargo Udeps

```bash
cargo install cargo-udeps
rustup install nightly
```

Step 2: Nightly run -
This runs the unused dependency checker and lists packages not being used.

```bash
cargo +nightly udeps
```

## Unit tests

Unit tests can be executed using following commands:

```bash
# in any directories include `Cargo.toml`
cargo test
```

## Integration tests

Some `Pullpiri` modules has `tests` folder.

```bash
# in src/server/apiserver directory
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

For general information of integration testing, refer to [rust doc](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html).

## [cargo tarpaulin](https://crates.io/crates/cargo-tarpaulin) - Code coverage

cargo-tarpaulin is a code coverage tool specifically designed for Rust projects.
It works by instrumenting the Rust code, running the tests,and
generating a detailed report that shows which lines of code were covered during testing.
It supports a variety of Rust test strategies, including unit tests and integration tests.

### Install and run

To use cargo-tarpaulin you need to install it on your system.
This can be done easily via cargo:

```bash
cargo install cargo-tarpaulin

#Once installed, you can run the tool in your project by using the following command:
# in any directories include `Cargo.toml`
cargo tarpaulin
```

## Using Ports

`Pullpiri` uses ports from 47001 ~ 47099.

```Text
for gRPC
47001~

for REST
~47099

for etcd (default)
2379, 2380
```

## Other document

Files for documentation of this project are located in the [doc](/doc/) directory comprising:

- [Getting Started](/doc/docs/getting-started.md): how to run
- [Examples](/examples/README.md): directory containing all files and guides for performing example
- (Deprecated) ~~[pullpiri.drawio](/doc/images/pullpiri.drawio):
file containing all diagrams used for Pullpiri~~
