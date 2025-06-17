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

In Rust, integration tests are entirely external to your library. They use your library in the same way any other code would, which means they can only call functions that are part of your library’s public API. Their purpose is to test whether many parts of your library work together correctly. Units of code that work correctly on their own could have problems when integrated, so test coverage of the integrated code is important as well. To create integration tests, you first need a tests directory.

Integration tests helps to examine the following developers issues:

- Integration Tests for Binary Crates
- The tests Directory
- Submodules in Integration Tests
- Code Coverage in Rust with cargo-tarpaulin

## Integration Tests for Binary Crates

If our project is a binary crate that only contains a src/main.rs file and doesn’t have a src/lib.rs file, we can’t create integration tests in the tests directory and bring functions defined in the src/main.rs file into scope with a use statement. Only library crates expose functions that other crates can use; binary crates are meant to be run on their own.

This is one of the reasons Rust projects that provide a binary have a straightforward src/main.rs file that calls logic that lives in the src/lib.rs file. Using that structure, integration tests can test the library crate with use to make the important functionality available. If the important functionality works, the small amount of code in the src/main.rs file will work as well, and that small amount of code doesn’t need to be tested.

## The tests Directory

We create a tests directory at the top level of our project directory, next to src. Cargo knows to look for integration test files in this directory. We can then make as many test files as we want, and Cargo will compile each of the files as an individual crate.

Let’s create an integration test. With the code below in the src/lib.rs file, make a tests directory, and create a new file named tests/integration_test.rs. Your directory structure should look like this:
```bash
adder
├── Cargo.lock
├── Cargo.toml
├── src
│   └── lib.rs
└── tests
    └── integration_test.rs
```
**Filename: src/lib.rs**
```rust
pub fn add_two(a: usize) -> usize {
    internal_adder(a, 2)
}

fn internal_adder(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal() {
        let result = internal_adder(2, 2);
        assert_eq!(result, 4);
    }
}
```
**Filename: tests/integration_test.rs**

```rust
use adder::add_two;

#[test]
fn it_adds_two() {
    let result = add_two(2);
    assert_eq!(result, 4);
}
```
Each file in the tests directory is a separate crate, so we need to bring our library into each test crate’s scope. For that reason we add use adder::add_two; at the top of the code, which we didn’t need in the unit tests.

We don’t need to annotate any code in tests/integration_test.rs with #[cfg(test)]. Cargo treats the tests directory specially and compiles files in this directory only when we run cargo test. Run cargo test now:

```bash
$ cargo test
   Compiling adder v0.1.0 (file:///projects/adder)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.31s
     Running unittests src/lib.rs (target/debug/deps/adder-1082c4b063a8fbe6)

running 1 test
test tests::internal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/integration_test.rs (target/debug/deps/integration_test-1082c4b063a8fbe6)

running 1 test
test it_adds_two ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests adder

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The three sections of output include the unit tests, the integration test, and the doc tests. Note that if any test in a section fails, the following sections will not be run. For example, if a unit test fails, there won’t be any output for integration and doc tests because those tests will only be run if all unit tests are passing.

The first section for the unit tests is the same as we’ve been seeing: one line for each unit test and then a summary line for the unit tests.

The integration tests section starts with the line Running tests/integration_test.rs. Next, there is a line for each test function in that integration test and a summary line for the results of the integration test just before the Doc-tests adder section starts.

Each integration test file has its own section, so if we add more files in the tests directory, there will be more integration test sections.

We can still run a particular integration test function by specifying the test function’s name as an argument to cargo test. To run all the tests in a particular integration test file, use the --test argument of cargo test followed by the name of the file:

```bash
$ cargo test --test integration_test
   Compiling adder v0.1.0 (file:///projects/adder)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.64s
     Running tests/integration_test.rs (target/debug/deps/integration_test-82e7799c1bc62298)

running 1 test
test it_adds_two ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
This command runs only the tests in the tests/integration_test.rs file.

## Submodules in Integration Tests

We just wanted to share some code with the other integration test files. we’ll create tests/common/mod.rs. The project directory now looks like this:

```bash
├── Cargo.lock
├── Cargo.toml
├── src
│   └── lib.rs
└── tests
    ├── common
    │   └── mod.rs
    └── integration_test.rs
```

Naming the file this way tells Rust not to treat the common module as an integration test file. Where we move the setup function code into tests/common/mod.rs . Files in subdirectories of the tests directory don’t get compiled as separate crates or have sections in the test output.

After we’ve created tests/common/mod.rs, we can use it from any of the integration test files as a module. Here’s an example of calling the setup function from the it_adds_two test in tests/integration_test.rs:

**Filename: tests/integration_test.rs**

```rust
use adder::add_two;

mod common;

#[test]
fn it_adds_two() {
    common::setup();

    let result = add_two(2);
    assert_eq!(result, 4);
}
```
```bash
$ cargo test
   Compiling adder v0.1.0 (file:///projects/adder)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.89s
     Running unittests src/lib.rs (target/debug/deps/adder-92948b65e88960b4)

running 1 test
test tests::internal ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/integration_test.rs (target/debug/deps/integration_test-92948b65e88960b4)

running 1 test
test it_adds_two ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests adder

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
Unit tests exercise different parts of a library separately and can test private implementation details. Integration tests check that many parts of the library work together correctly, and they use the library’s public API to test the code in the same way external code will use it

## Code Coverage in Rust with cargo-tarpaulin

cargo-tarpaulin is a code coverage tool specifically designed for Rust projects. It works by instrumenting the Rust code, running the tests, and generating a detailed report that shows which lines of code were covered during testing. It supports a variety of Rust test strategies, including unit tests and integration tests.

**Installing cargo-tarpaulin**

To use cargo-tarpaulin you need to install it on your system. This can be done easily via cargo:

```bash
cargo install cargo-tarpaulin

#Once installed, you can run the tool in your project by using the following command:

cargo tarpaulin
```
This will run the tests and generate a code coverage report for your project.

**code coverage reports typically highlight the following areas:**

**-Regions:** Lines of code that were executed.

**-Functions:** Functions that were called during testing.

**-Branches:** Conditional branches (if statements, loops, etc.) that were evaluated.

Rust’s tools like cargo-tarpaulin work by analyzing which regions of your code were executed during test runs.

## Code Coverage Configuration
This section documents how to exclude certain files or functions from code coverage reporting using `cargo tarpaulin`.

-Method A: Using `Cargo.toml` Configuration

-Method B: CLI Flag

-Using Conditional Compilation
  
**Method A: Using `Cargo.toml` Configuration**

Add this to your `Cargo.toml`:
```rust
[package.metadata.tarpaulin]
exclude-files = ["src/main.rs"]
```

Command to Run:
```bash
cargo tarpaulin
```

*Note (Automatically excludes `src/main.rs` without code changes)*

**Method B: CLI Flag**

Alternatively, exclude at runtime:
```bash
cargo tarpaulin --exclude-files src/main.rs
```

**Using Conditional Compilation**

Annotate functions to exclude them from coverage:
```rust
#[cfg(not(tarpaulin_include))]

fn main() {
    process_input();
}
```

**Key Points:**

- Works "without"  adding `tarpaulin` as a dependency
  
- Uses Rust's built-in `cfg` attributes
  
- Only excludes the annotated function

Using `#[tarpaulin::skip]` (Alternative)

If you prefer explicit macros:

1. Enable the feature in `Cargo.toml`:
```rust
   [dev-dependencies]
   tarpaulin = { version = "0.26", features = ["skip"] }
```
2. Annotate functions:
```rust
   #[tarpaulin::skip]

   fn main() {
       process_input();
   }
```

**Usage / Recommended Approach**

-Cargo.toml  : File-level

-#[cfg(not(tarpaulin_include))]:	Function-level	

-#[tarpaulin::skip] :	Function-level	


## Strategies for Maximizing Coverage with cargo-tarpaulin:

To get the most out of cargo-tarpaulin and aim for high coverage:

**Refactor main.rs:** Move all business logic into lib.rs or separate modules that can be more easily tested.

**Write Comprehensive Tests:** Ensure your unit and integration tests cover all functions, methods, and branches within your modules.

**Leverage #[cfg(test)]:** Use conditional compilation to include test-specific code that doesn't interfere with the production build.

## Sample Coverage Report with cargo-tarpaulin:
```bash
2025-06-10T06:11:23.502021Z  INFO cargo_tarpaulin::report: Coverage Results:
|| Uncovered Lines:
|| src/main.rs: 3-4
|| Tested/Total Lines:
|| src/lib.rs: 20/20 +0.00%
|| src/main.rs: 0/2 +0.00%
|| 
90.91% coverage, 20/22 lines covered, +0.00% change in coverage
```
**Explanation of Results:**

**lib.rs:** 100% of the lines in lib.rs are covered by tests.

**main.rs:** No coverage because cargo-tarpaulin does not execute the main() function during tests.

**TOTAL:** The overall coverage is 90.91%, but this includes the main.rs file, which doesn’t contribute to testable code.


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
