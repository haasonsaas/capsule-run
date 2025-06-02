# Quick Start Tutorial

This tutorial will get you up and running with capsule-run in 5 minutes.

## Prerequisites

- capsule-run installed ([Installation Guide](installation.md))
- Basic command-line familiarity

## Your First Sandboxed Execution

### 1. Basic Command Execution

```bash
capsule-run -- echo "Hello, sandbox!"
```

Output:
```json
{
  "execution_id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "status": "success",
  "exit_code": 0,
  "stdout": "Hello, sandbox!\n",
  "stderr": "",
  "metrics": {
    "wall_time_ms": 12,
    "cpu_time_ms": 2,
    "user_time_ms": 1,
    "kernel_time_ms": 1,
    "max_memory_bytes": 2048576,
    "io_bytes_read": 0,
    "io_bytes_written": 0
  },
  "timestamps": {
    "started": "2024-01-15T10:30:00Z",
    "completed": "2024-01-15T10:30:00.012Z"
  }
}
```

### 2. Adding Resource Limits

```bash
capsule-run --timeout 5000 --memory 128M -- python3 -c "
import sys
print(f'Python version: {sys.version}')
print('Memory limit: 128MB')
"
```

### 3. Verbose Output

```bash
capsule-run --verbose --timeout 5000 -- python3 -c "print('Debug mode')"
```

You'll see additional debug information:
```
capsule-run v0.1.0
Execution ID: auto-generated
Command: ["python3", "-c", "print('Debug mode')"]
Timeout: 5000ms
Memory limit: 268435456 bytes
Network enabled: false
```

## Real-World Examples

### Example 1: Running Untrusted Python Code

```bash
# Safe execution of user-provided Python code
capsule-run \
  --timeout 10000 \
  --memory 256M \
  --max-output 1M \
  --workdir /tmp \
  -- python3 -c "
import os
import time

print('Starting computation...')
# Simulate some work
for i in range(5):
    print(f'Step {i+1}/5')
    time.sleep(0.5)

print('Current directory:', os.getcwd())
print('Done!')
"
```

### Example 2: Node.js with Environment Variables

```bash
# Execute Node.js with custom environment
capsule-run \
  --timeout 15000 \
  --env "NODE_ENV=sandbox" \
  --env "MAX_ITERATIONS=3" \
  -- node -e "
console.log('Environment:', process.env.NODE_ENV);
console.log('Max iterations:', process.env.MAX_ITERATIONS);

for (let i = 0; i < parseInt(process.env.MAX_ITERATIONS); i++) {
  console.log(\`Iteration \${i + 1}\`);
}
"
```

### Example 3: File System Access Control

```bash
# Allow read access to /usr, write access to /tmp
capsule-run \
  --readonly /usr \
  --writable /tmp \
  --workdir /tmp \
  -- python3 -c "
import os

# This works - reading from allowed path
with open('/usr/bin/python3', 'rb') as f:
    print(f'Python binary size: {len(f.read())} bytes')

# This works - writing to allowed path  
with open('/tmp/test.txt', 'w') as f:
    f.write('Hello from sandbox!')

print('File operations completed')
"
```

## Understanding the Output

Capsule-run returns detailed JSON output with execution results:

```json
{
  "execution_id": "unique-identifier",
  "status": "success|timeout|error", 
  "exit_code": 0,
  "stdout": "captured output",
  "stderr": "captured errors",
  "metrics": {
    "wall_time_ms": 1234,      // Real time elapsed
    "cpu_time_ms": 567,        // CPU time used
    "user_time_ms": 400,       // User-space CPU time
    "kernel_time_ms": 167,     // Kernel-space CPU time  
    "max_memory_bytes": 8388608, // Peak memory usage
    "io_bytes_read": 1024,     // Data read from disk
    "io_bytes_written": 512    // Data written to disk
  },
  "timestamps": {
    "started": "2024-01-15T10:30:00Z",
    "completed": "2024-01-15T10:30:01.234Z"
  }
}
```

### Status Types

- **`success`**: Command completed normally
- **`timeout`**: Command exceeded time limit
- **`error`**: Command failed or was terminated

### Error Handling

```bash
# This will timeout after 1 second
capsule-run --timeout 1000 -- sleep 5
```

Output:
```json
{
  "status": "timeout",
  "error": {
    "code": "E3001", 
    "message": "Command exceeded timeout limit of 1000ms",
    "details": {
      "elapsed_ms": 1008,
      "timeout_ms": 1000
    }
  }
}
```

## Configuration Files

### Creating Your First Config

```bash
# Generate a default configuration file
capsule-run --create-config my-config.toml
```

This creates `my-config.toml`:
```toml
[defaults]
timeout_ms = 30000

[defaults.resources]
memory_bytes = 268435456
cpu_shares = 1024
max_output_bytes = 1048576
max_pids = 100

[defaults.isolation]
network = false
readonly_paths = []
writable_paths = []
working_directory = "/workspace"

[security]
blocked_commands = ["rm", "sudo", "chmod"]
max_concurrent_executions = 10
```

### Using Configuration Files

```bash
# Use your custom configuration
capsule-run --config my-config.toml -- python3 script.py

# Override specific settings
capsule-run --config my-config.toml --timeout 60000 -- long-running-task
```

## Security Features

### Command Filtering

The default configuration blocks dangerous commands:

```bash
# This will be blocked by security policy
capsule-run --config my-config.toml -- rm -rf /

# Output: Error: Security violation: Command 'rm' is not allowed by security policy
```

### Network Isolation

```bash
# Network access disabled by default
capsule-run -- curl http://example.com
# This will fail unless --network flag is used

# Enable network access
capsule-run --network -- curl -s http://httpbin.org/ip
```

## Advanced Features

### Streaming Output for Long Processes

For commands with timeout > 10 seconds, capsule-run automatically uses streaming I/O:

```bash
# This will stream output in real-time
capsule-run --timeout 15000 -- python3 -c "
import time
for i in range(10):
    print(f'Processing {i}/10', flush=True)
    time.sleep(1)
"
```

### Pretty JSON Output

```bash
# Human-readable JSON formatting
capsule-run --pretty -- echo "Hello"
```

### Custom Execution ID

```bash
# Track executions with custom IDs
capsule-run --execution-id "my-task-001" -- python3 script.py
```

## Next Steps

### Learn More
- [CLI Reference](cli.md) - Complete command-line options
- [Configuration Guide](configuration.md) - Advanced configuration
- [Security Guide](security.md) - Security policies and best practices

### Real-World Integration
- [API Integration](api.md) - Using capsule-run as a Rust library
- [Examples](examples/) - Production use cases
- [Performance Tuning](performance.md) - Optimization tips

### Troubleshooting
- [Common Issues](troubleshooting.md) - Solutions to common problems
- [FAQ](faq.md) - Frequently asked questions

## Example Scripts

Save these examples to try different scenarios:

**`test-memory.py`**:
```python
# Test memory limit enforcement
import sys
data = bytearray(100 * 1024 * 1024)  # 100MB
print(f"Allocated {len(data)} bytes")
```

**`test-timeout.py`**:
```python
# Test timeout handling  
import time
print("Starting long task...")
time.sleep(30)  # Will be killed by timeout
print("This won't print")
```

**`test-io.py`**:
```python
# Test I/O monitoring
import os
print("Current directory:", os.getcwd())
with open("/tmp/test.txt", "w") as f:
    f.write("Hello" * 1000)
print("File written successfully")
```

Run them with:
```bash
capsule-run --timeout 5000 --memory 50M -- python3 test-memory.py
capsule-run --timeout 5000 -- python3 test-timeout.py  
capsule-run --writable /tmp -- python3 test-io.py
```