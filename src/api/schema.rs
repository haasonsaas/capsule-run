use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionRequest {
    pub command: Vec<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub resources: ResourceLimits,
    #[serde(default)]
    pub isolation: IsolationConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceLimits {
    #[serde(default = "default_memory")]
    pub memory_bytes: u64,
    #[serde(default = "default_cpu_shares")]
    pub cpu_shares: u32,
    #[serde(default = "default_max_output")]
    pub max_output_bytes: usize,
    #[serde(default = "default_max_pids")]
    pub max_pids: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IsolationConfig {
    #[serde(default = "default_network")]
    pub network: bool,
    #[serde(default)]
    pub readonly_paths: Vec<String>,
    #[serde(default)]
    pub writable_paths: Vec<String>,
    #[serde(default = "default_working_directory")]
    pub working_directory: String,
    #[serde(default)]
    pub bind_mounts: Vec<BindMount>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BindMount {
    pub source: String,
    pub destination: String,
    pub readonly: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResponse {
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<ExecutionMetrics>,
    pub timestamps: ExecutionTimestamps,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Success,
    Error,
    Timeout,
    Killed,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionMetrics {
    pub wall_time_ms: u64,
    pub cpu_time_ms: u64,
    pub user_time_ms: u64,
    pub kernel_time_ms: u64,
    pub max_memory_bytes: u64,
    pub io_bytes_read: u64,
    pub io_bytes_written: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionTimestamps {
    pub started: DateTime<Utc>,
    pub completed: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_bytes: default_memory(),
            cpu_shares: default_cpu_shares(),
            max_output_bytes: default_max_output(),
            max_pids: default_max_pids(),
        }
    }
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            network: default_network(),
            readonly_paths: vec![],
            writable_paths: vec![],
            working_directory: default_working_directory(),
            bind_mounts: vec![],
        }
    }
}

impl ExecutionResponse {
    pub fn success(
        execution_id: Uuid,
        exit_code: i32,
        stdout: String,
        stderr: String,
        metrics: ExecutionMetrics,
        started: DateTime<Utc>,
        completed: DateTime<Utc>,
    ) -> Self {
        Self {
            execution_id,
            status: ExecutionStatus::Success,
            exit_code: Some(exit_code),
            stdout: Some(stdout),
            stderr: Some(stderr),
            metrics: Some(metrics),
            timestamps: ExecutionTimestamps { started, completed },
            error: None,
        }
    }

    pub fn error(
        execution_id: Uuid,
        error: ErrorResponse,
        started: DateTime<Utc>,
        completed: DateTime<Utc>,
    ) -> Self {
        Self {
            execution_id,
            status: ExecutionStatus::Error,
            exit_code: None,
            stdout: None,
            stderr: None,
            metrics: None,
            timestamps: ExecutionTimestamps { started, completed },
            error: Some(error),
        }
    }

    pub fn timeout(
        execution_id: Uuid,
        timeout_ms: u64,
        started: DateTime<Utc>,
        completed: DateTime<Utc>,
    ) -> Self {
        let error = ErrorResponse {
            code: "E3001".to_string(),
            message: format!("Command exceeded timeout limit of {}ms", timeout_ms),
            details: Some(serde_json::json!({
                "timeout_ms": timeout_ms,
                "elapsed_ms": (completed - started).num_milliseconds()
            })),
        };

        Self {
            execution_id,
            status: ExecutionStatus::Timeout,
            exit_code: None,
            stdout: None,
            stderr: None,
            metrics: None,
            timestamps: ExecutionTimestamps { started, completed },
            error: Some(error),
        }
    }
}

fn default_timeout() -> u64 {
    30_000 // 30 seconds
}

fn default_memory() -> u64 {
    268_435_456 // 256 MB
}

fn default_cpu_shares() -> u32 {
    1024 // Default CPU shares
}

fn default_max_output() -> usize {
    1_048_576 // 1 MB
}

fn default_max_pids() -> u32 {
    100 // Maximum number of processes
}

fn default_network() -> bool {
    false // Network disabled by default
}

fn default_working_directory() -> String {
    "/workspace".to_string()
}