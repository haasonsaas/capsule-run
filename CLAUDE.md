# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building and Testing
```bash
# Quick development cycle
make pre-commit              # Format, lint, and test before committing
make check                   # Check compilation (both with/without seccomp)
make test                    # Run all tests (both feature configurations)
make build-release           # Build optimized release binaries

# Individual commands
cargo build                  # Debug build with default features
cargo build --no-default-features  # Build without seccomp (macOS compatible)
cargo test --lib                   # Run only library tests
cargo test --bin capsule-run       # Run only binary tests
cargo test --test integration_tests # Run integration tests
cargo test executor::monitor::tests::test_timeout_monitor  # Run specific test

# Local CI testing (requires nektos/act)
make test-local-quick        # Fast validation (~2 min)
make test-local-basic        # Basic CI checks (~5 min)
make test-local-comprehensive # Full test suite (~15 min)
./scripts/test-ci-locally.sh feature-matrix  # Test seccomp/no-seccomp
```

### Code Quality
```bash
cargo fmt --all             # Format code
cargo fmt --all -- --check  # Check formatting without changes
cargo clippy -- -D warnings # Lint with warnings as errors
make release-check          # Full release validation
```

## Architecture Overview

### Core Design Philosophy
capsule-run is a **single-binary sandboxed execution engine** designed for AI agents. It prioritizes:
- **Security**: Defense-in-depth isolation using Linux namespaces, seccomp, cgroups
- **Performance**: <50ms startup time, minimal overhead
- **Portability**: Works on Linux (full features) and macOS (reduced features)
- **Integration**: JSON API designed for programmatic use by AI systems

### Architecture Diagrams
See [docs/architecture-diagrams.md](docs/architecture-diagrams.md) for detailed Mermaid diagrams covering:
- System overview and component relationships
- Request processing flow (sequence diagram)
- Platform-specific architecture with conditional compilation
- Security layers and isolation mechanisms
- Resource monitoring data flow
- Error handling architecture
- Testing strategy matrix

### Module Architecture

```
src/
├── main.rs              # CLI interface and argument parsing
├── lib.rs               # Public library interface
├── api/                 # Request/response schemas and validation
│   ├── schema.rs        # ExecutionRequest, ExecutionResponse, etc.
│   └── validation.rs    # Input validation and security checks
├── executor/            # Command execution engine
│   ├── mod.rs          # Main Executor struct and execution logic
│   ├── io.rs           # I/O capture and streaming
│   ├── io_stats.rs     # Process I/O statistics collection
│   └── monitor.rs      # Resource monitoring and timeout handling
├── sandbox/             # Platform-specific isolation
│   ├── mod.rs          # Platform-conditional compilation
│   ├── cgroups.rs      # Linux: Resource limits via cgroups v2
│   ├── namespaces.rs   # Linux: Process/filesystem isolation
│   ├── seccomp.rs      # Linux: Syscall filtering (optional)
│   ├── filesystem.rs   # Linux: Mount namespace and pivot_root
│   └── macos.rs        # macOS: Process limits and monitoring
├── config.rs           # TOML configuration and profiles
└── error.rs            # Structured error handling with codes
```

### Key Data Flow

1. **Request Processing**: `main.rs` → `api/validation.rs` → `ExecutionRequest`
2. **Sandbox Setup**: `Executor::new()` → `Sandbox::new()` → Platform-specific managers
3. **Command Execution**: `Executor::execute()` → Process spawn → Monitoring
4. **Resource Monitoring**: Parallel monitoring threads collect metrics during execution
5. **Response Generation**: Results + metrics → `ExecutionResponse` JSON

### Platform Abstractions

The codebase uses **conditional compilation** for cross-platform support:

- **Linux**: Full sandbox with namespaces, cgroups, seccomp
- **macOS**: Process limits via `setrlimit()`, basic monitoring via `getrusage()`
- **Other platforms**: Stub implementations that compile but provide no isolation

Key pattern: `#[cfg(target_os = "linux")]` vs `#[cfg(target_os = "macos")]`

### Feature Flags

```toml
[features]
default = ["seccomp"]          # Linux with syscall filtering
seccomp = ["libseccomp"]       # Optional seccomp support
```

**Important**: Always test both `--features seccomp` (Linux) and `--no-default-features` (macOS/CI) configurations.

### Security Architecture

**Multi-layer isolation**:
1. **User namespaces**: Map container root to unprivileged host user
2. **Mount namespaces**: Isolated filesystem with `pivot_root`
3. **PID namespaces**: Process isolation
4. **Seccomp filters**: Syscall allowlist (~50 permitted syscalls)
5. **Cgroups v2**: Resource limits and OOM detection

**Error codes** follow pattern: `E{category}{number}` (E1xxx=config, E2xxx=security, etc.)

### Execution Model

- **Async/await**: Uses `tokio` for non-blocking I/O and timeouts
- **Arc<Sandbox>**: Shared ownership for monitoring threads
- **Streaming I/O**: Real-time stdout/stderr capture with size limits
- **Graceful shutdown**: SIGTERM → wait → SIGKILL sequence

## Important Development Notes

### Cross-Platform Testing
Always test both feature configurations:
```bash
cargo test                    # Test with seccomp (Linux)
cargo test --no-default-features  # Test without seccomp (macOS)
```

### Memory Safety
- Uses `Arc<Mutex<>>` for thread-safe seccomp context
- Manual `unsafe impl Send + Sync` for libseccomp wrapper
- Careful lifetime management in monitoring threads

### Error Handling Pattern
```rust
// Structured errors with codes for programmatic handling
return Err(ExecutionError::Timeout { elapsed_ms, timeout_ms }.into());
```

### Configuration Loading
Searches multiple locations: `./capsule.toml`, `~/.config/capsule-run/config.toml`, etc.

### Testing Async Code
Use `#[tokio::test]` for async tests. Monitor tests may be timing-sensitive.

### Local CI with nektos/act
The project includes comprehensive local testing setup. Use `make test-local-*` commands to validate changes before pushing.