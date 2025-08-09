# Error Handling and Logging Implementation

This document describes the comprehensive error handling and logging system implemented for the Pullpiri project.

## Overview

The implementation provides:
- ✅ Custom error types with automatic conversion
- ✅ Error propagation using `tokio::sync::mpsc` channels
- ✅ Structured logging with JSON formatting for production
- ✅ Error statistics and monitoring
- ✅ Performance and security event logging
- ✅ Integration tests and validation

## Features Implemented

### 1. Custom Error Types (`src/common/src/error.rs`)

```rust
#[derive(Debug, Error, Clone)]
pub enum PullpiriError {
    Configuration { message: String },
    Grpc { message: String },
    Etcd { message: String },
    Io { message: String },
    Parse { message: String },
    Runtime { message: String },
    Timeout { timeout_ms: u64 },
    Internal { message: String },
}
```

**Automatic conversions** from:
- `tonic::Status` → `PullpiriError::Grpc`
- `dbus::Error` → `PullpiriError::Runtime`
- `etcd_client::Error` → `PullpiriError::Etcd`
- `std::io::Error` → `PullpiriError::Io`
- `serde_yaml::Error` → `PullpiriError::Parse`
- `serde_json::Error` → `PullpiriError::Parse`
- `String` → `PullpiriError::Runtime`
- `&str` → `PullpiriError::Runtime`

### 2. Error Propagation System (`src/common/src/error_reporting.rs`)

**ErrorReporter**: Reports errors asynchronously via channels
```rust
let error = PullpiriError::runtime("Something went wrong");
error_reporter.report_error(error, Some("context".to_string())).await;
```

**ErrorCollector**: Collects and monitors errors across components
- Tracks error statistics per component
- Monitors error rates and triggers alerts
- Provides centralized error logging

**Usage**:
```rust
let (error_collector, reporter_factory) = create_error_system(1000);
let error_reporter = reporter_factory("component_name".to_string());

// Start collector in background
tokio::spawn(async move { error_collector.start().await; });
```

### 3. Structured Logging (`src/common/src/logging.rs`)

**Environment-aware logging**:
- Development: Human-readable format
- Production (`PULLPIRI_ENV=production`): JSON format for monitoring

**Logging macros**:
```rust
log_operation_start!("database_migration");
log_operation_success!("database_migration", user_id = 123);
log_operation_error!("database_migration", &error, context = "init");
```

**Specialized logging functions**:
```rust
// Performance metrics
log_performance_metric("scenario_processing", 150, true);

// Security events
log_security_event("unauthorized_access", "192.168.1.100", "high");

// System events
log_system_event("config_reload", "actioncontroller", "Settings updated");
```

### 4. Result Extension Trait

```rust
use common::error_reporting::ResultExt;

let result = risky_operation()
    .log_error(&error_reporter, Some("Operation context".to_string()))
    .await?;
```

## Integration Example

### ActionController Integration

The ActionController has been enhanced to demonstrate the error handling and logging system:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging first
    logging::init_logging()?;

    // Create error reporting system  
    let (error_collector, reporter_factory) = create_error_system(1000);
    let error_reporter = reporter_factory("actioncontroller".to_string());

    // Start error collector in background
    let collector_handle = tokio::spawn(async move {
        error_collector.start().await;
    });

    let result = run_service(&error_reporter).await;

    // Clean shutdown
    collector_handle.abort();
    result
}
```

## Sample Log Output

### Development Mode
```
  2025-08-09T15:39:04.818830Z  INFO common::logging: Logging initialized successfully
    at /home/runner/work/pullpiri/pullpiri/src/common/src/logging.rs:57

  2025-08-09T15:39:04.818998Z  INFO common::error_reporting: Error collector service started
    at /home/runner/work/pullpiri/pullpiri/src/common/src/error_reporting.rs:109
```

### Production Mode (JSON)
```json
{"timestamp":"2025-08-09T15:39:20.175722Z","level":"INFO","fields":{"message":"Logging initialized successfully"},"target":"common::logging","threadName":"main","threadId":"ThreadId(1)"}
{"timestamp":"2025-08-09T15:39:20.175904Z","level":"INFO","fields":{"message":"Error collector service started"},"target":"common::error_reporting","threadName":"tokio-runtime-worker","threadId":"ThreadId(5)"}
```

## Configuration

### Environment Variables
- `PULLPIRI_ENV=production` - Enable JSON logging for production
- `RUST_LOG=pullpiri=info` - Set log level
- `PULLPIRI_LOG_FORMAT=json` - Force JSON formatting

### Cargo.toml Dependencies
```toml
[dependencies]
common = { workspace = true }
tracing = "0.1.41"
tracing-subscriber = { version = "0.3.19", features = ["env-filter", "json"] }
```

## Testing Results

### Unit Tests Status
- ✅ Common library: 132 tests passed
- ✅ ActionController: 45 tests passed, 5 expected integration failures (no external services)

**Expected integration test failures**: Tests requiring external gRPC services (PolicyManager, NodeAgent) fail as expected in isolated test environment.

### Error Scenarios Tested
- ✅ Configuration errors (invalid YAML)
- ✅ gRPC communication failures  
- ✅ ETCD operation timeouts
- ✅ D-Bus API errors
- ✅ File I/O errors
- ✅ Serialization/parsing errors

## Benefits

1. **Reliability**: Comprehensive error handling prevents crashes and provides graceful degradation
2. **Observability**: Structured logging enables effective monitoring and debugging
3. **Maintainability**: Centralized error handling reduces code duplication
4. **Performance**: Asynchronous error reporting doesn't block operations
5. **Production-Ready**: JSON logging format integrates with log aggregation systems

## Future Enhancements

- [ ] Add documentation for error handling patterns
- [ ] Implement error rate-based circuit breakers
- [ ] Add metrics collection integration (Prometheus/OpenTelemetry)
- [ ] Create error dashboard for monitoring
- [ ] Add automated alerting based on error thresholds

## Demo

Run the error handling demonstration:
```bash
cd /home/runner/work/pullpiri/pullpiri
cargo run --example error_handling_demo
```

Or test the ActionController with enhanced logging:
```bash
# Development mode (human-readable)
PULLPIRI_ENV=development cargo run --manifest-path=src/player/actioncontroller/Cargo.toml

# Production mode (JSON)
PULLPIRI_ENV=production cargo run --manifest-path=src/player/actioncontroller/Cargo.toml
```