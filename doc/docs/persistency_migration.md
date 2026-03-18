# Pullpiri Persistency Integration

## Overview

This integration replaces RocksDB with the Eclipse SCORE persistency library (rust_kvs) across all Pullpiri components. The migration maintains backward compatibility with existing code while providing a centralized, efficient persistence layer.

## Architecture

### Before (RocksDB-based)
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ API Server  │    │Monitor Server│   │Settings Svc │
│             │    │             │    │             │
│   rocksdb   │    │   rocksdb   │    │   rocksdb   │
└──────┬──────┘    └──────┬──────┘    └──────┬──────┘
       │                  │                  │
       └──────────────────┼──────────────────┘
                          │
                    ┌─────▼─────┐
                    │  RocksDB  │
                    │ (External)│
                    └───────────┘
```

### After (Persistency-based)
```text
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ API Server  │    │Monitor Server│   │Settings Svc │
│             │    │             │    │             │
│ persistency │    │ persistency │    │ persistency │
│   client    │    │   client    │    │   client    │
└──────┬──────┘    └──────┬──────┘    └──────┬──────┘
       │                  │                  │
       └──────────────────┼──────────────────┘
                          │ gRPC
                    ┌─────▼─────┐
                    │Persistency│
                    │ Service   │
                    │(rust_kvs) │
                    └───────────┘
```

## Components Modified

### 1. Common Module (`/pullpiri/src/common`)
- **`persistency.rs`**: The main public API module offering storage access to all workspace crates
- **`persistency_client.rs`**: New gRPC client for persistency service
- **`proto/persistency.proto`**: Protocol buffer definitions for the service using rich `KvsValue` types
- **`Cargo.toml`**: Removed `rocksdb` dependency
- **`build.rs`**: Compiles the `persistency.proto` using `tonic_build`

### 2. Persistency Service (`/pullpiri/src/server/persistency-service`)
- **`src/lib.rs`**: Service implementation wrapping rust_kvs with gRPC endpoints (`put`, `get`, `delete`, `batch_put`, etc.)
- **`src/main.rs`**: Standalone service binary serving on port 47007
- **`Cargo.toml`**: Dependencies for rust_kvs and gRPC

### 3. Server, Player, and Agent Components
All `pullpiri` components have been explicitly updated. The usages of `common::rocksdb::*` have been replaced with `common::persistency::*`:
- `server/apiserver/*`
- `server/settingsservice/*`
- `server/monitoringserver/*`
- `player/filtergateway/*`
- `player/statemanager/*`
- `player/actioncontroller/*`
- `agent/nodeagent/*`

## Key Benefits

### 1. Single Initialization Point
- One persistency service process for all Pullpiri components
- Shared storage eliminates data duplication
- Centralized configuration and monitoring

### 2. Improved Performance
- In-process rust_kvs operations (no network overhead for service)
- Efficient gRPC communication between components
- Better memory management with Rust

### 3. Enhanced Reliability
- Built-in data integrity checking (Adler32 checksums)
- Atomic operations within the service
- Snapshot capabilities for backup/restore

### 4. Fully Migrated Codebase
- Direct usage of the persistency wrapper, reducing layers of abstraction.
- Built-in methods like `batch_put` available for optimization.

## Usage

### Starting the Persistency Service

```bash
# Build the service
cd /home/acrn/new_ak/pullpir_persis/pullpiri/src
CARGO_TARGET_DIR=/tmp/persistency_build cargo build -p persistency-service --release

# Run the service
./target/release/persistency-service
```

The service listens on port `47007` (configured in `common/src/lib.rs`).

### Using from Components

All Pullpiri components natively use the persistency backend directly:

```rust
use common::persistency;

async fn example() -> Result<(), String> {
    // Store data
    persistency::put("mykey", "myvalue").await?;
    
    // Retrieve data
    let value = persistency::get("mykey").await?;
    
    // Get all with prefix
    let kvs = persistency::get_all_with_prefix("prefix/").await?;
    
    Ok(())
}
```

### Testing the Integration

Test the entire workspace to ensure storage functions pass using integration mocks:

```bash
cd /home/acrn/new_ak/pullpir_persis/pullpiri/src
CARGO_TARGET_DIR=/tmp/persistency_build cargo test --workspace
```

## Configuration

### Service Configuration
The persistency service uses the same host IP configuration (`setting::get_config().host.ip`).

### Data Storage
- Data is stored in JSON format via rust_kvs
- Default location: Current working directory where `persistency-service` is executed
- Files: `kvs_*.json` and `hash_*.json`

### Network Configuration
- **Port**: 47007 (default, configurable in `common/src/lib.rs`)
- **Protocol**: gRPC (HTTP/2)
- **Security**: Currently unencrypted (can be enhanced with TLS)

## Migration Notes

### For Developers
- Replace all usages of `common::rocksdb` with `common::persistency`.
- New features can use enhanced rust_kvs capabilities, like structured `KvsValue`s.
- Error handling uses the same `Result<T, String>` signature to make migration easy.

### For Deployment
- Start persistency service before other components
- Ensure port 47007 is available
- Consider running as a system service (systemd)

### Data Migration
- Existing RocksDB data needs manual migration
- Use export/import scripts for data transfer
- Test thoroughly in staging environment

## Future Enhancements

1. **TLS Security**: Add mutual TLS for production deployments
2. **Clustering**: Support for distributed persistency service
3. **Monitoring**: Expose metrics and health endpoints
4. **Configuration**: External configuration file support
5. **Backup**: Automated snapshot scheduling

## Troubleshooting

### Service Won't Start
- Check if port 47007 is available: `netstat -tulpn | grep 47007`
- Verify rust_kvs dependencies are installed
- Check host IP configuration in settings.yaml

### Connection Errors
- Ensure persistency service is running
- Verify network connectivity
- Check firewall settings for port 47007

### Data Issues
- Check file permissions in working directory
- Verify JSON files are not corrupted
- Use rust_kvs diagnostic tools

## Files Modified/Created

### New Files
- `/pullpiri/src/common/proto/persistency.proto`
- `/pullpiri/src/common/src/persistency_client.rs`
- `/pullpiri/src/common/src/persistency.rs` (Replaced `rocksdb.rs` functionality)
- `/pullpiri/src/server/persistency-service/`

### Modified Files
- `/pullpiri/src/common/src/rocksdb.rs` (Deprecated/Unused)
- `/pullpiri/src/common/src/lib.rs` (Added persistency modules)
- `/pullpiri/src/common/build.rs` (Added persistency.proto)
- `/pullpiri/src/common/Cargo.toml` (Removed rocksdb dependency)
- `/pullpiri/src/server/Cargo.toml` (Added persistency-service to workspace)
- `/pullpiri/src/Cargo.toml` (Added persistency-service to workspace)
- Replaced `common::rocksdb` with `common::persistency` across 23 sub-components.