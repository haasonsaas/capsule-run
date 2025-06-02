# Architecture Diagrams

This document contains detailed Mermaid diagrams illustrating the architecture and data flow of capsule-run.

## System Overview

```mermaid
graph TB
    subgraph "Entry Points"
        CLI[CLI Arguments]
        JSON[JSON stdin]
    end
    
    subgraph "Core Components"
        Parser[Argument Parser]
        Validator[Request Validator]
        Executor[Executor Engine]
        Sandbox[Sandbox Manager]
    end
    
    subgraph "Platform-Specific Isolation"
        LinuxSandbox[Linux Sandbox]
        MacOSSandbox[macOS Sandbox]
        StubSandbox[Stub Sandbox]
    end
    
    subgraph "Linux Isolation Layers"
        Namespaces[Namespaces<br/>user, mount, pid, net]
        Cgroups[Cgroups v2<br/>memory, cpu, pids]
        Seccomp[Seccomp Filter<br/>syscall allowlist]
        Filesystem[Filesystem<br/>pivot_root, bind mounts]
    end
    
    subgraph "macOS Isolation"
        SetRLimit[setrlimit<br/>process limits]
        GetRUsage[getrusage<br/>resource monitoring]
    end
    
    subgraph "Monitoring & I/O"
        IOCapture[I/O Capture<br/>stdout/stderr streaming]
        ResourceMonitor[Resource Monitor<br/>memory, cpu, i/o stats]
        TimeoutMonitor[Timeout Monitor<br/>graceful shutdown]
    end
    
    CLI --> Parser
    JSON --> Parser
    Parser --> Validator
    Validator --> Executor
    Executor --> Sandbox
    
    Sandbox --> LinuxSandbox
    Sandbox --> MacOSSandbox
    Sandbox --> StubSandbox
    
    LinuxSandbox --> Namespaces
    LinuxSandbox --> Cgroups
    LinuxSandbox --> Seccomp
    LinuxSandbox --> Filesystem
    
    MacOSSandbox --> SetRLimit
    MacOSSandbox --> GetRUsage
    
    Executor --> IOCapture
    Executor --> ResourceMonitor
    Executor --> TimeoutMonitor
    
    style CLI fill:#e1f5fe
    style JSON fill:#e1f5fe
    style LinuxSandbox fill:#e8f5e8
    style MacOSSandbox fill:#fff3e0
    style StubSandbox fill:#fce4ec
```

## Request Processing Flow

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Validator
    participant Config
    participant Executor
    participant Sandbox
    participant Process
    participant Monitor
    
    User->>CLI: Command arguments or JSON
    CLI->>CLI: Parse arguments
    
    alt JSON mode
        CLI->>CLI: Read JSON from stdin
    else CLI mode
        CLI->>CLI: Convert args to ExecutionRequest
    end
    
    CLI->>Config: Load configuration
    Config-->>CLI: Config with profiles/policies
    
    CLI->>Validator: validate_execution_request()
    Validator->>Validator: Check command safety
    Validator->>Validator: Validate resource limits
    Validator->>Validator: Sanitize paths
    Validator-->>CLI: Validated request
    
    CLI->>Executor: new(execution_id)
    Executor->>Sandbox: new(execution_id)
    
    alt Linux
        Sandbox->>Sandbox: Setup namespaces
        Sandbox->>Sandbox: Configure cgroups
        Sandbox->>Sandbox: Apply seccomp filter
        Sandbox->>Sandbox: Prepare filesystem
    else macOS
        Sandbox->>Sandbox: Configure process limits
    end
    
    Sandbox-->>Executor: Ready sandbox
    Executor-->>CLI: Ready executor
    
    CLI->>Executor: execute(request)
    
    par Execution
        Executor->>Sandbox: apply_isolation()
        Sandbox->>Process: spawn command
        Process-->>Executor: Process handle
    and Monitoring
        Executor->>Monitor: Start resource monitoring
        Executor->>Monitor: Start timeout monitoring
        Executor->>Monitor: Start I/O capture
    end
    
    loop While running
        Monitor->>Process: Check resource usage
        Monitor->>Executor: Report metrics
        Process->>Executor: Stream stdout/stderr
    end
    
    alt Normal completion
        Process-->>Executor: Exit code
    else Timeout
        Monitor->>Process: SIGTERM
        Monitor->>Process: SIGKILL (if needed)
        Monitor-->>Executor: Timeout error
    else Resource limit exceeded
        Sandbox->>Process: OOM kill / limit exceeded
        Sandbox-->>Executor: Resource violation
    end
    
    Executor->>Executor: Collect final metrics
    Executor-->>CLI: ExecutionResponse
    CLI-->>User: JSON output
```

## Platform-Specific Architecture

```mermaid
graph TB
    subgraph "Conditional Compilation"
        Target{Target OS}
    end
    
    subgraph "Linux Implementation"
        LinuxSandbox[Sandbox<br/>Linux]
        NamespaceManager[NamespaceManager<br/>unshare, clone]
        CgroupManager[CgroupManager<br/>cgroups v2 API]
        FilesystemManager[FilesystemManager<br/>mount, pivot_root]
        SeccompFilter[SeccompFilter<br/>libseccomp]
        
        LinuxSandbox --> NamespaceManager
        LinuxSandbox --> CgroupManager
        LinuxSandbox --> FilesystemManager
        LinuxSandbox --> SeccompFilter
    end
    
    subgraph "macOS Implementation"
        MacOSSandbox[Sandbox<br/>macOS]
        MacOSCore[MacOSSandbox<br/>setrlimit, getrusage]
        
        MacOSSandbox --> MacOSCore
    end
    
    subgraph "Stub Implementation"
        StubSandbox[Sandbox<br/>Other OS]
        StubComponents[Stub Components<br/>No-op implementations]
        
        StubSandbox --> StubComponents
    end
    
    subgraph "Feature Flags"
        SeccompFeature{seccomp feature}
        SeccompEnabled[Seccomp Enabled]
        SeccompDisabled[Seccomp Disabled]
    end
    
    Target -->|cfg(target_os = "linux")| LinuxSandbox
    Target -->|cfg(target_os = "macos")| MacOSSandbox
    Target -->|cfg(not(any(linux, macos)))| StubSandbox
    
    SeccompFeature -->|--features seccomp| SeccompEnabled
    SeccompFeature -->|--no-default-features| SeccompDisabled
    
    SeccompEnabled --> SeccompFilter
    SeccompDisabled -.-> SeccompFilter
    
    style LinuxSandbox fill:#e8f5e8
    style MacOSSandbox fill:#fff3e0
    style StubSandbox fill:#fce4ec
    style SeccompEnabled fill:#e8f5e8
    style SeccompDisabled fill:#ffebee
```

## Security Layers (Linux)

```mermaid
graph TD
    subgraph "Host System"
        HostUser[Host User<br/>UID 1000]
        HostFS[Host Filesystem<br/>/home, /usr, /etc]
        HostProcs[Host Processes<br/>PID namespace 0]
        HostNet[Host Network<br/>Full access]
    end
    
    subgraph "Container Process"
        ContainerRoot[Container Root<br/>UID 0 → mapped to UID 1000]
        ContainerFS[Container Filesystem<br/>pivot_root to /tmp/capsule-*]
        ContainerProcs[Container Processes<br/>PID namespace isolated]
        ContainerNet[Container Network<br/>No network access]
    end
    
    subgraph "Security Enforcement"
        UserNS[User Namespace<br/>UID/GID mapping]
        MountNS[Mount Namespace<br/>Filesystem isolation]
        PIDNS[PID Namespace<br/>Process isolation]
        NetNS[Network Namespace<br/>No network by default]
        Seccomp[Seccomp Filter<br/>~50 allowed syscalls]
        Cgroups[Cgroups v2<br/>Resource limits]
        Capabilities[Capability Drop<br/>All capabilities removed]
    end
    
    HostUser -.->|maps to| ContainerRoot
    HostFS -.->|isolated from| ContainerFS
    HostProcs -.->|isolated from| ContainerProcs
    HostNet -.->|blocked by| ContainerNet
    
    UserNS --> ContainerRoot
    MountNS --> ContainerFS
    PIDNS --> ContainerProcs
    NetNS --> ContainerNet
    
    Seccomp --> ContainerProcs
    Cgroups --> ContainerProcs
    Capabilities --> ContainerProcs
    
    style HostUser fill:#ffcdd2
    style HostFS fill:#ffcdd2
    style HostProcs fill:#ffcdd2
    style HostNet fill:#ffcdd2
    style ContainerRoot fill:#c8e6c9
    style ContainerFS fill:#c8e6c9
    style ContainerProcs fill:#c8e6c9
    style ContainerNet fill:#c8e6c9
```

## Resource Monitoring Flow

```mermaid
graph TB
    subgraph "Process Execution"
        SpawnedProcess[Spawned Process<br/>Command execution]
    end
    
    subgraph "Monitoring Threads"
        ResourceMonitor[Resource Monitor<br/>Memory, CPU tracking]
        TimeoutMonitor[Timeout Monitor<br/>Deadline enforcement]
        IOMonitor[I/O Monitor<br/>stdout/stderr capture]
    end
    
    subgraph "Data Sources"
        LinuxSources[Linux Sources]
        MacOSSources[macOS Sources]
        
        subgraph "Linux Data"
            ProcStat[/proc/PID/stat<br/>CPU times]
            ProcStatus[/proc/PID/status<br/>Memory usage]
            ProcIO[/proc/PID/io<br/>I/O statistics]
            CgroupMem[cgroup/memory.current<br/>Memory usage]
            CgroupEvents[cgroup/memory.events<br/>OOM events]
        end
        
        subgraph "macOS Data"
            GetRUsage[getrusage()<br/>CPU, memory]
            ProcInfo[proc_pidinfo()<br/>Advanced stats]
        end
    end
    
    subgraph "Metrics Collection"
        MetricsAggregator[Metrics Aggregator]
        ExecutionMetrics[Execution Metrics<br/>Peak memory, CPU time, I/O]
    end
    
    SpawnedProcess --> ResourceMonitor
    SpawnedProcess --> TimeoutMonitor
    SpawnedProcess --> IOMonitor
    
    ResourceMonitor --> LinuxSources
    ResourceMonitor --> MacOSSources
    
    LinuxSources --> ProcStat
    LinuxSources --> ProcStatus
    LinuxSources --> ProcIO
    LinuxSources --> CgroupMem
    LinuxSources --> CgroupEvents
    
    MacOSSources --> GetRUsage
    MacOSSources --> ProcInfo
    
    ProcStat --> MetricsAggregator
    ProcStatus --> MetricsAggregator
    ProcIO --> MetricsAggregator
    CgroupMem --> MetricsAggregator
    CgroupEvents --> MetricsAggregator
    GetRUsage --> MetricsAggregator
    ProcInfo --> MetricsAggregator
    
    MetricsAggregator --> ExecutionMetrics
    
    style SpawnedProcess fill:#e3f2fd
    style LinuxSources fill:#e8f5e8
    style MacOSSources fill:#fff3e0
    style ExecutionMetrics fill:#f3e5f5
```

## Error Handling Architecture

```mermaid
graph TD
    subgraph "Error Categories"
        ConfigError[Configuration Error<br/>E1xxx]
        SecurityError[Security Error<br/>E2xxx]
        ExecutionError[Execution Error<br/>E3xxx]
        ResourceError[Resource Error<br/>E4xxx]
        SystemError[System Error<br/>E6xxx]
    end
    
    subgraph "Error Sources"
        CLIParsing[CLI Parsing]
        JSONParsing[JSON Parsing]
        Validation[Request Validation]
        SandboxSetup[Sandbox Setup]
        ProcessSpawn[Process Spawn]
        ResourceLimits[Resource Limits]
        Timeouts[Timeouts]
        SystemCalls[System Calls]
    end
    
    subgraph "Error Handling"
        ErrorCode[Structured Error Code]
        ErrorMessage[Human-readable Message]
        ErrorDetails[Technical Details]
        JSONResponse[JSON Error Response]
    end
    
    CLIParsing --> ConfigError
    JSONParsing --> ConfigError
    Validation --> SecurityError
    SandboxSetup --> SecurityError
    ProcessSpawn --> ExecutionError
    ResourceLimits --> ResourceError
    Timeouts --> ExecutionError
    SystemCalls --> SystemError
    
    ConfigError --> ErrorCode
    SecurityError --> ErrorCode
    ExecutionError --> ErrorCode
    ResourceError --> ErrorCode
    SystemError --> ErrorCode
    
    ErrorCode --> ErrorMessage
    ErrorCode --> ErrorDetails
    ErrorMessage --> JSONResponse
    ErrorDetails --> JSONResponse
    
    style ConfigError fill:#fff3e0
    style SecurityError fill:#ffebee
    style ExecutionError fill:#e8f5e8
    style ResourceError fill:#f3e5f5
    style SystemError fill:#e1f5fe
```

## Testing Strategy

```mermaid
graph TB
    subgraph "Test Levels"
        UnitTests[Unit Tests<br/>Individual components]
        IntegrationTests[Integration Tests<br/>Full execution flow]
        BinaryTests[Binary Tests<br/>CLI interface]
    end
    
    subgraph "Feature Matrix Testing"
        WithSeccomp[With Seccomp<br/>cargo test]
        WithoutSeccomp[Without Seccomp<br/>cargo test --no-default-features]
    end
    
    subgraph "Platform Testing"
        LinuxTesting[Linux Testing<br/>Full feature set]
        MacOSTesting[macOS Testing<br/>Reduced feature set]
        CITesting[CI Testing<br/>GitHub Actions]
    end
    
    subgraph "Local CI Testing"
        ActQuick[act quick<br/>Fast validation]
        ActBasic[act basic<br/>Standard checks]
        ActComprehensive[act comprehensive<br/>Full validation]
        ActSecurity[act security<br/>Security validation]
    end
    
    UnitTests --> WithSeccomp
    UnitTests --> WithoutSeccomp
    IntegrationTests --> WithSeccomp
    IntegrationTests --> WithoutSeccomp
    BinaryTests --> WithSeccomp
    BinaryTests --> WithoutSeccomp
    
    WithSeccomp --> LinuxTesting
    WithoutSeccomp --> MacOSTesting
    WithoutSeccomp --> CITesting
    
    LinuxTesting --> ActQuick
    MacOSTesting --> ActQuick
    CITesting --> ActBasic
    
    ActQuick --> ActBasic
    ActBasic --> ActComprehensive
    ActComprehensive --> ActSecurity
    
    style UnitTests fill:#e8f5e8
    style IntegrationTests fill:#fff3e0
    style BinaryTests fill:#e3f2fd
    style LinuxTesting fill:#e8f5e8
    style MacOSTesting fill:#fff3e0
    style CITesting fill:#f3e5f5
```