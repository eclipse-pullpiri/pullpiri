<!--
SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.

SPDX-License-Identifier: Apache-2.0
-->

# Pullpiri

- [Development](#development)
  - [Environment Setup](#environment-setup)
  - [Code Style](#code-style)
  - [Linting](#linting)
  - [Structure](#structure)
  - [Build](#build)
  - [Static Code Analysis](#static-code-analysis)
  - [Unit Tests](#unit-tests)
  - [Integration Tests](#integration-tests)
  - [Running](#running)
    - [Using Ports](#using-ports)
- [Documentation](#documentation)

## Development

### Environment Setup

For detail, refer to [installation](/doc/docs/getting-started.md#installation).

### Code Style

[rustfmt](https://github.com/rust-lang/rustfmt) is used.

```bash
# in src directory
cargo fmt
```

### Linting

[Clippy](https://doc.rust-lang.org/nightly/clippy/) is used.

```bash
# in src directory
cargo clippy
```

### Structure

Deprecated

<img alt="pullpiri overview" src="../images/overview.png"
width="75%"
height="75%"
/>

For detail, refer to [Structure](/doc/docs/structure.md).

### Build

The first priority is to use Containers, but direct build is also possible. (However, build errors may occur depending on the system.)  
The project is using [cargo](https://doc.rust-lang.org/cargo/) as its build system and command is wrapped in [Makefile](/Makefile).

The binaries and other artifacts (such as the man pages) can be built via:

```bash
make build
# same as
cd src/ && cargo build
```

After successfully compiling the binaries, they will be in `./src/target`.

You can use these directly, but it is recommended to use containers.

```bash
make image
make install
```

For more details, refer to the [Getting started](/doc/docs/getting-started.md)

### Static Code Analysis
Static analysis tools evaluate code to identify potential issues such as code style violations, logical errors and security vulnerabilities.
Static analysis helps to examine the following developers issues:

- Code style issues
- Linting Logic errors
- Security vulnerabilities
- Licensing and safety issues in third-party crates
- Unused dependencies

## Code Style - rustfmt - Rust Code Formatter
Rust fmt formats Rust code according to the official rust style guide. It enforces consistency across codebases and making code easier to read and review.

**Steps to Execute:**

Step 1: Install Rustfmt -->Installs the formatter tool for Rust code.
```bash
rustup component add rustfmt 
```
Step 2: Format Your Code --> Recursively formats all .rs files in the project.
```bash
cargo fmt
```

## clippy - Rust Linter
Clippy is a collection of lints to catch common mistakes and improve Rust code. It analyzes source code and provides suggestions for idiomatic Rust practices to performance improvements and potential bugs.

**Steps to Execute:**

Step 1: Install Clippy
```bash
rustup component add clippy
```
Step 2: Run Clippy --> #Runs the linter on your project. It reports warnings and suggestions for your code.
```bash
cargo clippy
```

Optional: Automatically fix warnings --> #Applies safe automatic fixes to clippy warnings.
```bash
cargo clippy --fix
```


## cargo-audit - Security Vulnerability Scanner
Cargo Audit scans your Cargo.lock file for crates with known security vulnerabilities using the RustSec Advisory Database. It helps you ensure your dependencies are secure.

**Steps to Execute:**

Step 1: Install Cargo Audit -->#Installs the audit tool globally.
```bash
cargo install cargo-audit
```

Step 2: Run Security Audit -->#Scans the dependency tree for known vulnerabilities and outdated crates.
```bash
cargo audit
```

## cargo-deny - Dependency & License Checker
Cargo Deny checks for issues including:It’s used to enforce policies for project dependencies.
- Duplicate crates
- Insecure or unwanted licenses
- Vulnerabilities
- Unmaintained crates


**Steps to Execute:**

Step 1: Install Cargo Deny
```bash
cargo install cargo-deny
```
Step 2: Initialize Configuration (for New components) -->#Creates a default deny.toml file where you can configure license policies, bans and exceptions.
```bash
cargo deny init
```
Step 3: Run Check -->#Analyzes dependency metadata and reports issues related to licenses, advisories and duplicates.
```bash
cargo deny check
```

## cargo-udeps - Unused Dependency Detector
Cargo Udeps identifies unused dependencies in your 'Cargo.toml' Keeping unused dependencies can bloat the project and expose unnecessary vulnerabilities."For this requires Nightly Rust to run".

**Steps to Execute:**

Step 1: Install Cargo Udeps
```bash
cargo install cargo-udeps
rustup install nightly
```

Step 2: Enable Nightly and Run --> #This runs the unused dependency checker and lists packages not being used.
```bash
rustup override set nightly
cargo +nightly udeps
```


** For eg:**
**Static Code Analysis can be executed using following commands:**
in src/server directory
```bash
cargo fmt -- -check
cargo clippy -- -D warnings
```
dependency cargo install cargo-deny (run **cargo deny init** to generatre **deny.toml** for new components)
```bash
cargo deny check
```
dependency cargo install cargo-audit ( requires the component level Cargo.lock file)
```bash
cargo audit
```

### Unit tests

Unit tests can be executed using following commands:

```bash
# in src directory
cargo test
```

### Integration tests

TBD

### Running

The following sections describe how to run the built application(s) locally on one machine.  
Refer to [Getting Started](/doc/docs/getting-started.md).

#### Using Ports

```Text
gRPC
apiserver : 47001
gateway : 47002
statemanager: 47003

REST
apiserver : 47099

etcd : 2379, 2380
```

## Documentation

Files for documentation of this project are located in the [doc](/doc/) directory comprising:

- [Examples](/examples/version-display/): directory containing all files and guides for performing example
- [pullpiri.drawio](/doc/images/pullpiri.drawio) file containing all diagrams used for Pullpiri

<!-- markdownlint-disable-file MD033 no-inline-html -->
