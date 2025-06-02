mod api;
mod error;
mod executor;
mod sandbox;

use crate::api::{
    validate_execution_request, BindMount, ExecutionRequest, IsolationConfig, ResourceLimits,
};
use crate::error::CapsuleResult;
use crate::executor::Executor;
use clap::{ArgAction, Parser};
use std::collections::HashMap;
use std::io::{self, Read};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "capsule-run")]
#[command(about = "Lightweight, secure sandboxed command execution for AI agents")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    /// Read JSON request from stdin instead of using CLI arguments
    #[arg(long, action = ArgAction::SetTrue)]
    json: bool,

    /// Command timeout in milliseconds
    #[arg(long, short = 't', value_name = "MS")]
    timeout: Option<u64>,

    /// Memory limit (e.g., 256M, 1G)
    #[arg(long, short = 'm', value_name = "SIZE")]
    memory: Option<String>,

    /// CPU shares (relative weight)
    #[arg(long, value_name = "SHARES")]
    cpu: Option<u32>,

    /// Maximum output size (e.g., 1M, 10K)
    #[arg(long, value_name = "SIZE")]
    max_output: Option<String>,

    /// Maximum number of processes
    #[arg(long, value_name = "NUM")]
    max_pids: Option<u32>,

    /// Enable network access (disabled by default for security)
    #[arg(long, action = ArgAction::SetTrue)]
    network: bool,

    /// Working directory inside the sandbox
    #[arg(long, short = 'w', value_name = "DIR", default_value = "/workspace")]
    workdir: String,

    /// Environment variable (can be used multiple times)
    #[arg(long, short = 'e', value_name = "KEY=VALUE", action = ArgAction::Append)]
    env: Vec<String>,

    /// Read-only bind mount (can be used multiple times)
    #[arg(long, value_name = "PATH", action = ArgAction::Append)]
    readonly: Vec<String>,

    /// Writable bind mount (can be used multiple times)
    #[arg(long, value_name = "PATH", action = ArgAction::Append)]
    writable: Vec<String>,

    /// Bind mount source:dest[:ro|rw] (can be used multiple times)
    #[arg(long, value_name = "SRC:DEST[:MODE]", action = ArgAction::Append)]
    bind: Vec<String>,

    /// Execution ID for tracking (auto-generated if not provided)
    #[arg(long, value_name = "UUID")]
    execution_id: Option<String>,

    /// Pretty print JSON output
    #[arg(long, action = ArgAction::SetTrue)]
    pretty: bool,

    /// Verbose output (show debugging information)
    #[arg(long, short = 'v', action = ArgAction::SetTrue)]
    verbose: bool,

    /// Command and arguments to execute
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Vec<String>,
}

#[tokio::main]
async fn main() {
    let result = run().await;

    match result {
        Ok(exit_code) => {
            std::process::exit(exit_code);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run() -> CapsuleResult<i32> {
    let cli = Cli::parse();

    if cli.verbose {
        eprintln!("capsule-run v{}", env!("CARGO_PKG_VERSION"));
        eprintln!(
            "Execution ID: {}",
            cli.execution_id.as_deref().unwrap_or("auto-generated")
        );
    }

    // Parse execution ID or generate one
    let execution_id = if let Some(id_str) = &cli.execution_id {
        Uuid::parse_str(id_str).map_err(|e| {
            crate::error::CapsuleError::Config(format!("Invalid execution ID: {}", e))
        })?
    } else {
        Uuid::new_v4()
    };

    if cli.verbose {
        eprintln!("Using execution ID: {}", execution_id);
    }

    // Create execution request
    let request = if cli.json {
        read_json_request()?
    } else {
        create_request_from_cli(&cli)?
    };

    if cli.verbose {
        eprintln!("Command: {:?}", request.command);
        eprintln!("Timeout: {}ms", request.timeout_ms);
        eprintln!("Memory limit: {} bytes", request.resources.memory_bytes);
        eprintln!("Network enabled: {}", request.isolation.network);
    }

    // Validate request
    validate_execution_request(&request)?;

    // Create executor and run
    let executor = Executor::new(execution_id)?;
    let response = executor.execute(request).await?;

    // Output response
    let json_output = if cli.pretty {
        serde_json::to_string_pretty(&response)?
    } else {
        serde_json::to_string(&response)?
    };

    println!("{}", json_output);

    // Return appropriate exit code
    match response.status {
        crate::api::ExecutionStatus::Success => Ok(response.exit_code.unwrap_or(0)),
        crate::api::ExecutionStatus::Error => Ok(1),
        crate::api::ExecutionStatus::Timeout => Ok(124), // Standard timeout exit code
        crate::api::ExecutionStatus::Killed => Ok(128 + 9), // SIGKILL
    }
}

fn read_json_request() -> CapsuleResult<ExecutionRequest> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let request: ExecutionRequest = serde_json::from_str(&buffer)?;
    Ok(request)
}

fn create_request_from_cli(cli: &Cli) -> CapsuleResult<ExecutionRequest> {
    if cli.command.is_empty() {
        return Err(crate::error::CapsuleError::Config(
            "No command specified. Use --json for JSON input or provide command arguments."
                .to_string(),
        ));
    }

    // Parse environment variables
    let mut environment = HashMap::new();
    for env_var in &cli.env {
        if let Some((key, value)) = env_var.split_once('=') {
            environment.insert(key.to_string(), value.to_string());
        } else {
            return Err(crate::error::CapsuleError::Config(format!(
                "Invalid environment variable format: {}. Use KEY=VALUE.",
                env_var
            )));
        }
    }

    // Parse bind mounts
    let mut bind_mounts = Vec::new();
    for bind_spec in &cli.bind {
        let bind_mount = parse_bind_mount(bind_spec)?;
        bind_mounts.push(bind_mount);
    }

    // Create resource limits
    let resources = ResourceLimits {
        memory_bytes: cli
            .memory
            .as_ref()
            .map(|s| parse_size(s))
            .transpose()?
            .unwrap_or(268_435_456), // 256MB default
        cpu_shares: cli.cpu.unwrap_or(1024),
        max_output_bytes: cli
            .max_output
            .as_ref()
            .map(|s| parse_size(s))
            .transpose()?
            .map(|s| s as usize)
            .unwrap_or(1_048_576), // 1MB default
        max_pids: cli.max_pids.unwrap_or(100),
    };

    // Create isolation config
    let isolation = IsolationConfig {
        network: cli.network,
        readonly_paths: cli.readonly.clone(),
        writable_paths: cli.writable.clone(),
        working_directory: cli.workdir.clone(),
        bind_mounts,
    };

    Ok(ExecutionRequest {
        command: cli.command.clone(),
        environment,
        timeout_ms: cli.timeout.unwrap_or(30_000), // 30 seconds default
        resources,
        isolation,
    })
}

fn parse_bind_mount(spec: &str) -> CapsuleResult<BindMount> {
    let parts: Vec<&str> = spec.split(':').collect();

    match parts.len() {
        2 => {
            Ok(BindMount {
                source: parts[0].to_string(),
                destination: parts[1].to_string(),
                readonly: true, // Default to readonly for security
            })
        }
        3 => {
            let readonly = match parts[2] {
                "ro" => true,
                "rw" => false,
                _ => {
                    return Err(crate::error::CapsuleError::Config(format!(
                        "Invalid bind mount mode '{}'. Use 'ro' or 'rw'.",
                        parts[2]
                    )))
                }
            };

            Ok(BindMount {
                source: parts[0].to_string(),
                destination: parts[1].to_string(),
                readonly,
            })
        }
        _ => Err(crate::error::CapsuleError::Config(format!(
            "Invalid bind mount format '{}'. Use 'source:dest' or 'source:dest:mode'.",
            spec
        ))),
    }
}

fn parse_size(size_str: &str) -> CapsuleResult<u64> {
    let size_str = size_str.trim().to_uppercase();

    if let Some(number_part) = size_str.strip_suffix('K') {
        let number: u64 = number_part.parse().map_err(|_| {
            crate::error::CapsuleError::Config(format!("Invalid size format: {}", size_str))
        })?;
        Ok(number * 1024)
    } else if let Some(number_part) = size_str.strip_suffix('M') {
        let number: u64 = number_part.parse().map_err(|_| {
            crate::error::CapsuleError::Config(format!("Invalid size format: {}", size_str))
        })?;
        Ok(number * 1024 * 1024)
    } else if let Some(number_part) = size_str.strip_suffix('G') {
        let number: u64 = number_part.parse().map_err(|_| {
            crate::error::CapsuleError::Config(format!("Invalid size format: {}", size_str))
        })?;
        Ok(number * 1024 * 1024 * 1024)
    } else {
        // Assume bytes
        size_str.parse().map_err(|_| {
            crate::error::CapsuleError::Config(format!("Invalid size format: {}", size_str))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024").unwrap(), 1024);
        assert_eq!(parse_size("1K").unwrap(), 1024);
        assert_eq!(parse_size("1M").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1G").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("256m").unwrap(), 256 * 1024 * 1024);
    }

    #[test]
    fn test_parse_bind_mount() {
        let bind = parse_bind_mount("/host/path:/container/path").unwrap();
        assert_eq!(bind.source, "/host/path");
        assert_eq!(bind.destination, "/container/path");
        assert!(bind.readonly);

        let bind = parse_bind_mount("/host/path:/container/path:rw").unwrap();
        assert_eq!(bind.source, "/host/path");
        assert_eq!(bind.destination, "/container/path");
        assert!(!bind.readonly);

        let bind = parse_bind_mount("/host/path:/container/path:ro").unwrap();
        assert!(bind.readonly);

        assert!(parse_bind_mount("/invalid").is_err());
        assert!(parse_bind_mount("/a:/b:/c:/d").is_err());
        assert!(parse_bind_mount("/a:/b:invalid").is_err());
    }

    #[test]
    fn test_cli_parsing() {
        use clap::Parser;

        let cli = Cli::try_parse_from(&[
            "capsule-run",
            "--timeout",
            "5000",
            "--memory",
            "512M",
            "--env",
            "PATH=/usr/bin",
            "--env",
            "HOME=/tmp",
            "--readonly",
            "/usr",
            "--",
            "echo",
            "hello",
        ])
        .unwrap();

        assert_eq!(cli.timeout, Some(5000));
        assert_eq!(cli.memory, Some("512M".to_string()));
        assert_eq!(cli.env, vec!["PATH=/usr/bin", "HOME=/tmp"]);
        assert_eq!(cli.readonly, vec!["/usr"]);
        assert_eq!(cli.command, vec!["echo", "hello"]);
    }
}
