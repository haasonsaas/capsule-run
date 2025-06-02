# Command Line Interface Reference

Complete reference for all capsule-run command-line options and usage patterns.

## Synopsis

```bash
capsule-run [OPTIONS] [-- COMMAND [ARGS...]]
capsule-run [OPTIONS] --json < request.json
```

## Basic Usage

### Standard Execution
```bash
capsule-run [OPTIONS] -- COMMAND [ARGS...]
```

### JSON Input Mode
```bash
echo '{"command": ["python3", "-c", "print(42)"]}' | capsule-run --json
```

### Configuration Management
```bash
capsule-run --create-config CONFIG_FILE
capsule-run --config CONFIG_FILE [OPTIONS] -- COMMAND
```

## Global Options

### Input/Output Control

| Option | Short | Description | Example |
|--------|-------|-------------|---------|
| `--json` | | Read JSON request from stdin | `capsule-run --json < request.json` |
| `--pretty` | | Pretty-print JSON output | `capsule-run --pretty -- echo hi` |
| `--verbose` | `-v` | Enable verbose output | `capsule-run -v -- python3 script.py` |

### Configuration

| Option | Short | Description | Example |
|--------|-------|-------------|---------|
| `--config` | `-c` | Configuration file path | `--config production.toml` |
| `--profile` | `-p` | Configuration profile name | `--profile development` |
| `--create-config` | | Create default config file | `--create-config config.toml` |

### Execution Control

| Option | Short | Description | Default | Example |
|--------|-------|-------------|---------|---------|
| `--timeout` | `-t` | Timeout in milliseconds | 30000 | `--timeout 60000` |
| `--execution-id` | | Custom execution identifier | auto-generated | `--execution-id task-001` |

## Resource Limits

### Memory Management

| Option | Short | Description | Default | Example |
|--------|-------|-------------|---------|---------|
| `--memory` | `-m` | Memory limit | 256M | `--memory 1G` |
| `--max-output` | | Output size limit | 1M | `--max-output 10M` |
| `--max-pids` | | Maximum processes | 100 | `--max-pids 50` |

**Memory Size Formats:**
- Bytes: `1048576`, `1024`
- Kilobytes: `1K`, `1KB`, `1024K`
- Megabytes: `1M`, `1MB`, `512M`
- Gigabytes: `1G`, `1GB`, `2G`

```bash
# Examples
capsule-run --memory 512M -- python3 script.py
capsule-run --memory 1G --max-output 50M -- node app.js
```

### CPU Control

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `--cpu` | CPU shares (relative weight) | 1024 | `--cpu 2048` |

```bash
# Higher CPU priority (more shares)
capsule-run --cpu 2048 -- compute-heavy-task

# Lower CPU priority
capsule-run --cpu 512 -- background-task
```

## Security & Isolation

### Network Control

| Option | Description | Default | Example |
|--------|-------------|---------|---------|
| `--network` | Enable network access | disabled | `--network` |

```bash
# Network disabled (default)
capsule-run -- curl http://example.com  # Will fail

# Network enabled
capsule-run --network -- curl http://example.com  # Works
```

### Filesystem Access

| Option | Description | Example |
|--------|-------------|---------|
| `--workdir` | Working directory | `--workdir /tmp` |
| `--readonly` | Read-only path access | `--readonly /usr` |
| `--writable` | Read-write path access | `--writable /tmp` |
| `--bind` | Bind mount (src:dest[:ro]) | `--bind /host/data:/data:ro` |

**Filesystem Examples:**
```bash
# Set working directory
capsule-run --workdir /workspace -- pwd

# Allow reading system files
capsule-run --readonly /usr --readonly /lib -- python3 script.py

# Allow writing to specific directories
capsule-run --writable /tmp --writable /var/log -- script.sh

# Complex bind mounts
capsule-run \
  --bind /host/input:/input:ro \
  --bind /host/output:/output \
  --workdir /workspace \
  -- process-data.py
```

### Environment Variables

| Option | Description | Example |
|--------|-------------|---------|
| `--env` | Set environment variable | `--env "PATH=/usr/bin"` |

```bash
# Single environment variable
capsule-run --env "DEBUG=1" -- python3 app.py

# Multiple environment variables
capsule-run \
  --env "NODE_ENV=production" \
  --env "PORT=3000" \
  --env "DATABASE_URL=sqlite:///data.db" \
  -- node server.js
```

## Output Formats

### Success Response
```json
{
  "execution_id": "a1b2c3d4-...",
  "status": "success",
  "exit_code": 0,
  "stdout": "Hello, World!\n",
  "stderr": "",
  "metrics": {
    "wall_time_ms": 45,
    "cpu_time_ms": 12,
    "user_time_ms": 8,
    "kernel_time_ms": 4,
    "max_memory_bytes": 8388608,
    "io_bytes_read": 1024,
    "io_bytes_written": 512
  },
  "timestamps": {
    "started": "2024-01-15T10:30:00.000Z",
    "completed": "2024-01-15T10:30:00.045Z"
  }
}
```

### Timeout Response
```json
{
  "execution_id": "a1b2c3d4-...",
  "status": "timeout", 
  "timestamps": {
    "started": "2024-01-15T10:30:00.000Z",
    "completed": "2024-01-15T10:30:05.001Z"
  },
  "error": {
    "code": "E3001",
    "message": "Command exceeded timeout limit of 5000ms",
    "details": {
      "elapsed_ms": 5001,
      "timeout_ms": 5000
    }
  }
}
```

### Error Response
```json
{
  "execution_id": "a1b2c3d4-...",
  "status": "error",
  "timestamps": {
    "started": "2024-01-15T10:30:00.000Z", 
    "completed": "2024-01-15T10:30:00.123Z"
  },
  "error": {
    "code": "E2003",
    "message": "Command 'rm' is not allowed by security policy"
  }
}
```

## JSON Input Format

When using `--json`, provide requests via stdin:

```json
{
  "command": ["python3", "-c", "print('Hello, JSON!')"],
  "environment": {
    "DEBUG": "1",
    "PYTHONPATH": "/custom/path"
  },
  "timeout_ms": 10000,
  "resources": {
    "memory_bytes": 134217728,
    "cpu_shares": 1024,
    "max_output_bytes": 1048576,
    "max_pids": 50
  },
  "isolation": {
    "network": false,
    "readonly_paths": ["/usr", "/lib"],
    "writable_paths": ["/tmp"],
    "working_directory": "/workspace",
    "bind_mounts": [
      {
        "source": "/host/data",
        "destination": "/data",
        "readonly": true
      }
    ]
  }
}
```

**Usage:**
```bash
# From file
capsule-run --json < request.json

# From pipe
echo "$REQUEST_JSON" | capsule-run --json

# From here-doc
capsule-run --json << 'EOF'
{
  "command": ["echo", "Hello"],
  "timeout_ms": 5000
}
EOF
```

## Advanced Usage Patterns

### Configuration with CLI Overrides

Configuration files provide defaults, CLI options override:

```bash
# Use config defaults
capsule-run --config production.toml -- python3 app.py

# Override specific settings
capsule-run --config production.toml --timeout 60000 --memory 2G -- python3 app.py

# Use profile with overrides
capsule-run --config app.toml --profile development --network -- python3 app.py
```

### Execution ID Tracking

```bash
# Generate and track execution IDs
EXEC_ID=$(uuidgen)
capsule-run --execution-id "$EXEC_ID" -- python3 task.py
echo "Task $EXEC_ID completed"
```

### Batch Processing

```bash
#!/bin/bash
# Process multiple files safely
for file in *.py; do
  echo "Processing $file..."
  capsule-run \
    --timeout 30000 \
    --memory 512M \
    --execution-id "process-$(basename "$file" .py)" \
    --env "INPUT_FILE=$file" \
    -- python3 processor.py
done
```

### Error Handling in Scripts

```bash
#!/bin/bash
# Robust error handling
output=$(capsule-run --timeout 10000 -- risky-command 2>&1)
exit_code=$?

if [ $exit_code -eq 0 ]; then
  echo "Success: $output"
else
  echo "Failed with exit code $exit_code"
  echo "Output: $output"
  
  # Parse JSON error details
  error_code=$(echo "$output" | jq -r '.error.code // "unknown"')
  echo "Error code: $error_code"
fi
```

## Exit Codes

| Exit Code | Meaning | Description |
|-----------|---------|-------------|
| 0 | Success | Command completed successfully |
| 1 | General Error | Configuration, parsing, or runtime error |
| 2 | Invalid Arguments | Bad command-line arguments |
| 3 | Permission Denied | Insufficient permissions for operation |
| 4 | Timeout | Command exceeded time limit |
| 5 | Resource Limit | Memory, CPU, or other resource limit exceeded |

## Environment Variables

Capsule-run respects these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `CAPSULE_CONFIG` | Default config file path | none |
| `CAPSULE_PROFILE` | Default profile name | none |
| `RUST_LOG` | Logging level | `info` |
| `NO_COLOR` | Disable colored output | false |

```bash
# Set default config
export CAPSULE_CONFIG="$HOME/.config/capsule-run/config.toml"
export CAPSULE_PROFILE="development"

# Enable debug logging
export RUST_LOG="capsule_run=debug"

# Use environment defaults
capsule-run -- python3 script.py
```

## Shell Integration

### Bash Completion

```bash
# Add to ~/.bashrc
eval "$(capsule-run --completion bash)"
```

### Fish Completion

```fish
# Add to ~/.config/fish/config.fish
capsule-run --completion fish | source
```

### Zsh Completion

```zsh
# Add to ~/.zshrc
eval "$(capsule-run --completion zsh)"
```

### Aliases and Functions

```bash
# Useful aliases
alias sandbox='capsule-run --config ~/.config/capsule-run/default.toml'
alias safe-python='capsule-run --timeout 30000 --memory 512M -- python3'
alias safe-node='capsule-run --timeout 30000 --memory 512M --network -- node'

# Function for AI code execution
ai-exec() {
  local code="$1"
  local timeout="${2:-30000}"
  capsule-run --timeout "$timeout" --memory 256M -- python3 -c "$code"
}

# Usage
ai-exec "print('Hello from AI!')" 10000
```

## Debugging and Troubleshooting

### Verbose Output

```bash
# Enable verbose logging
capsule-run --verbose --timeout 5000 -- python3 script.py
```

Output includes:
- Execution ID
- Command being executed  
- Resource limits applied
- Security restrictions
- Timing information

### Debug Logging

```bash
# Maximum debug information
RUST_LOG=debug capsule-run --verbose -- command
```

### Dry Run Mode

```bash
# Validate configuration without execution
capsule-run --config app.toml --profile test --dry-run -- python3 script.py
```

## Performance Tips

1. **Use configuration files** for repeated executions
2. **Set appropriate timeouts** - too high wastes resources, too low kills processes
3. **Tune memory limits** based on actual usage patterns
4. **Use streaming mode** for long-running processes (>10s timeout)
5. **Minimize filesystem mounts** for better isolation performance

## See Also

- [Configuration Guide](configuration.md) - Detailed config file documentation
- [Security Guide](security.md) - Security policies and best practices
- [Examples](examples/) - Real-world usage examples
- [Troubleshooting](troubleshooting.md) - Common issues and solutions