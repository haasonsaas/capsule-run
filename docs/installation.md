# Installation Guide

This guide covers installing capsule-run on different platforms and setting up the development environment.

## Quick Install

### From Crates.io (Recommended)
```bash
cargo install capsule-run
```

### From Source
```bash
git clone https://github.com/haasonsaas/capsule-run.git
cd capsule-run
cargo install --path .
```

### From GitHub Releases
```bash
# Download latest release
curl -L https://github.com/haasonsaas/capsule-run/releases/latest/download/capsule-run-linux-x86_64.tar.gz | tar xz
sudo mv capsule-run /usr/local/bin/
```

## Platform-Specific Setup

### Linux (Recommended)

**Prerequisites:**
- Rust 1.70+ (`rustup install stable`)
- Linux kernel 3.2+ with cgroups v2 support
- libseccomp development files

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install build-essential libseccomp-dev
cargo install capsule-run
```

**RHEL/CentOS/Fedora:**
```bash
sudo dnf install gcc libseccomp-devel
cargo install capsule-run
```

**Arch Linux:**
```bash
sudo pacman -S base-devel libseccomp
cargo install capsule-run
```

### macOS

**Prerequisites:**
- Rust 1.70+ (`rustup install stable`)
- Xcode Command Line Tools

**Install:**
```bash
xcode-select --install
cargo install capsule-run
```

**Note:** macOS provides basic sandboxing with process limits and resource monitoring. Full filesystem isolation requires additional setup.

### Windows

**Status:** ⚠️ **Limited Support**

Basic process execution works, but sandboxing features are limited.

```powershell
# Install Rust first: https://rustup.rs/
cargo install capsule-run
```

## Permissions Setup

### Linux Cgroups (Optional but Recommended)

For full resource limiting, enable cgroups v2:

```bash
# Check current cgroup version
mount | grep cgroup

# Enable cgroups v2 (if needed)
sudo grub-editenv - set systemd.unified_cgroup_hierarchy=1
sudo reboot

# Add user to required groups
sudo usermod -a -G docker $USER
newgrp docker
```

### macOS Permissions

For enhanced monitoring:
```bash
# Allow process monitoring (optional)
sudo dscl . -create /Groups/capsule-monitor
sudo dscl . -append /Groups/capsule-monitor GroupMembership $USER
```

## Verification

Test your installation:

```bash
# Basic functionality test
capsule-run --version

# Simple execution test
capsule-run --timeout 5000 -- echo "Hello, World!"

# Resource monitoring test
capsule-run --timeout 5000 --memory 64M --verbose -- python3 -c "print('Testing...')"

# Configuration test
capsule-run --create-config test-config.toml
capsule-run --config test-config.toml -- echo "Config works!"
```

Expected output for the resource test:
```json
{
  "execution_id": "...",
  "status": "success", 
  "exit_code": 0,
  "stdout": "Testing...\n",
  "stderr": "",
  "metrics": {
    "wall_time_ms": 45,
    "cpu_time_ms": 12,
    "max_memory_bytes": 8388608,
    "io_bytes_read": 0,
    "io_bytes_written": 0
  }
}
```

## Development Setup

### Building from Source

```bash
git clone https://github.com/haasonsaas/capsule-run.git
cd capsule-run

# Install dependencies
cargo check

# Run tests
cargo test

# Build release version
cargo build --release

# Install locally
cargo install --path .
```

### Development Dependencies

**Additional tools for development:**
```bash
# Code formatting
rustup component add rustfmt

# Linting
rustup component add clippy

# Documentation generation
cargo install cargo-docs

# Security auditing
cargo install cargo-audit
```

### IDE Setup

**VS Code:**
```json
// .vscode/settings.json
{
    "rust-analyzer.cargo.buildScripts.enable": true,
    "rust-analyzer.checkOnSave.command": "clippy"
}
```

**Recommended extensions:**
- rust-analyzer
- CodeLLDB (for debugging)
- Error Lens

## Troubleshooting

### Common Issues

**"Permission denied" errors:**
```bash
# Linux: Check cgroup permissions
ls -la /sys/fs/cgroup/
sudo chown -R $USER:$USER /sys/fs/cgroup/user/

# macOS: Check security settings
sudo spctl --status
```

**Build failures:**
```bash
# Update Rust
rustup update

# Clear cargo cache
cargo clean
rm -rf ~/.cargo/registry/index/*

# Retry build
cargo build
```

**Runtime errors:**
```bash
# Enable debug logging
RUST_LOG=debug capsule-run --verbose -- your-command

# Check system resources
ulimit -a
free -h  # Linux
vm_stat  # macOS
```

### Getting Help

1. **Check the logs**: Use `--verbose` flag for detailed output
2. **Review system requirements**: Ensure your platform is supported
3. **Search existing issues**: [GitHub Issues](https://github.com/haasonsaas/capsule-run/issues)
4. **Create a new issue**: Include system info and error messages

### System Information for Bug Reports

```bash
# Include this information in bug reports
capsule-run --version
rustc --version
uname -a

# Linux
lsb_release -a
cat /proc/version

# macOS  
sw_vers
system_profiler SPSoftwareDataType
```

## Next Steps

- [Quick Start Tutorial](quickstart.md) - Learn basic usage
- [Configuration Guide](configuration.md) - Set up config files
- [CLI Reference](cli.md) - Complete command-line options
- [Security Guide](security.md) - Configure security policies