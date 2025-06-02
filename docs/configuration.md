# Configuration Guide

Detailed guide for configuring capsule-run with TOML and JSON configuration files.

## Overview

Capsule-run supports flexible configuration through TOML and JSON files. Configuration files allow you to:

- Set default resource limits and timeouts
- Create reusable execution profiles  
- Define security policies and command restrictions
- Configure monitoring and logging preferences
- Customize platform-specific settings

## Configuration File Locations

Capsule-run searches for configuration files in this order:

1. **Command line**: `--config path/to/config.toml`
2. **Environment variable**: `$CAPSULE_CONFIG`
3. **User config directory**:
   - Linux: `~/.config/capsule-run/config.toml`
   - macOS: `~/Library/Application Support/capsule-run/config.toml`
   - Windows: `%APPDATA%\capsule-run\config.toml`
4. **System directory**: `/etc/capsule-run/config.toml`

## Creating Configuration Files

### Generate Default Configuration

```bash
# Create a default configuration file
capsule-run --create-config my-config.toml

# Create with JSON format
capsule-run --create-config my-config.json
```

### Basic Configuration Structure (TOML)

```toml
# Default settings applied to all executions
[defaults]
timeout_ms = 30000

# Default resource limits
[defaults.resources]
memory_bytes = 268435456  # 256MB
cpu_shares = 1024
max_output_bytes = 1048576  # 1MB
max_pids = 100

# Default isolation settings
[defaults.isolation]
network = false
readonly_paths = ["/usr", "/lib"]
writable_paths = ["/tmp"]
working_directory = "/workspace"

# Security policies
[security]
blocked_commands = ["rm", "sudo", "chmod", "chown"]
allowed_commands = []  # Empty = allow all except blocked
max_concurrent_executions = 10
enforce_command_validation = true

# Monitoring configuration
[monitoring]
enable_resource_tracking = true
monitor_interval_ms = 50
enable_io_statistics = true
log_executions = true
```

## Complete Configuration Reference

### Defaults Section

The `[defaults]` section sets baseline values for all executions:

```toml
[defaults]
# Execution timeout in milliseconds
timeout_ms = 30000

# Custom execution environment variables
[defaults.environment]
PATH = "/usr/bin:/bin"
LANG = "en_US.UTF-8"
TZ = "UTC"
```

### Resource Limits

```toml
[defaults.resources]
# Memory limit in bytes (supports size suffixes: K, M, G)
memory_bytes = 268435456      # 256MB

# CPU shares (relative weight, 1024 = normal priority)
cpu_shares = 1024

# Maximum output size in bytes
max_output_bytes = 1048576    # 1MB

# Maximum number of processes/threads
max_pids = 100

# Maximum execution time (alternative to timeout_ms)
max_cpu_time_ms = 10000

# I/O limits (Linux only)
max_read_bytes = 10485760     # 10MB
max_write_bytes = 10485760    # 10MB
```

**Size Suffixes:**
- `K` or `KB`: Kilobytes (1024 bytes)
- `M` or `MB`: Megabytes (1024²)
- `G` or `GB`: Gigabytes (1024³)

### Isolation Configuration

```toml
[defaults.isolation]
# Network access control
network = false

# Read-only filesystem paths
readonly_paths = [
    "/usr",
    "/lib",
    "/lib64",
    "/bin",
    "/sbin"
]

# Writable filesystem paths
writable_paths = [
    "/tmp",
    "/var/tmp"
]

# Working directory for command execution
working_directory = "/workspace"

# Bind mounts (source:destination:mode)
bind_mounts = [
    "/host/data:/data:ro",
    "/host/output:/output:rw"
]

# Additional environment variables
[defaults.environment]
HOME = "/workspace"
USER = "sandbox"
```

### Security Policies

```toml
[security]
# Commands that are explicitly blocked
blocked_commands = [
    "rm", "rmdir", "unlink",
    "sudo", "su", "doas",
    "chmod", "chown", "chgrp",
    "mount", "umount",
    "systemctl", "service",
    "iptables", "netfilter",
    "dd", "shred"
]

# Commands that are explicitly allowed (empty = allow all except blocked)
allowed_commands = []

# Maximum concurrent executions across all processes
max_concurrent_executions = 10

# Enable strict command validation
enforce_command_validation = true

# Allow shell metacharacters and pipes
allow_shell_features = false

# Maximum command line length
max_command_length = 8192
```

### Monitoring Configuration

```toml
[monitoring]
# Enable real-time resource tracking
enable_resource_tracking = true

# Resource monitoring interval in milliseconds
monitor_interval_ms = 50

# Enable detailed I/O statistics collection
enable_io_statistics = true

# Log all executions to system log
log_executions = true

# Log file path (optional)
log_file = "/var/log/capsule-run.log"

# Log level: error, warn, info, debug, trace
log_level = "info"

# Enable performance metrics collection
enable_metrics = true
```

## Execution Profiles

Profiles allow you to define named configurations for different use cases:

```toml
# AI agent profile - strict security, moderate resources
[profiles.ai-agent]
timeout_ms = 30000

[profiles.ai-agent.resources]
memory_bytes = 536870912     # 512MB
cpu_shares = 1024
max_output_bytes = 2097152   # 2MB
max_pids = 50

[profiles.ai-agent.isolation]
network = false
readonly_paths = ["/usr", "/lib"]
writable_paths = ["/tmp"]
working_directory = "/workspace"

# Development profile - relaxed security, higher resources
[profiles.development]
timeout_ms = 120000          # 2 minutes

[profiles.development.resources]
memory_bytes = 2147483648    # 2GB
cpu_shares = 2048
max_output_bytes = 10485760  # 10MB
max_pids = 200

[profiles.development.isolation]
network = true
readonly_paths = []
writable_paths = ["/tmp", "/workspace"]
working_directory = "/workspace"

# Production profile - balanced security and performance
[profiles.production]
timeout_ms = 60000

[profiles.production.resources]
memory_bytes = 1073741824    # 1GB
cpu_shares = 1024
max_output_bytes = 5242880   # 5MB
max_pids = 100

[profiles.production.isolation]
network = false
readonly_paths = ["/usr", "/lib", "/etc"]
writable_paths = ["/tmp", "/var/tmp"]
working_directory = "/app"

[profiles.production.security]
blocked_commands = ["rm", "sudo", "chmod", "wget", "curl"]
max_concurrent_executions = 5
```

## Using Profiles

```bash
# Use a specific profile
capsule-run --config app.toml --profile ai-agent -- python3 script.py

# Override profile settings
capsule-run --config app.toml --profile development --timeout 180000 -- node app.js

# Set default profile via environment
export CAPSULE_PROFILE="production"
capsule-run --config app.toml -- python3 task.py
```

## Platform-Specific Configuration

### Linux-Specific Settings

```toml
[platform.linux]
# Cgroups version preference
cgroups_version = "v2"         # "v1" or "v2"

# Cgroups mount point
cgroups_root = "/sys/fs/cgroup"

# Enable seccomp filtering
enable_seccomp = true

# Seccomp policy: "strict", "moderate", "permissive"
seccomp_policy = "moderate"

# Enable namespace isolation
enable_namespaces = true
namespace_types = ["pid", "net", "ipc", "uts", "mount"]

# Enable user namespace mapping
enable_user_namespaces = false
user_id_mapping = "1000:0:1"
group_id_mapping = "1000:0:1"
```

### macOS-Specific Settings

```toml
[platform.macos]
# Enable sandbox-exec integration
enable_sandbox_exec = true

# Sandbox profile template
sandbox_profile = "no-network"

# Enable resource monitoring via Activity Monitor integration
enable_activity_monitor = true

# Use setrlimit for resource enforcement
enable_setrlimit = true
```

### Windows-Specific Settings

```toml
[platform.windows]
# Enable job objects for process control
enable_job_objects = true

# Use Windows Sandbox (requires Windows 10 Pro+)
enable_windows_sandbox = false

# Resource monitoring method
monitoring_method = "wmi"      # "wmi" or "perfcounters"
```

## JSON Configuration Format

Alternatively, use JSON for configuration:

```json
{
  "defaults": {
    "timeout_ms": 30000,
    "resources": {
      "memory_bytes": 268435456,
      "cpu_shares": 1024,
      "max_output_bytes": 1048576,
      "max_pids": 100
    },
    "isolation": {
      "network": false,
      "readonly_paths": ["/usr", "/lib"],
      "writable_paths": ["/tmp"],
      "working_directory": "/workspace"
    }
  },
  "security": {
    "blocked_commands": ["rm", "sudo", "chmod"],
    "max_concurrent_executions": 10,
    "enforce_command_validation": true
  },
  "profiles": {
    "ai-agent": {
      "timeout_ms": 30000,
      "resources": {
        "memory_bytes": 536870912,
        "max_output_bytes": 2097152
      },
      "isolation": {
        "network": false
      }
    }
  }
}
```

## Configuration Validation

### Validate Configuration

```bash
# Test configuration syntax
capsule-run --config test.toml --dry-run -- echo "test"

# Validate specific profile
capsule-run --config test.toml --profile ai-agent --dry-run -- python3 -c "print('test')"
```

### Common Validation Errors

1. **Invalid memory size**: Use proper suffixes (M, G) or raw bytes
2. **Missing profile**: Ensure referenced profiles exist in config
3. **Invalid timeout**: Timeout must be positive integer
4. **Path access conflicts**: Ensure readonly/writable paths don't overlap
5. **Command conflicts**: Commands can't be both blocked and allowed

## Environment Variable Integration

Configuration supports environment variable substitution:

```toml
[defaults]
timeout_ms = "${CAPSULE_TIMEOUT:30000}"  # Default to 30000 if not set

[defaults.resources]
memory_bytes = "${CAPSULE_MEMORY:268435456}"

[defaults.environment]
API_KEY = "${API_KEY}"                    # Pass through from environment
DATABASE_URL = "${DATABASE_URL:sqlite:///tmp/db.sqlite}"
```

## Advanced Configuration

### Custom Command Validation

```toml
[security.command_validation]
# Custom regex patterns for command validation
allowed_patterns = [
    "^python3? .*\.py$",           # Python scripts
    "^node .*\.js$",               # Node.js scripts
    "^/usr/bin/safe-.*"            # Only safe- prefixed commands
]

# Block patterns (takes precedence over allowed)
blocked_patterns = [
    ".*rm\\s+-rf.*",               # Dangerous rm commands
    ".*>\\s*/dev/.*",              # Writing to device files
    ".*\\|\\s*sh.*"                # Pipes to shell
]
```

### Resource Monitoring Thresholds

```toml
[monitoring.thresholds]
# Warning thresholds (percentage of limit)
memory_warning = 80
cpu_warning = 90
io_warning = 75

# Action thresholds
memory_kill = 95               # Kill process at 95% memory
cpu_throttle = 95              # Throttle at 95% CPU

# Alert configuration
enable_alerts = true
alert_webhook = "https://api.example.com/alerts"
alert_email = "admin@example.com"
```

### Custom Sandbox Profiles

```toml
# Define custom sandbox profiles for different languages
[profiles.python-data-science]
timeout_ms = 300000            # 5 minutes for data processing

[profiles.python-data-science.resources]
memory_bytes = 4294967296      # 4GB for large datasets
cpu_shares = 2048              # High CPU priority

[profiles.python-data-science.isolation]
readonly_paths = ["/usr", "/lib", "/opt/conda"]
writable_paths = ["/tmp", "/workspace", "/data"]
bind_mounts = ["/host/datasets:/data:ro"]

[profiles.python-data-science.environment]
PYTHONPATH = "/opt/conda/lib/python3.9/site-packages"
JUPYTER_CONFIG_DIR = "/workspace/.jupyter"
```

## Configuration Best Practices

1. **Start with defaults**: Use `--create-config` to generate a base configuration
2. **Use profiles**: Create specific profiles for different use cases
3. **Validate regularly**: Test configurations with `--dry-run`
4. **Version control**: Keep configuration files in version control
5. **Document custom settings**: Comment your configuration files
6. **Monitor resource usage**: Adjust limits based on actual usage patterns
7. **Security first**: Start with restrictive settings and relax as needed
8. **Environment-specific configs**: Use different configs for dev/staging/production

## See Also

- [CLI Reference](cli.md) - Command-line options that override config
- [Security Guide](security.md) - Security policies and best practices
- [Examples](examples/) - Real-world configuration examples
- [Troubleshooting](troubleshooting.md) - Configuration troubleshooting