# Troubleshooting Guide

Common issues, solutions, and debugging techniques for capsule-run.

## Quick Diagnosis

### Check System Status

```bash
# Verify installation
capsule-run --version

# Test basic functionality
capsule-run --timeout 5000 -- echo "test"

# Enable verbose logging
RUST_LOG=debug capsule-run --verbose --timeout 5000 -- echo "debug test"

# Validate configuration
capsule-run --config myconfig.toml --dry-run -- echo "config test"
```

### System Information

```bash
# Gather system information for bug reports
echo "=== System Information ==="
uname -a
rustc --version
capsule-run --version

echo "\n=== Platform Specific ==="
# Linux
if command -v lsb_release >/dev/null; then
    lsb_release -a
    cat /proc/version
fi

# macOS
if command -v sw_vers >/dev/null; then
    sw_vers
fi

echo "\n=== Resource Limits ==="
ulimit -a
```

## Common Issues

### Installation Problems

#### Issue: Compilation fails with missing dependencies

**Error:**
```
error: failed to compile capsule-run
caused by: could not find libseccomp
```

**Solution (Linux):**
```bash
# Ubuntu/Debian
sudo apt update
sudo apt install build-essential libseccomp-dev

# RHEL/CentOS/Fedora
sudo dnf install gcc libseccomp-devel

# Arch Linux
sudo pacman -S base-devel libseccomp

# Retry installation
cargo install capsule-run
```

**Solution (macOS):**
```bash
# Install Xcode Command Line Tools
xcode-select --install

# Update Rust
rustup update

# Retry installation
cargo install capsule-run
```

#### Issue: Permission denied during installation

**Error:**
```
Permission denied (os error 13)
```

**Solution:**
```bash
# Don't use sudo with cargo install
# Instead, ensure cargo bin directory is in PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Install without sudo
cargo install capsule-run
```

### Runtime Issues

#### Issue: "Permission denied" when running commands

**Error:**
```json
{
  "status": "error",
  "error": {
    "code": "E2001",
    "message": "Permission denied"
  }
}
```

**Diagnosis:**
```bash
# Check file permissions
ls -la $(which capsule-run)

# Check if running with appropriate permissions
id
groups
```

**Solution:**
```bash
# Linux: Add user to required groups
sudo usermod -a -G docker $USER
newgrp docker

# Check cgroup permissions
ls -la /sys/fs/cgroup/
sudo chown -R $USER:$USER /sys/fs/cgroup/user/ || true

# macOS: Check security settings
sudo spctl --status
```

#### Issue: Commands fail with "Command not found"

**Error:**
```json
{
  "status": "error",
  "error": {
    "code": "E2002",
    "message": "Command 'python3' not found"
  }
}
```

**Diagnosis:**
```bash
# Check if command exists in PATH
which python3
echo $PATH

# Check inside sandbox environment
capsule-run --verbose -- which python3
capsule-run --verbose -- echo $PATH
```

**Solution:**
```bash
# Use full path to executable
capsule-run -- /usr/bin/python3 -c "print('test')"

# Or add to readonly paths
capsule-run --readonly /usr/bin -- python3 -c "print('test')"

# Or set PATH in configuration
capsule-run --env "PATH=/usr/bin:/bin" -- python3 -c "print('test')"
```

#### Issue: Process killed unexpectedly

**Error:**
```json
{
  "status": "error",
  "error": {
    "code": "E3003",
    "message": "Process killed by signal 9",
    "details": {
      "signal": 9,
      "signal_name": "SIGKILL"
    }
  }
}
```

**Common Causes:**
1. **Out of Memory (OOM)**: Process exceeded memory limit
2. **Timeout**: Process exceeded time limit
3. **Resource limits**: Hit CPU, file descriptor, or other limits

**Diagnosis:**
```bash
# Check memory usage
capsule-run --verbose --memory 1G -- python3 -c "
import sys
print('Memory limit test')
data = bytearray(100 * 1024 * 1024)  # 100MB
print(f'Allocated {len(data)} bytes')
"

# Check system resources
free -h  # Linux
vm_stat  # macOS

# Check for OOM killer (Linux)
dmesg | grep -i "killed process"
journalctl | grep -i oom
```

**Solution:**
```bash
# Increase memory limit
capsule-run --memory 2G -- your-command

# Increase timeout
capsule-run --timeout 60000 -- your-command

# Monitor resource usage
capsule-run --verbose --memory 1G --timeout 30000 -- your-command
```

### Configuration Issues

#### Issue: Configuration file not found

**Error:**
```
Error: Configuration file 'config.toml' not found
```

**Solution:**
```bash
# Check file exists
ls -la config.toml

# Use absolute path
capsule-run --config /full/path/to/config.toml -- echo test

# Create default configuration
capsule-run --create-config config.toml

# Check search paths
echo "Config search paths:"
echo "1. --config argument"
echo "2. \$CAPSULE_CONFIG: ${CAPSULE_CONFIG:-'not set'}"
echo "3. ~/.config/capsule-run/config.toml"
echo "4. /etc/capsule-run/config.toml"
```

#### Issue: Invalid configuration syntax

**Error:**
```
Error: TOML parse error at line 15, column 1
```

**Diagnosis:**
```bash
# Validate TOML syntax
toml-validator config.toml  # If you have a TOML validator

# Check with dry-run
capsule-run --config config.toml --dry-run -- echo test

# Enable debug logging
RUST_LOG=debug capsule-run --config config.toml -- echo test
```

**Solution:**
```bash
# Common TOML syntax issues:

# 1. Missing quotes around strings with special characters
# BAD:  command = rm -rf
# GOOD: command = "rm -rf"

# 2. Invalid array syntax
# BAD:  paths = ["/usr" "/lib"]
# GOOD: paths = ["/usr", "/lib"]

# 3. Missing section headers
# BAD:  memory_bytes = 1024
# GOOD: [defaults.resources]
#       memory_bytes = 1024

# Regenerate default config if needed
mv config.toml config.toml.backup
capsule-run --create-config config.toml
```

#### Issue: Profile not found

**Error:**
```
Error: Profile 'production' not found in configuration
```

**Solution:**
```bash
# List available profiles
grep -A1 "\[profiles\." config.toml

# Or check configuration structure
capsule-run --config config.toml --dry-run -- echo test

# Add missing profile
cat >> config.toml << 'EOF'
[profiles.production]
timeout_ms = 60000

[profiles.production.resources]
memory_bytes = 1073741824
EOF
```

### Platform-Specific Issues

#### Linux Issues

**Issue: Cgroups v2 not available**

**Error:**
```
Error: Failed to setup cgroups: cgroups v2 not mounted
```

**Solution:**
```bash
# Check cgroup version
mount | grep cgroup
cat /proc/filesystems | grep cgroup

# Enable cgroups v2 (requires reboot)
sudo grub-editenv - set systemd.unified_cgroup_hierarchy=1
sudo reboot

# Alternative: Use cgroups v1 (add to config)
echo '[platform.linux]
cgroups_version = "v1"' >> config.toml
```

**Issue: Seccomp not supported**

**Error:**
```
Error: Seccomp filtering not supported on this kernel
```

**Solution:**
```bash
# Check kernel seccomp support
grep SECCOMP /boot/config-$(uname -r)
cat /proc/version

# Disable seccomp in configuration
echo '[platform.linux]
enable_seccomp = false' >> config.toml
```

#### macOS Issues

**Issue: Sandbox-exec not found**

**Error:**
```
Error: sandbox-exec command not found
```

**Solution:**
```bash
# Check macOS version (sandbox-exec available on 10.5+)
sw_vers

# Disable sandbox-exec integration
echo '[platform.macos]
enable_sandbox_exec = false' >> config.toml

# Use basic process limits only
capsule-run --memory 256M --timeout 30000 -- your-command
```

**Issue: Gatekeeper blocking execution**

**Error:**
```
"capsule-run" cannot be opened because the developer cannot be verified
```

**Solution:**
```bash
# Allow unsigned binaries (temporary)
sudo spctl --master-disable

# Or add specific exception
sudo spctl --add /path/to/capsule-run

# Re-enable gatekeeper
sudo spctl --master-enable

# Better: Build from source
git clone https://github.com/haasonsaas/capsule-run
cd capsule-run
cargo install --path .
```

### Performance Issues

#### Issue: Slow execution times

**Symptoms:**
- Commands take much longer than expected
- High CPU usage from capsule-run process
- System becomes unresponsive

**Diagnosis:**
```bash
# Profile execution
time capsule-run --verbose --timeout 30000 -- python3 -c "print('test')"

# Check system load
top
htop  # If available

# Monitor resource usage
capsule-run --verbose --timeout 30000 -- python3 -c "
import time
for i in range(5):
    print(f'Step {i}')
    time.sleep(1)
"
```

**Solutions:**
```bash
# 1. Reduce monitoring frequency
echo '[monitoring]
monitor_interval_ms = 100  # Default is 50ms' >> config.toml

# 2. Disable unnecessary features
echo '[monitoring]
enable_io_statistics = false
enable_resource_tracking = false' >> config.toml

# 3. Use streaming I/O for long processes
capsule-run --timeout 60000 -- long-running-command

# 4. Increase resource limits to avoid swapping
capsule-run --memory 2G --cpu 2048 -- memory-intensive-task
```

#### Issue: High memory usage

**Symptoms:**
- capsule-run process uses excessive memory
- System starts swapping
- Out of memory errors

**Diagnosis:**
```bash
# Monitor memory usage
ps aux | grep capsule-run
valgrind --tool=memcheck capsule-run --timeout 5000 -- echo test  # If available

# Check for memory leaks
RUST_LOG=debug capsule-run --timeout 5000 -- echo test
```

**Solutions:**
```bash
# 1. Limit concurrent executions
echo '[security]
max_concurrent_executions = 3' >> config.toml

# 2. Reduce output buffer sizes
echo '[defaults.resources]
max_output_bytes = 524288  # 512KB instead of 1MB' >> config.toml

# 3. Use streaming for large outputs
capsule-run --timeout 60000 --max-output 10M -- command-with-large-output
```

## Debugging Techniques

### Enable Detailed Logging

```bash
# Full debug logging
RUST_LOG=debug capsule-run --verbose --timeout 5000 -- your-command

# Specific module logging
RUST_LOG=capsule_run::sandbox=debug capsule-run --timeout 5000 -- your-command

# Log to file
RUST_LOG=debug capsule-run --verbose --timeout 5000 -- your-command 2> debug.log
```

### Trace System Calls (Linux)

```bash
# Trace system calls made by capsule-run
strace -f -o trace.log capsule-run --timeout 5000 -- echo test

# Analyze trace
grep -E "(execve|clone|mount|unshare)" trace.log

# Monitor specific syscalls
strace -e trace=execve,mount,unshare capsule-run --timeout 5000 -- echo test
```

### Monitor Resource Usage

```bash
# Real-time monitoring
watch -n 1 'ps aux | grep capsule-run'

# Detailed process information
top -p $(pgrep capsule-run)

# Memory maps (Linux)
cat /proc/$(pgrep capsule-run)/maps
cat /proc/$(pgrep capsule-run)/status
```

### Network Debugging

```bash
# Test network isolation
capsule-run --network -- curl -v http://httpbin.org/ip
capsule-run -- curl -v http://httpbin.org/ip  # Should fail

# Monitor network activity
sudo netstat -tulpn | grep capsule-run
sudo ss -tulpn | grep capsule-run

# Packet capture
sudo tcpdump -i any host httpbin.org
```

## Error Code Reference

### Configuration Errors (E1xxx)

| Code | Description | Solution |
|------|-------------|----------|
| E1001 | Invalid configuration file | Check TOML/JSON syntax |
| E1002 | Missing required field | Add missing configuration fields |
| E1003 | Invalid value range | Check numeric limits and ranges |
| E1004 | Profile not found | Add profile or use existing one |

### Execution Errors (E2xxx)

| Code | Description | Solution |
|------|-------------|----------|
| E2001 | Permission denied | Check file permissions and user groups |
| E2002 | Command not found | Use full path or add to readonly paths |
| E2003 | Blocked by security policy | Check blocked_commands in config |
| E2004 | Invalid command syntax | Validate command arguments |

### Timeout Errors (E3xxx)

| Code | Description | Solution |
|------|-------------|----------|
| E3001 | Execution timeout | Increase timeout or optimize command |
| E3002 | Setup timeout | Check system resources and permissions |
| E3003 | Process killed by signal | Check memory limits and system resources |

### Resource Errors (E4xxx)

| Code | Description | Solution |
|------|-------------|----------|
| E4001 | Memory limit exceeded | Increase memory limit or optimize usage |
| E4002 | OOM killed | Increase memory limit significantly |
| E4003 | Too many processes | Increase max_pids or reduce process creation |
| E4004 | Output limit exceeded | Increase max_output_bytes or reduce output |

### System Errors (E5xxx)

| Code | Description | Solution |
|------|-------------|----------|
| E5001 | Sandbox setup failed | Check system capabilities and permissions |
| E5002 | Monitoring failed | Check system resources and reduce monitoring |
| E5003 | Platform not supported | Use supported platform or basic mode |

## Performance Optimization

### Configuration Tuning

```toml
# High-performance configuration
[defaults]
timeout_ms = 30000

[defaults.resources]
memory_bytes = 1073741824   # 1GB - generous limit
cpu_shares = 2048           # High priority
max_output_bytes = 10485760 # 10MB output
max_pids = 100              # Reasonable process limit

[monitoring]
enable_resource_tracking = true
monitor_interval_ms = 100   # Less frequent monitoring
enable_io_statistics = false # Disable if not needed

[security]
max_concurrent_executions = 5 # Limit concurrency
```

### System Optimization

```bash
# Increase file descriptor limits
echo '* soft nofile 65536' | sudo tee -a /etc/security/limits.conf
echo '* hard nofile 65536' | sudo tee -a /etc/security/limits.conf

# Optimize memory settings (Linux)
echo 'vm.swappiness=10' | sudo tee -a /etc/sysctl.conf
echo 'vm.dirty_ratio=15' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p

# Enable cgroups v2 for better performance
sudo grub-editenv - set systemd.unified_cgroup_hierarchy=1
```

## Getting Help

### Before Asking for Help

1. **Check this troubleshooting guide**
2. **Enable debug logging**: `RUST_LOG=debug capsule-run --verbose`
3. **Test with minimal configuration**: Use `--create-config`
4. **Check system requirements**: Verify platform support
5. **Gather system information**: Use commands from "System Information" section

### Reporting Issues

**Include this information:**

```bash
# System information
capsule-run --version
rustc --version
uname -a

# Configuration (if using)
cat config.toml

# Full command that failed
echo "Command: capsule-run --your --flags -- your command"

# Debug output
RUST_LOG=debug capsule-run --verbose --your --flags -- your command 2>&1 | tee debug.log
```

**Where to get help:**

1. **GitHub Issues**: [https://github.com/haasonsaas/capsule-run/issues](https://github.com/haasonsaas/capsule-run/issues)
2. **GitHub Discussions**: [https://github.com/haasonsaas/capsule-run/discussions](https://github.com/haasonsaas/capsule-run/discussions)
3. **Documentation**: [https://capsule-run.dev](https://capsule-run.dev)

### Emergency Procedures

**If capsule-run is consuming excessive resources:**

```bash
# Stop all capsule-run processes
pkill -f capsule-run

# Kill hanging child processes
pkill -f "python3.*sandbox"
pkill -f "node.*sandbox"

# Check for remaining processes
ps aux | grep -E "(capsule|sandbox)"

# Clean up temporary files
find /tmp -name "capsule-*" -mtime +1 -delete
```

## See Also

- [Installation Guide](installation.md) - Setup and installation issues
- [Configuration Guide](configuration.md) - Configuration problems
- [Security Guide](security.md) - Security-related issues
- [CLI Reference](cli.md) - Command-line usage
- [Examples](examples/) - Working examples and templates