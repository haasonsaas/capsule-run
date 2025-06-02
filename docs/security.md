# Security Guide

Comprehensive security policies and best practices for running untrusted code with capsule-run.

## Security Model

Capsule-run provides defense-in-depth security through multiple isolation layers:

```
┌─────────────────────────────────────┐
│           Command Input             │
├─────────────────────────────────────┤
│      Command Validation Layer       │  ← Block dangerous commands
├─────────────────────────────────────┤
│       Resource Limit Layer         │  ← Memory, CPU, time limits
├─────────────────────────────────────┤
│      Filesystem Isolation          │  ← Read-only/writable paths
├─────────────────────────────────────┤
│       Network Isolation            │  ← Disable network access
├─────────────────────────────────────┤
│       Process Isolation            │  ← Separate process namespace
├─────────────────────────────────────┤
│       System Call Filtering        │  ← Block dangerous syscalls (Linux)
├─────────────────────────────────────┤
│        Host Operating System       │
└─────────────────────────────────────┘
```

## Threat Model

### What Capsule-run Protects Against

✅ **Malicious Code Execution**
- Resource exhaustion attacks (fork bombs, memory bombs)
- Filesystem damage (rm -rf, file corruption)
- Network-based attacks (data exfiltration, C&C communication)
- Privilege escalation attempts
- System configuration changes

✅ **Accidental Damage**
- Unintended file deletion or modification
- Infinite loops consuming resources
- Large output flooding disk space
- Processes spawning too many children

✅ **Information Disclosure**
- Reading sensitive system files
- Accessing other users' data
- Environment variable leakage
- Process information disclosure

### What Capsule-run Does NOT Protect Against

❌ **Host Kernel Vulnerabilities**
- Kernel exploits that break container isolation
- Hardware-level attacks (Spectre, Meltdown)
- Physical access to the system

❌ **Side-Channel Attacks**
- Timing attacks
- Cache-based attacks
- Electromagnetic emissions

❌ **Denial of Service on Host**
- Network flooding (if network enabled)
- Excessive legitimate resource usage within limits

## Security Configuration

### Basic Security Setup

```toml
# Secure default configuration
[defaults]
timeout_ms = 30000              # 30 second timeout

[defaults.resources]
memory_bytes = 134217728        # 128MB limit
cpu_shares = 512               # Low CPU priority
max_output_bytes = 1048576     # 1MB output limit
max_pids = 10                  # Minimal process count

[defaults.isolation]
network = false                # Disable network
readonly_paths = ["/usr", "/lib", "/bin", "/sbin"]
writable_paths = ["/tmp"]      # Only /tmp writable
working_directory = "/tmp"

[security]
blocked_commands = [
    # File system manipulation
    "rm", "rmdir", "unlink", "mv", "cp",
    "chmod", "chown", "chgrp",
    
    # System administration
    "sudo", "su", "doas", "runas",
    "systemctl", "service", "systemd",
    
    # Network tools
    "wget", "curl", "nc", "netcat", "ssh", "scp",
    
    # Package management
    "apt", "yum", "dnf", "pacman", "brew",
    "pip", "npm", "cargo", "go",
    
    # Dangerous utilities
    "dd", "fdisk", "mount", "umount",
    "iptables", "netfilter",
    "crontab", "at"
]

enforce_command_validation = true
max_concurrent_executions = 3
```

### High-Security Profile

```toml
[profiles.high-security]
timeout_ms = 10000              # Very short timeout

[profiles.high-security.resources]
memory_bytes = 67108864         # 64MB only
cpu_shares = 256               # Very low CPU
max_output_bytes = 65536       # 64KB output
max_pids = 5                   # Minimal processes

[profiles.high-security.isolation]
network = false
readonly_paths = ["/usr", "/lib", "/bin"]
writable_paths = []            # No writable paths except working dir
working_directory = "/tmp/sandbox"

# Minimal environment
[profiles.high-security.environment]
PATH = "/usr/bin:/bin"
HOME = "/tmp/sandbox"
USER = "nobody"
SHELL = "/bin/false"

[profiles.high-security.security]
blocked_commands = ["*"]       # Block everything except allowed
allowed_commands = [
    "python3", "node", "java",
    "echo", "cat", "head", "tail",
    "grep", "sed", "awk"
]
max_command_length = 1024
allow_shell_features = false
```

## Command Validation

### Command Filtering Strategies

**1. Blocklist Approach (Default)**
```toml
[security]
blocked_commands = ["rm", "sudo", "curl"]
allowed_commands = []          # Empty = allow all except blocked
```

**2. Allowlist Approach (Most Secure)**
```toml
[security]
blocked_commands = []
allowed_commands = ["python3", "node", "echo"]  # Only these allowed
```

**3. Pattern-Based Filtering**
```toml
[security.command_validation]
# Allow only specific patterns
allowed_patterns = [
    "^python3 [a-zA-Z0-9_/-]+\.py$",    # Python files only
    "^node [a-zA-Z0-9_/-]+\.js$",       # Node.js files only
    "^echo [^;|&]+$"                    # Simple echo commands
]

# Block dangerous patterns
blocked_patterns = [
    ".*rm\\s+-rf.*",                    # rm -rf commands
    ".*>\\s*/etc/.*",                   # Writing to /etc
    ".*\\|.*",                         # Any pipes
    ".*;.*",                           # Command chaining
    ".*&&.*",                          # Command chaining
    ".*\\$\\(.*\\).*"                     # Command substitution
]
```

### Common Dangerous Commands

```toml
[security]
blocked_commands = [
    # File destruction
    "rm", "rmdir", "unlink", "shred",
    
    # Permission changes
    "chmod", "chown", "chgrp", "setfacl",
    
    # System modification
    "mount", "umount", "sysctl",
    "systemctl", "service",
    
    # Network access
    "wget", "curl", "nc", "netcat",
    "ssh", "scp", "rsync", "ftp",
    
    # Privilege escalation
    "sudo", "su", "doas", "runuser",
    
    # Package installation
    "apt", "apt-get", "yum", "dnf",
    "pip", "npm", "gem", "cargo",
    
    # System information
    "ps", "top", "netstat", "lsof",
    "who", "w", "last", "id",
    
    # Dangerous utilities
    "dd", "fdisk", "cfdisk", "parted",
    "mkfs", "fsck", "badblocks",
    
    # Shell and scripting
    "bash", "sh", "zsh", "fish", "csh",
    "eval", "exec", "source",
    
    # Archive manipulation
    "tar", "gzip", "gunzip", "zip", "unzip"
]
```

## Filesystem Security

### Safe Path Configuration

```toml
[defaults.isolation]
# Allow reading system libraries and binaries
readonly_paths = [
    "/usr/bin",         # System binaries
    "/usr/lib",         # System libraries  
    "/usr/lib64",       # 64-bit libraries
    "/lib",             # Essential libraries
    "/lib64",           # 64-bit essential libraries
    "/bin",             # Basic binaries
    "/usr/share",       # Shared data (fonts, etc.)
    "/etc/ld.so.conf",  # Dynamic linker config
    "/etc/ssl/certs",   # SSL certificates
    "/usr/local/lib"    # Local libraries
]

# Restrict writing to safe locations only
writable_paths = [
    "/tmp",             # Temporary files
    "/var/tmp"          # Alternative temp location
]

# Dangerous paths to avoid
# Never add these to writable_paths:
# /etc - System configuration
# /var/log - System logs
# /home - User home directories
# /root - Root home directory
# /usr - System programs and libraries
# /boot - Boot files
# /dev - Device files
# /proc - Process information
# /sys - System information
```

### Bind Mount Security

```toml
[profiles.data-processing]
# Safe bind mount example
bind_mounts = [
    "/host/input:/data/input:ro",      # Read-only input data
    "/host/output:/data/output:rw",    # Write output data
    "/host/config:/config:ro"          # Read-only configuration
]

# Working directory inside sandbox
working_directory = "/workspace"
```

**Security Guidelines for Bind Mounts:**
1. Use `:ro` for read-only mounts whenever possible
2. Mount only specific directories, not entire filesystems
3. Avoid mounting sensitive directories like `/etc`, `/root`, `/home`
4. Use dedicated directories for data exchange
5. Set proper host filesystem permissions

## Network Security

### Network Isolation

```toml
[defaults.isolation]
network = false                 # Disable all network access (default)
```

### Selective Network Access

```toml
[profiles.web-scraper]
# When network access is absolutely required
network = true

# Additional security measures
[profiles.web-scraper.security]
# Block dangerous network tools
blocked_commands = [
    "ssh", "scp", "rsync", "nc", "netcat",
    "telnet", "ftp", "tftp"
]

# Firewall rules (Linux with iptables)
[profiles.web-scraper.platform.linux]
firewall_rules = [
    "OUTPUT -p tcp --dport 80 -j ACCEPT",   # Allow HTTP
    "OUTPUT -p tcp --dport 443 -j ACCEPT",  # Allow HTTPS
    "OUTPUT -j DROP"                        # Drop everything else
]
```

## Resource-Based Security

### Memory Limits

```toml
[security.resource_limits]
# Prevent memory bombs
max_memory_per_process = 134217728      # 128MB per process
total_memory_limit = 268435456          # 256MB total

# Early warning thresholds
memory_warning_threshold = 0.8          # Warn at 80%
memory_kill_threshold = 0.95            # Kill at 95%
```

### CPU and Process Limits

```toml
[security.resource_limits]
# Prevent fork bombs
max_processes = 10
max_threads_per_process = 5

# CPU time limits
max_cpu_time_ms = 30000                 # 30 seconds CPU time
cpu_shares = 512                        # Low priority

# Prevent infinite loops
max_wall_time_ms = 60000                # 1 minute wall time
```

### I/O Limits

```toml
[security.resource_limits]
# Prevent output flooding
max_output_bytes = 1048576              # 1MB total output
max_stdout_bytes = 524288               # 512KB stdout
max_stderr_bytes = 524288               # 512KB stderr

# File I/O limits (Linux)
max_read_bytes = 10485760               # 10MB read
max_write_bytes = 5242880               # 5MB write
max_open_files = 64                     # File descriptor limit
```

## Platform-Specific Security

### Linux Security Features

```toml
[platform.linux]
# Enable seccomp system call filtering
enable_seccomp = true
seccomp_policy = "strict"

# Namespace isolation
enable_namespaces = true
namespace_types = ["pid", "net", "ipc", "uts", "mount"]

# Disable dangerous namespaces
enable_user_namespaces = false

# Cgroups configuration
cgroups_version = "v2"
enable_memory_accounting = true
enable_cpu_accounting = true

# AppArmor/SELinux integration
enable_mandatory_access_control = true
mac_profile = "capsule-run-restricted"
```

### macOS Security Features

```toml
[platform.macos]
# Enable sandbox-exec integration
enable_sandbox_exec = true
sandbox_profile = "no-network"

# Code signing requirements
require_code_signing = true
allowed_signing_authorities = [
    "Apple",
    "Developer ID Application"
]

# Disable dangerous entitlements
blocked_entitlements = [
    "com.apple.security.get-task-allow",
    "com.apple.private.kernel.override-cpumon"
]
```

## Security Monitoring

### Real-time Monitoring

```toml
[monitoring.security]
# Enable security event logging
enable_security_logging = true
log_file = "/var/log/capsule-run-security.log"

# Monitor for suspicious activity
monitor_file_access = true
monitor_network_attempts = true
monitor_process_creation = true
monitor_resource_usage = true

# Alert thresholds
alert_on_blocked_commands = true
alert_on_resource_limits = true
alert_on_timeout = true
```

### Security Metrics

```toml
[monitoring.metrics]
# Track security-relevant metrics
track_command_blocks = true
track_resource_violations = true
track_isolation_bypasses = true
track_execution_failures = true

# Export metrics for analysis
metrics_endpoint = "http://prometheus:9090/metrics"
metrics_interval_seconds = 60
```

## Security Best Practices

### 1. Principle of Least Privilege

- **Start restrictive**: Begin with the most restrictive settings
- **Add permissions gradually**: Only grant what's absolutely necessary
- **Regular audits**: Review and tighten permissions periodically
- **Separate profiles**: Use different profiles for different trust levels

### 2. Defense in Depth

```toml
# Layer multiple security controls
[security.layered_defense]
command_validation = true       # Layer 1: Command filtering
resource_limits = true          # Layer 2: Resource control
filesystem_isolation = true     # Layer 3: Filesystem restrictions
network_isolation = true        # Layer 4: Network control
process_isolation = true        # Layer 5: Process separation
syscall_filtering = true        # Layer 6: System call control
```

### 3. Input Validation

```toml
[security.input_validation]
# Validate all inputs
max_command_length = 2048
max_environment_vars = 20
max_environment_var_length = 1024
max_argument_count = 50
max_argument_length = 256

# Character restrictions
allowed_characters = "a-zA-Z0-9_./- "
block_shell_metacharacters = true
block_null_bytes = true
```

### 4. Monitoring and Logging

```toml
[security.logging]
# Log all security-relevant events
log_executions = true
log_blocked_commands = true
log_resource_violations = true
log_isolation_events = true

# Centralized logging
syslog_facility = "local0"
syslog_priority = "info"
forward_to_siem = true
siem_endpoint = "https://siem.company.com/api/events"
```

### 5. Regular Security Reviews

**Weekly:**
- Review execution logs for anomalies
- Check resource usage patterns
- Audit blocked command attempts

**Monthly:**
- Update blocked command lists
- Review and test configuration changes
- Analyze security metrics trends

**Quarterly:**
- Full security configuration audit
- Penetration testing
- Update threat model

## Security Testing

### Test Dangerous Commands

```bash
# Test that dangerous commands are blocked
capsule-run --config secure.toml -- rm -rf /tmp/test  # Should fail
capsule-run --config secure.toml -- sudo whoami      # Should fail
capsule-run --config secure.toml -- curl google.com  # Should fail if network disabled
```

### Test Resource Limits

```bash
# Test memory limit enforcement
capsule-run --memory 64M -- python3 -c "
data = bytearray(100 * 1024 * 1024)  # Try to allocate 100MB
print('Should not reach here')
"  # Should be killed

# Test process limit enforcement  
capsule-run --max-pids 5 -- python3 -c "
import os
for i in range(20):
    if os.fork() == 0:
        exit(0)
"  # Should be limited
```

### Test Filesystem Isolation

```bash
# Test that sensitive files cannot be accessed
capsule-run --config secure.toml -- cat /etc/passwd    # Should fail
capsule-run --config secure.toml -- ls /root           # Should fail
capsule-run --config secure.toml -- touch /etc/test    # Should fail
```

## Incident Response

### Security Event Response

1. **Immediate Actions**:
   - Stop all active executions
   - Preserve logs and evidence
   - Isolate affected systems

2. **Investigation**:
   - Analyze execution logs
   - Check resource usage patterns
   - Review command history

3. **Recovery**:
   - Update security policies
   - Patch vulnerabilities
   - Test new configurations

### Emergency Procedures

```bash
# Emergency stop all executions
pkill -f capsule-run

# Check for suspicious processes
ps aux | grep -E "(rm|sudo|curl|wget|nc)"

# Review recent executions
journalctl -u capsule-run --since "1 hour ago"

# Audit filesystem changes
find /tmp -newer /tmp/checkpoint -ls
```

## Compliance and Standards

### Security Standards Alignment

- **NIST Cybersecurity Framework**: Identify, Protect, Detect, Respond, Recover
- **OWASP Top 10**: Input validation, access control, security logging
- **CIS Controls**: Secure configuration, access control, monitoring
- **ISO 27001**: Information security management

### Compliance Reporting

```toml
[compliance]
# Generate compliance reports
enable_compliance_logging = true
compliance_standards = ["nist", "owasp", "cis"]
report_format = "json"
report_destination = "/var/log/compliance/"
report_frequency = "daily"
```

## See Also

- [Configuration Guide](configuration.md) - Security configuration options
- [CLI Reference](cli.md) - Security-related command-line flags
- [Troubleshooting](troubleshooting.md) - Security-related issues
- [Examples](examples/) - Security configuration examples