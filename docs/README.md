# Documentation

This directory contains detailed documentation and diagrams for the capsule-run project.

## Files

### [architecture-diagrams.md](architecture-diagrams.md)
Comprehensive Mermaid diagrams illustrating the system architecture:

- **System Overview**: High-level component relationships and platform abstractions
- **Request Processing Flow**: Detailed sequence diagram of execution flow
- **Platform-Specific Architecture**: Linux vs macOS vs stub implementations  
- **Security Layers**: Multi-layer isolation on Linux with namespaces, cgroups, seccomp
- **Resource Monitoring**: Data collection from proc/cgroup/rusage sources
- **Error Handling**: Structured error codes and categories
- **Testing Strategy**: Feature matrix and platform testing approach

## Additional Documentation

For installation, usage, and API reference, see the main [README.md](../README.md).

For development guidance and architecture notes, see [CLAUDE.md](../CLAUDE.md).

For local CI testing, see [LOCAL_TESTING.md](../LOCAL_TESTING.md).