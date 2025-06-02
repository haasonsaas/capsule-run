# Capsule-Run Documentation

**Lightweight, secure sandboxed command execution for AI agents**

Capsule-run provides a production-ready sandboxing solution that combines security, performance, and ease of use. Perfect for AI agents that need to execute untrusted code safely.

## Quick Start

```bash
# Install
cargo install capsule-run

# Basic usage
capsule-run --timeout 5000 --memory 512M -- python3 script.py

# With configuration
capsule-run --config production.toml --profile ai-agent -- node app.js
```

## Documentation Index

### 🚀 **Getting Started**
- [Installation Guide](installation.md) - Setup and requirements
- [Quick Start Tutorial](quickstart.md) - Your first sandboxed execution
- [Configuration Guide](configuration.md) - Settings and profiles

### 📖 **Usage Guides**  
- [Command Line Interface](cli.md) - Complete CLI reference
- [Configuration Files](config-files.md) - TOML/JSON configuration
- [Security Policies](security.md) - Command filtering and restrictions
- [Resource Management](resources.md) - Memory, CPU, and I/O limits

### 🔧 **Advanced Topics**
- [Platform Support](platforms.md) - Linux, macOS, and Windows
- [Monitoring & Metrics](monitoring.md) - Resource tracking and I/O stats
- [API Integration](api.md) - Using capsule-run as a library
- [Performance Tuning](performance.md) - Optimization for production

### 🛡️ **Security**
- [Security Model](security-model.md) - Threat model and protections
- [Sandbox Isolation](isolation.md) - Filesystem, network, and process isolation
- [Best Practices](best-practices.md) - Production deployment guidelines

### 🔍 **Reference**
- [Error Codes](error-codes.md) - Complete error reference
- [Examples](examples/) - Real-world usage examples
- [Troubleshooting](troubleshooting.md) - Common issues and solutions
- [FAQ](faq.md) - Frequently asked questions

### 🏗️ **Development**
- [Architecture](architecture.md) - Internal design and components
- [Contributing](../CONTRIBUTING.md) - How to contribute
- [Changelog](../CHANGELOG.md) - Version history

## Features Overview

| Feature | Linux | macOS | Windows | Description |
|---------|-------|-------|---------|-------------|
| **Process Isolation** | ✅ | ✅ | ⚠️ | Secure process sandboxing |
| **Memory Limits** | ✅ | ✅ | ❌ | Hard memory enforcement |
| **CPU Limits** | ✅ | ✅ | ❌ | Resource usage control |
| **Filesystem Isolation** | ✅ | ⚠️ | ❌ | Read-only/writable paths |
| **Network Control** | ✅ | ⚠️ | ❌ | Network access policies |
| **Real-time Monitoring** | ✅ | ✅ | ⚠️ | Resource usage tracking |
| **I/O Statistics** | ✅ | ⚠️ | ❌ | Detailed I/O metrics |
| **Streaming Output** | ✅ | ✅ | ✅ | Real-time output capture |
| **Configuration Files** | ✅ | ✅ | ✅ | TOML/JSON config support |
| **Security Policies** | ✅ | ✅ | ✅ | Command filtering |

**Legend:** ✅ Full Support, ⚠️ Partial Support, ❌ Not Supported

## Quick Examples

### Basic Sandboxed Execution
```bash
capsule-run --timeout 10000 --memory 256M -- python3 -c "print('Hello, sandbox!')"
```

### With Security Policy
```bash
capsule-run --config secure.toml -- python3 script.py
# secure.toml blocks dangerous commands like rm, sudo
```

### Long-running Process with Streaming
```bash
capsule-run --timeout 60000 -- python3 -c "
import time
for i in range(10):
    print(f'Processing item {i}')
    time.sleep(2)
"
```

### AI Agent Integration
```python
from capsule_run import Executor, ExecutionRequest
import asyncio

async def execute_ai_code(code):
    executor = Executor.new()
    request = ExecutionRequest(
        command=["python3", "-c", code],
        timeout_ms=30000,
        resources=ResourceLimits(memory_bytes=512*1024*1024)
    )
    return await executor.execute(request)
```

## Support

- **Issues**: [GitHub Issues](https://github.com/haasonsaas/capsule-run/issues)
- **Discussions**: [GitHub Discussions](https://github.com/haasonsaas/capsule-run/discussions)
- **Documentation**: [Online Docs](https://capsule-run.dev)

## License

MIT License - see [LICENSE](../LICENSE) for details.