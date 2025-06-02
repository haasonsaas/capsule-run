# Examples

Real-world usage examples and configuration templates for capsule-run.

## Configuration Examples

- [AI Agent Configuration](ai-agent-config.toml) - Secure config for AI code execution
- [Development Environment](development-config.toml) - Relaxed settings for development
- [Production Deployment](production-config.toml) - Battle-tested production settings
- [High Security](high-security-config.toml) - Maximum security configuration
- [Multi-Language Support](multi-language-config.toml) - Support for Python, Node.js, Java, etc.

## Integration Examples

- [Python Library Integration](python-integration.py) - Using capsule-run from Python
- [Node.js Integration](nodejs-integration.js) - Using capsule-run from Node.js
- [REST API Server](rest-api-server.py) - HTTP API wrapper around capsule-run
- [WebSocket Integration](websocket-integration.py) - Real-time code execution
- [Docker Integration](docker-integration.md) - Running capsule-run in containers

## Use Case Examples

- [AI Code Execution](ai-code-execution/) - Examples for AI agents and LLMs
- [Educational Platform](educational-platform/) - Online coding platforms
- [CI/CD Integration](cicd-integration/) - Continuous integration examples
- [Code Review Automation](code-review/) - Automated code analysis
- [Data Processing](data-processing/) - Safe data transformation pipelines

## Security Examples

- [Security Policies](security-policies/) - Various security configuration examples
- [Penetration Testing](security-testing/) - Security validation scripts
- [Incident Response](incident-response/) - Security monitoring and response

## Quick Examples

### Basic Usage

```bash
# Simple Python execution
capsule-run --timeout 5000 --memory 256M -- python3 -c "print('Hello, World!')"

# With configuration file
capsule-run --config examples/ai-agent-config.toml -- python3 script.py

# Multiple environment variables
capsule-run \
  --env "DEBUG=1" \
  --env "API_KEY=secret" \
  --timeout 10000 \
  -- node app.js
```

### JSON Input/Output

```bash
# JSON input
echo '{
  "command": ["python3", "-c", "print(42)"],
  "timeout_ms": 5000,
  "resources": {"memory_bytes": 134217728}
}' | capsule-run --json

# Pretty JSON output
capsule-run --pretty --timeout 5000 -- echo "test"
```

### Advanced Filesystem Control

```bash
# Read-only system access, writable temp
capsule-run \
  --readonly /usr \
  --readonly /lib \
  --writable /tmp \
  --workdir /tmp \
  -- python3 -c "
import os, tempfile
print('Working in:', os.getcwd())
with tempfile.NamedTemporaryFile(mode='w', delete=False) as f:
    f.write('Test data')
    print('Created file:', f.name)
"
```

### Data Processing Pipeline

```bash
# Process data with bind mounts
capsule-run \
  --bind /host/input:/data/input:ro \
  --bind /host/output:/data/output \
  --workdir /workspace \
  --timeout 60000 \
  --memory 1G \
  -- python3 -c "
import json, os
with open('/data/input/data.json') as f:
    data = json.load(f)
result = {'processed': len(data), 'status': 'complete'}
with open('/data/output/result.json', 'w') as f:
    json.dump(result, f)
print('Processing complete')
"
```

## Configuration Templates

### Minimal Secure Configuration

```toml
[defaults]
timeout_ms = 30000

[defaults.resources]
memory_bytes = 134217728
max_output_bytes = 1048576
max_pids = 10

[defaults.isolation]
network = false
working_directory = "/tmp"

[security]
blocked_commands = ["rm", "sudo", "curl", "wget"]
```

### AI Agent Configuration

```toml
[defaults]
timeout_ms = 30000

[defaults.resources]
memory_bytes = 536870912  # 512MB
cpu_shares = 1024
max_output_bytes = 2097152  # 2MB
max_pids = 50

[defaults.isolation]
network = false
readonly_paths = ["/usr", "/lib"]
writable_paths = ["/tmp"]
working_directory = "/workspace"

[security]
blocked_commands = [
    "rm", "rmdir", "sudo", "curl", "wget",
    "ssh", "chmod", "chown", "mount"
]
enforce_command_validation = true
max_concurrent_executions = 5

[monitoring]
enable_resource_tracking = true
log_executions = true
```

## Error Handling Examples

### Bash Script Integration

```bash
#!/bin/bash
# Robust error handling in shell scripts

set -euo pipefail

execute_code() {
    local code="$1"
    local timeout="${2:-30000}"
    
    # Execute with proper error handling
    local output
    local exit_code
    
    output=$(capsule-run \
        --config secure.toml \
        --timeout "$timeout" \
        --json <<< "{
            \"command\": [\"python3\", \"-c\", \"$code\"],
            \"timeout_ms\": $timeout
        }" 2>&1)
    exit_code=$?
    
    if [ $exit_code -eq 0 ]; then
        # Parse successful execution
        local status
        status=$(echo "$output" | jq -r '.status')
        
        case "$status" in
            "success")
                echo "✓ Execution successful"
                echo "$output" | jq -r '.stdout'
                ;;
            "timeout")
                echo "⚠ Execution timed out"
                ;;
            "error")
                echo "✗ Execution failed"
                echo "$output" | jq -r '.error.message'
                ;;
        esac
    else
        echo "✗ capsule-run failed with exit code $exit_code"
        echo "$output"
    fi
}

# Usage examples
execute_code "print('Hello, World!')" 5000
execute_code "import time; time.sleep(5); print('Done')" 10000
```

### Python Integration

```python
#!/usr/bin/env python3

import json
import subprocess
import sys
from typing import Dict, Any, Optional

class CapsuleRunner:
    def __init__(self, config_file: Optional[str] = None):
        self.config_file = config_file
    
    def execute(self, 
                command: list, 
                timeout_ms: int = 30000,
                memory_mb: int = 256,
                environment: Optional[Dict[str, str]] = None) -> Dict[str, Any]:
        """
        Execute code safely using capsule-run
        """
        request = {
            "command": command,
            "timeout_ms": timeout_ms,
            "resources": {
                "memory_bytes": memory_mb * 1024 * 1024,
                "max_output_bytes": 1024 * 1024  # 1MB
            },
            "environment": environment or {}
        }
        
        cmd = ["capsule-run", "--json"]
        if self.config_file:
            cmd.extend(["--config", self.config_file])
        
        try:
            result = subprocess.run(
                cmd,
                input=json.dumps(request),
                capture_output=True,
                text=True,
                timeout=timeout_ms / 1000 + 10  # Add buffer for overhead
            )
            
            if result.returncode == 0:
                return json.loads(result.stdout)
            else:
                return {
                    "status": "error",
                    "error": {
                        "code": "E9999",
                        "message": f"capsule-run failed: {result.stderr}"
                    }
                }
        except subprocess.TimeoutExpired:
            return {
                "status": "timeout",
                "error": {
                    "code": "E3001", 
                    "message": "capsule-run process timed out"
                }
            }
        except Exception as e:
            return {
                "status": "error",
                "error": {
                    "code": "E9998",
                    "message": f"Unexpected error: {str(e)}"
                }
            }

# Usage example
if __name__ == "__main__":
    runner = CapsuleRunner("examples/ai-agent-config.toml")
    
    # Execute Python code
    result = runner.execute(
        command=["python3", "-c", "print('Hello from Python!')"],
        timeout_ms=10000,
        memory_mb=128
    )
    
    if result["status"] == "success":
        print("Output:", result["stdout"])
        print("Memory used:", result["metrics"]["max_memory_bytes"], "bytes")
    else:
        print("Error:", result["error"]["message"])
```

## Performance Examples

### Batch Processing

```bash
#!/bin/bash
# Process multiple files efficiently

PROCESS_DIR="/data/to_process"
OUTPUT_DIR="/data/processed"
CONFIG="production.toml"
MAX_PARALLEL=4

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Process files in parallel
find "$PROCESS_DIR" -name "*.py" -print0 | \
    xargs -0 -n 1 -P "$MAX_PARALLEL" -I {} bash -c '
        file="{}"
        basename=$(basename "$file" .py)
        echo "Processing $basename..."
        
        capsule-run \
            --config "'$CONFIG'" \
            --execution-id "process-$basename" \
            --timeout 60000 \
            --memory 512M \
            --bind "'$PROCESS_DIR':/input:ro" \
            --bind "'$OUTPUT_DIR':/output" \
            -- python3 "/input/$basename.py"
    '

echo "Batch processing complete"
```

### Load Testing

```bash
#!/bin/bash
# Load test capsule-run performance

CONCURRENCY=10
REQUESTS=100
CONFIG="load-test.toml"

echo "Starting load test: $REQUESTS requests with $CONCURRENCY concurrent executions"

seq 1 $REQUESTS | xargs -n 1 -P $CONCURRENCY -I {} bash -c '
    start_time=$(date +%s%3N)
    
    result=$(capsule-run \
        --config "'$CONFIG'" \
        --execution-id "load-test-{}" \
        --timeout 10000 \
        -- python3 -c "import time; time.sleep(0.1); print(\"Test {}\")")
    
    end_time=$(date +%s%3N)
    duration=$((end_time - start_time))
    
    if echo "$result" | jq -e ".status == \"success\"" > /dev/null; then
        echo "Request {}: SUCCESS ($duration ms)"
    else
        echo "Request {}: FAILED ($duration ms)"
    fi
'

echo "Load test complete"
```

## Monitoring Examples

### Resource Usage Analysis

```python
#!/usr/bin/env python3

import json
import subprocess
import time
from collections import defaultdict
from typing import Dict, List

def analyze_resource_usage(executions: List[Dict]) -> Dict:
    """
    Analyze resource usage patterns from execution results
    """
    stats = {
        "total_executions": len(executions),
        "successful": 0,
        "failed": 0,
        "timeouts": 0,
        "memory_usage": [],
        "cpu_usage": [],
        "wall_time": []
    }
    
    for execution in executions:
        if execution["status"] == "success":
            stats["successful"] += 1
            metrics = execution.get("metrics", {})
            stats["memory_usage"].append(metrics.get("max_memory_bytes", 0))
            stats["cpu_usage"].append(metrics.get("cpu_time_ms", 0))
            stats["wall_time"].append(metrics.get("wall_time_ms", 0))
        elif execution["status"] == "timeout":
            stats["timeouts"] += 1
        else:
            stats["failed"] += 1
    
    # Calculate averages
    if stats["memory_usage"]:
        stats["avg_memory_mb"] = sum(stats["memory_usage"]) / len(stats["memory_usage"]) / 1024 / 1024
        stats["max_memory_mb"] = max(stats["memory_usage"]) / 1024 / 1024
    
    if stats["cpu_usage"]:
        stats["avg_cpu_ms"] = sum(stats["cpu_usage"]) / len(stats["cpu_usage"])
        stats["max_cpu_ms"] = max(stats["cpu_usage"])
    
    if stats["wall_time"]:
        stats["avg_wall_time_ms"] = sum(stats["wall_time"]) / len(stats["wall_time"])
        stats["max_wall_time_ms"] = max(stats["wall_time"])
    
    return stats

# Example usage
if __name__ == "__main__":
    # Run multiple test executions
    executions = []
    
    test_commands = [
        ["python3", "-c", "print('test 1')"],
        ["python3", "-c", "import time; time.sleep(1); print('test 2')"],
        ["python3", "-c", "data = [i for i in range(100000)]; print(len(data))"],
    ]
    
    for i, cmd in enumerate(test_commands * 10):  # Run each command 10 times
        result = subprocess.run(
            ["capsule-run", "--json", "--timeout", "10000"],
            input=json.dumps({"command": cmd, "timeout_ms": 10000}),
            capture_output=True,
            text=True
        )
        
        if result.returncode == 0:
            executions.append(json.loads(result.stdout))
    
    # Analyze results
    stats = analyze_resource_usage(executions)
    print(json.dumps(stats, indent=2))
```

## See Also

- [Configuration Guide](../configuration.md) - Detailed configuration options
- [Security Guide](../security.md) - Security best practices
- [CLI Reference](../cli.md) - Command-line interface
- [Troubleshooting](../troubleshooting.md) - Common issues and solutions