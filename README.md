# capsule-run

A lightweight, secure sandboxed command execution utility designed for AI agents and automated systems.

## Features

- **Secure Isolation**: Linux namespaces, seccomp filters, and cgroups v2
- **Resource Limits**: Memory, CPU, process count, and I/O controls
- **Fast Startup**: Sub-50ms cold start times
- **Zero Dependencies**: Single static binary
- **JSON API**: Native integration with AI frameworks
- **Comprehensive Monitoring**: Resource usage tracking and metrics

## Quick Start

### Basic Usage

```bash
# Execute a simple command
capsule-run -- echo "Hello, World!"

# With resource limits
capsule-run --memory 256M --timeout 5000 -- python script.py

# JSON mode for programmatic use
echo '{"command":["python","-c","print(42)"]}' | capsule-run --json
```

### JSON API

```json
{
  "command": ["python", "-c", "print('Hello from sandbox')"],
  "environment": {"PYTHONPATH": "/workspace"},
  "timeout_ms": 5000,
  "resources": {
    "memory_bytes": 268435456,
    "cpu_shares": 1024,
    "max_output_bytes": 1048576
  },
  "isolation": {
    "network": false,
    "readonly_paths": ["/usr", "/bin"],
    "writable_paths": ["/tmp"],
    "working_directory": "/workspace"
  }
}
```

## Installation

### Pre-built Binaries

Download from the [releases page](https://github.com/haasonsaas/capsule-run/releases).

### Build from Source

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/haasonsaas/capsule-run.git
cd capsule-run
cargo build --release

# The binary will be at target/release/capsule-run
```

## Requirements

- Linux kernel 5.10+ (for cgroups v2)
- x86_64 or aarch64 architecture
- User namespaces enabled (usually default on modern systems)

## Security

capsule-run implements defense-in-depth security:

1. **User Namespaces**: Maps container root to unprivileged host user
2. **Mount Namespaces**: Isolated filesystem with pivot_root
3. **PID Namespaces**: Process isolation with custom init
4. **Seccomp Filters**: Strict syscall allowlist (~50 permitted syscalls)
5. **Capability Dropping**: All Linux capabilities removed
6. **Resource Limits**: cgroups v2 enforcement

### Threat Mitigation

- Container escape prevention
- Fork bomb protection
- Memory exhaustion safeguards
- Kernel exploit mitigation
- Path traversal prevention

## AI Framework Integration

### LangChain

```python
from langchain.tools import Tool
import subprocess
import json

class SecureCodeExecutor(Tool):
    name = "secure_code_executor"
    description = "Execute code safely with resource limits"
    
    def _run(self, code: str, language: str = "python") -> str:
        request = {
            "command": self._get_command(code, language),
            "resources": {"memory_bytes": 256 * 1024 * 1024},
            "timeout_ms": 30000
        }
        
        result = subprocess.run(
            ["capsule-run", "--json"],
            input=json.dumps(request),
            text=True,
            capture_output=True
        )
        
        response = json.loads(result.stdout)
        return response.get("stdout", "") if response["status"] == "success" else f"Error: {response.get('error', {}).get('message', 'Unknown error')}"
```

### OpenAI Function Calling

```json
{
  "name": "execute_code",
  "description": "Execute code in a secure sandbox",
  "parameters": {
    "type": "object",
    "properties": {
      "code": {"type": "string"},
      "language": {"type": "string", "enum": ["python", "javascript", "bash"]},
      "timeout_seconds": {"type": "integer", "minimum": 1, "maximum": 300}
    },
    "required": ["code", "language"]
  }
}
```

## Performance

- **Startup Time**: <50ms cold start
- **CPU Overhead**: <2% vs native execution
- **Memory Overhead**: ~2MB base + application memory
- **I/O Performance**: Zero-copy transfers for large outputs

## Command Line Options

```
capsule-run [OPTIONS] [COMMAND]...

Options:
  --json                     Read JSON request from stdin
  -t, --timeout <MS>         Command timeout in milliseconds
  -m, --memory <SIZE>        Memory limit (e.g., 256M, 1G)
  --cpu <SHARES>             CPU shares (relative weight)
  --max-output <SIZE>        Maximum output size
  --max-pids <NUM>           Maximum number of processes
  --network                  Enable network access
  -w, --workdir <DIR>        Working directory [default: /workspace]
  -e, --env <KEY=VALUE>      Environment variable
  --readonly <PATH>          Read-only bind mount
  --writable <PATH>          Writable bind mount
  --bind <SRC:DEST[:MODE]>   Bind mount with mode (ro/rw)
  --execution-id <UUID>      Execution ID for tracking
  --pretty                   Pretty print JSON output
  -v, --verbose              Verbose output
  -h, --help                 Print help
  -V, --version              Print version
```

## Error Codes

| Code | Category | Description |
|------|----------|-------------|
| E1xxx | Configuration | Invalid request parameters |
| E2xxx | Security | Sandbox setup failures |
| E3xxx | Execution | Runtime errors and timeouts |
| E4xxx | Resource | Resource limit violations |
| E5xxx | Security | Security violations |
| E6xxx | System | System-level errors |

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Comparison with Alternatives

| Feature | capsule-run | Docker | gVisor | Firecracker |
|---------|------------|--------|--------|-------------|
| Startup time | <50ms | 1-2s | ~125ms | ~125ms |
| Binary size | ~5MB | ~50MB | ~50MB | ~5MB |
| Dependencies | None | Daemon | None | KVM |
| AI integration | Native | Adapters | Adapters | N/A |