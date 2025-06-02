# Local GitHub Actions Testing

You can test our GitHub Actions workflows locally using [nektos/act](https://github.com/nektos/act) before pushing to GitHub.

## Installation

### macOS (using Homebrew)
```bash
brew install act
```

### Linux
```bash
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash
```

### Windows (using Chocolatey)
```bash
choco install act-cli
```

## Prerequisites

- **Docker**: act uses Docker to run workflows, so Docker must be installed and running
- **Docker Images**: On first run, act will ask which image size to use:
  - `micro` (200MB) - For small projects
  - `medium` (500MB) - For bigger projects ⭐ **Recommended**
  - `large` (17GB) - For enterprise projects

## Testing Our Workflows

### 1. List Available Workflows and Jobs

```bash
# List all workflows and jobs
act -l

# List jobs for specific events
act pull_request -l
act push -l
```

### 2. Test Individual Workflows

```bash
# Test the main CI workflow
act push

# Test specific jobs from CI workflow
act push -j check
act push -j test
act push -j fmt
act push -j clippy

# Test pull request workflows
act pull_request

# Test security workflow
act -W .github/workflows/security.yml

# Test comprehensive testing
act -W .github/workflows/comprehensive-test.yml
```

### 3. Test Specific Feature Combinations

```bash
# Test without seccomp (simulates CI environment without libseccomp-dev)
act push -j check --matrix features:\"--no-default-features\"

# Test with seccomp
act push -j check --matrix features:\"\"
```

### 4. Test Cross-Platform Builds

```bash
# Test macOS build (simulates macOS runner)
act push -j test-macos

# Test Ubuntu build with different targets
act push -j build --matrix target:\"x86_64-unknown-linux-gnu\"
```

## Local Testing Commands

### Quick Validation
```bash
# Test basic functionality locally
act push -j check
act push -j test
act push -j fmt --dry-run
```

### Security Testing
```bash
# Run security checks
act -W .github/workflows/security.yml -j audit
act -W .github/workflows/security.yml -j clippy-security
```

### Comprehensive Testing
```bash
# Run feature combination tests
act -W .github/workflows/comprehensive-test.yml -j test-feature-combinations

# Run documentation tests
act -W .github/workflows/comprehensive-test.yml -j documentation-test
```

## Debugging Failed Workflows

### Verbose Output
```bash
# Run with verbose output for debugging
act push -v

# Run with very verbose output
act push -vv
```

### Interactive Mode
```bash
# Run interactively to debug step by step
act push -j test --interactive
```

### Access Container Shell
```bash
# Get shell access to debug environment issues
act push -j test --shell
```

## Configuration

### Custom .actrc File
Create `.actrc` in your project root to set defaults:

```bash
# .actrc
--container-architecture linux/amd64
--artifact-server-path /tmp/artifacts
--env-file .env.local
```

### Environment Variables
Create `.env.local` for local testing environment:

```bash
# .env.local
CARGO_TERM_COLOR=always
RUST_LOG=debug
```

## Common Issues and Solutions

### 1. Docker Permission Issues
```bash
# Add your user to docker group (Linux)
sudo usermod -aG docker $USER
newgrp docker
```

### 2. Missing System Dependencies
```bash
# The act containers might not have all dependencies
# Test without seccomp features if libseccomp is missing
act push -j test --matrix features:\"--no-default-features\"
```

### 3. Matrix Strategy Testing
```bash
# Test specific matrix combinations
act push -j test --matrix os:\"ubuntu-latest\" --matrix features:\"\"
act push -j test --matrix os:\"ubuntu-latest\" --matrix features:\"--no-default-features\"
```

## Workflow-Specific Testing

### Test CI Pipeline
```bash
# Full CI pipeline test
act push

# Individual CI jobs
act push -j check
act push -j test  
act push -j fmt
act push -j clippy
act push -j build
```

### Test Security Pipeline
```bash
# Full security pipeline
act -W .github/workflows/security.yml

# Individual security jobs
act -W .github/workflows/security.yml -j audit
act -W .github/workflows/security.yml -j supply-chain
act -W .github/workflows/security.yml -j clippy-security
```

### Test Release Pipeline (Dry Run)
```bash
# Test release workflow without actual release
act -W .github/workflows/release.yml --dry-run
```

## Performance Tips

### Reuse Containers
```bash
# Reuse containers between runs for faster testing
act push --reuse
```

### Selective Job Testing
```bash
# Only test changed workflows
act push -j test -j check  # Only run specific jobs

# Skip expensive jobs during development
act push --skip build  # Skip build job
```

## Integration with Development Workflow

### Pre-Commit Testing
```bash
#!/bin/bash
# pre-commit-test.sh
echo \"Testing GitHub Actions locally before commit...\"
act push -j check -j test -j fmt -j clippy --reuse
```

### IDE Integration
Add to your IDE's tasks/commands:
- **Test Formatting**: `act push -j fmt --dry-run`
- **Test Compilation**: `act push -j check`
- **Run Tests**: `act push -j test`

## Limitations to Be Aware Of

1. **Resource Constraints**: Local Docker containers have limited resources
2. **Missing Tools**: act images might not have all tools GitHub runners have
3. **Network Access**: Some network-dependent tests might behave differently
4. **Platform Differences**: Local testing on macOS/Windows might differ from Linux CI

## Best Practices

1. **Test Early**: Run `act` before pushing commits
2. **Test Matrix**: Validate both feature combinations locally
3. **Incremental Testing**: Test individual jobs during development
4. **Full Validation**: Run complete workflows before important releases
5. **Debug Locally**: Use `act` to debug workflow issues instead of pushing repeatedly

This allows you to catch CI issues early and iterate quickly on workflow improvements! 🚀