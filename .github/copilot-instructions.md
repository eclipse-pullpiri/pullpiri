# Pullpiri (Vehicle Service Orchestrator)

**ALWAYS reference these instructions first. If you encounter unexpected information not covered here, you may: (1) search the project documentation, codebase, or official resources; (2) use only the bash commands explicitly listed in this document or standard diagnostic commands (e.g., `ls`, `cat`, `pwd`). If these steps do not resolve your issue, escalate by contacting a maintainer or opening an issue.**

Pullpiri is a Rust-based vehicle service orchestrator framework designed to enable efficient deployment and management of cloud-native in-vehicle services and applications. It uses a microservices architecture with server, agent, and player components that work together to orchestrate containerized workloads.

## Working Effectively

### Bootstrap and Build - NEVER CANCEL BUILDS
- **CRITICAL**: All build and dependency commands below require long timeouts. NEVER CANCEL these operations.
- Install dependencies: `scripts/installdeps.sh` -- takes 8-10 minutes. NEVER CANCEL. Set timeout to 15+ minutes.
- Build all components: `export PATH="$HOME/.cargo/bin:$PATH" && make build` -- takes 5-7 minutes. NEVER CANCEL. Set timeout to 15+ minutes.
- Format check: `export PATH="$HOME/.cargo/bin:$PATH" && scripts/fmt_check.sh` -- takes 1-2 seconds.
- Lint check: `export PATH="$HOME/.cargo/bin:$PATH" && scripts/clippy_check.sh` -- takes 2-3 minutes. NEVER CANCEL. Set timeout to 10+ minutes.

### Environment Setup Requirements
- **Operating System**: Tested on CentOS Stream 9, Ubuntu 20.04+
- **Required Dependencies**: Automatically installed by `scripts/installdeps.sh`:
  - Rust toolchain (rustup, cargo, clippy, rustfmt)
  - etcd v3.5.11 and etcdctl
  - protobuf-compiler
  - libdbus-1-dev, libssl-dev, pkg-config
  - Docker and Docker Compose
  - cargo-deny, cargo2junit
- **Ports Used**: 47001-47099 (gRPC: 47001+, REST: up to 47099), etcd: 2379, 2380

### Build Process
- **Direct Build**: Use `make build` for development builds
- **Release Build**: Use `make release` for optimized builds  
- **Clean**: Use `make clean` to clean all build artifacts
- **Build Time**: Expect 5-7 minutes for full build. Dependencies download adds 2-3 minutes on first build.

### Container Operations
- **Builder Images**: `make builder` -- builds development container images. May require elevated permissions.
- **Runtime Images**: `make image` -- builds final container images for deployment.
- **Install Services**: `make install` -- deploys containers as systemd services. Requires root/sudo.
- **Uninstall Services**: `make uninstall` -- stops and removes deployed services.
- **Container Build Issues**: Container builds may fail with permission errors in some environments. Use direct Rust builds instead.

## Validation and Testing

### Pre-commit Validation - ALWAYS RUN THESE
- **ALWAYS run these commands before committing changes:**
  1. `export PATH="$HOME/.cargo/bin:$PATH" && scripts/fmt_check.sh`
  2. `export PATH="$HOME/.cargo/bin:$PATH" && scripts/clippy_check.sh`
  3. `export PATH="$HOME/.cargo/bin:$PATH" && make build`

### Testing
- **Unit Tests**: `cargo test` in any crate directory (src/common, src/server, src/agent, src/player, src/tools)
- **Integration Tests**: Use `scripts/testNparse.sh` -- WARNING: Requires external dependencies and may fail in restricted environments
- **Manual Service Testing**: After building, you can run individual components:
  - Server: `cargo run --manifest-path=src/server/apiserver/Cargo.toml`
  - Agent: `cargo run --manifest-path=src/agent/nodeagent/Cargo.toml`  
  - Player: `cargo run --manifest-path=src/player/filtergateway/Cargo.toml`

### Validation Scenarios
- **After making changes to any Rust code:**
  1. Run formatting check: `scripts/fmt_check.sh`
  2. Run linting: `scripts/clippy_check.sh` 
  3. Build affected component: `cargo build --manifest-path=src/{component}/Cargo.toml`
  4. Run unit tests: `cargo test --manifest-path=src/{component}/Cargo.toml`

## Project Structure and Navigation

### Key Directories and Files
```
src/
├── common/           # Shared utilities and gRPC definitions
├── server/
│   ├── apiserver/    # Main REST API server (port 47099)
│   ├── policymanager/# Policy management service
│   └── monitoringserver/ # Monitoring service
├── agent/
│   └── nodeagent/    # Node agent for workload management
├── player/
│   ├── filtergateway/    # Gateway service (port 47002)
│   ├── actioncontroller/ # Action controller (port 47001)
│   └── statemanager/     # State management service
└── tools/
    ├── ppr/          # Package management tool
    ├── yamlvalidator/ # YAML validation tool
    └── idl2rs/       # IDL to Rust generator

scripts/              # Build and CI scripts
containers/           # Docker/Podman container definitions
examples/             # Example scenarios and configurations
doc/                  # Documentation
settings.yaml         # Main configuration file
```

### Important Configuration Files
- `settings.yaml` -- Main configuration (etcd endpoints, storage paths, networking)
- `Cargo.toml` files -- Rust project definitions in each component directory
- `containers/piccolo-*.yaml` -- Systemd service definitions for container deployment

## Common Development Tasks

### Adding New Features
1. Identify the component to modify (server, agent, player, tools)
2. Make changes in the appropriate `src/{component}/` directory
3. **ALWAYS** run formatting: `scripts/fmt_check.sh`
4. **ALWAYS** run linting: `scripts/clippy_check.sh`
5. Build and test: `cargo build && cargo test` in the component directory
6. Test integration with other components if applicable

### Working with Dependencies
- Add new Rust dependencies to the appropriate `Cargo.toml` file
- System dependencies should be added to `scripts/installdeps.sh`
- After adding dependencies, rebuild: `make build`

### Debugging Services
- Check etcd health: `etcdctl --endpoints=http://localhost:2379 endpoint health`
- View service logs when using containers: `podman logs {container-name}`
- For development, run services directly with `cargo run` for better debugging output

## Troubleshooting

### Common Issues
- **Build fails with missing dependencies**: Run `scripts/installdeps.sh` to install all required dependencies
- **etcd connection errors**: Ensure etcd is running with `etcdctl endpoint health`
- **Container permission errors**: Use direct Rust builds instead of container builds in restricted environments
- **Port conflicts**: Check that ports 47001-47099 and 2379-2380 are available
- **Formatting/linting failures**: Run `cargo fmt` and `cargo clippy --fix` in the specific component directory

### Build Time Expectations
- **Dependency installation**: 8-10 minutes (first time only)
- **Full build**: 5-7 minutes
- **Incremental build**: 1-3 minutes
- **Formatting check**: 1-2 seconds
- **Linting check**: 2-3 minutes
- **Container image build**: 10-20 minutes (if permissions allow)

### Environment Limitations
- Container builds may not work in all environments due to permission restrictions
- Full integration tests require external services that may not be available
- Some advanced features require root access for systemd service management
- Multi-node functionality requires additional network configuration

## Quick Reference Commands

### Daily Development Workflow
```bash
# Set up environment (once per session)
export PATH="$HOME/.cargo/bin:$PATH"

# Before making changes
make build                    # Verify current state
scripts/fmt_check.sh         # Check formatting
scripts/clippy_check.sh      # Check linting

# After making changes
scripts/fmt_check.sh         # Fix formatting
scripts/clippy_check.sh      # Fix linting issues
make build                   # Verify build works
cargo test --manifest-path=src/{component}/Cargo.toml  # Test your component
```

### Essential Validation Sequence
1. `scripts/fmt_check.sh` (1-2 seconds)
2. `scripts/clippy_check.sh` (2-3 minutes, NEVER CANCEL)
3. `make build` (5-7 minutes, NEVER CANCEL)
4. Component-specific `cargo test` (varies by component)

Always run this sequence before committing changes to ensure CI pipeline success.