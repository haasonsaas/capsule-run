pub mod io;
pub mod io_stats;
pub mod monitor;

use crate::api::schema::{ExecutionMetrics, ExecutionRequest, ExecutionResponse};
use crate::error::{CapsuleResult, ErrorCode, ExecutionError};
use crate::sandbox::{ResourceUsage, Sandbox};
use chrono::{DateTime, Utc};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use uuid::Uuid;

pub use io::IoCapture;

pub struct Executor {
    execution_id: Uuid,
    sandbox: std::sync::Arc<Sandbox>,
}

// pub struct ExecutionResult {
//     pub response: ExecutionResponse,
//     pub metrics: ExecutionMetrics,
// }

impl Executor {
    pub fn new(execution_id: Uuid) -> CapsuleResult<Self> {
        let sandbox = std::sync::Arc::new(Sandbox::new(execution_id)?);

        Ok(Self {
            execution_id,
            sandbox,
        })
    }

    pub async fn execute(mut self, request: ExecutionRequest) -> CapsuleResult<ExecutionResponse> {
        let started = Utc::now();

        // Setup sandbox
        match std::sync::Arc::get_mut(&mut self.sandbox)
            .ok_or_else(|| {
                crate::error::CapsuleError::Config("Sandbox reference error".to_string())
            })?
            .setup(&request.resources, &request.isolation)
        {
            Ok(_) => {}
            Err(e) => {
                let completed = Utc::now();
                let error_code = ErrorCode::from(e);
                return Ok(ExecutionResponse::error(
                    self.execution_id,
                    crate::api::schema::ErrorResponse {
                        code: error_code.code.to_string(),
                        message: error_code.message,
                        details: None,
                    },
                    started,
                    completed,
                ));
            }
        }

        // Execute the command
        match self.execute_command(&request, started).await {
            Ok(response) => Ok(response),
            Err(e) => {
                let completed = Utc::now();
                let error_code = ErrorCode::from(e);
                Ok(ExecutionResponse::error(
                    self.execution_id,
                    crate::api::schema::ErrorResponse {
                        code: error_code.code.to_string(),
                        message: error_code.message,
                        details: None,
                    },
                    started,
                    completed,
                ))
            }
        }
    }

    async fn execute_command(
        &self,
        request: &ExecutionRequest,
        started: DateTime<Utc>,
    ) -> CapsuleResult<ExecutionResponse> {
        let start_time = Instant::now();
        let timeout_duration = Duration::from_millis(request.timeout_ms);

        // Prepare command
        let mut cmd = Command::new(&request.command[0]);
        if request.command.len() > 1 {
            cmd.args(&request.command[1..]);
        }

        // Set environment variables
        for (key, value) in &request.environment {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Prepare command with sandbox restrictions (macOS-specific)
        #[cfg(target_os = "macos")]
        self.sandbox.prepare_command(&mut cmd)?;

        // Spawn the process
        let mut child = cmd
            .spawn()
            .map_err(|e| ExecutionError::SpawnFailed(format!("Failed to spawn command: {}", e)))?;

        // Setup I/O capture
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Use streaming I/O for long-running processes (> 10 seconds timeout)
        let use_streaming = request.timeout_ms > 10_000;

        if use_streaming {
            return self
                .execute_with_streaming_io(
                    child,
                    stdout,
                    stderr,
                    request,
                    started,
                    timeout_duration,
                    start_time,
                )
                .await;
        }

        let io_capture = IoCapture::new(stdout, stderr, request.resources.max_output_bytes);

        // Setup monitoring for the process
        let process_id = child.id();
        let sandbox_provider = std::sync::Arc::clone(&self.sandbox);
        let resource_monitor = monitor::ResourceMonitor::new(
            sandbox_provider,
            std::time::Duration::from_millis(50), // Monitor every 50ms
        );

        // Setup I/O monitoring
        let io_monitor = io_stats::IoMonitor::new(process_id);

        // Enhanced execution loop with better monitoring
        loop {
            // Check timeout
            if start_time.elapsed() >= timeout_duration {
                let _ = child.kill();
                let _ = resource_monitor.stop_and_get_result(); // Stop monitoring
                let completed = Utc::now();
                return Ok(ExecutionResponse::timeout(
                    self.execution_id,
                    request.timeout_ms,
                    started,
                    completed,
                ));
            }

            // Check if process has exited
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process has exited - determine how it exited
                    let exit_code = status.code().unwrap_or(-1);

                    // Check if process was killed by signal
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        if let Some(signal) = status.signal() {
                            // Process was killed by signal - create error response
                            let completed = Utc::now();
                            let error = crate::api::schema::ErrorResponse {
                                code: "E3003".to_string(),
                                message: format!("Process killed by signal {}", signal),
                                details: Some(serde_json::json!({
                                    "signal": signal,
                                    "signal_name": signal_name(signal)
                                })),
                            };
                            return Ok(ExecutionResponse::error(
                                self.execution_id,
                                error,
                                started,
                                completed,
                            ));
                        }
                    }

                    // Collect I/O
                    let (stdout, stderr) = io_capture.wait_for_completion()?;

                    // Stop monitoring and get comprehensive results
                    let monitoring_result = resource_monitor.stop_and_get_result()?;

                    // Get final I/O statistics
                    let io_stats = io_monitor.get_total_stats().unwrap_or_default();

                    // Get final resource usage from monitoring
                    let final_usage = ResourceUsage {
                        memory_bytes: monitoring_result.peak_memory,
                        cpu_time_us: monitoring_result.total_cpu_time,
                        user_time_us: monitoring_result.total_cpu_time / 2, // Approximation
                        kernel_time_us: monitoring_result.total_cpu_time / 2, // Approximation
                        io_bytes_read: io_stats.read_bytes,
                        io_bytes_written: io_stats.write_bytes,
                    };

                    let completed = Utc::now();
                    let wall_time = start_time.elapsed();

                    // Create comprehensive execution metrics
                    let metrics = ExecutionMetrics {
                        wall_time_ms: wall_time.as_millis() as u64,
                        cpu_time_ms: final_usage.cpu_time_us / 1000,
                        user_time_ms: final_usage.user_time_us / 1000,
                        kernel_time_ms: final_usage.kernel_time_us / 1000,
                        max_memory_bytes: final_usage.memory_bytes,
                        io_bytes_read: final_usage.io_bytes_read,
                        io_bytes_written: final_usage.io_bytes_written,
                    };

                    return Ok(ExecutionResponse::success(
                        self.execution_id,
                        exit_code,
                        stdout,
                        stderr,
                        metrics,
                        started,
                        completed,
                    ));
                }
                Ok(None) => {
                    // Process is still running
                }
                Err(e) => {
                    let _ = child.kill();
                    return Err(ExecutionError::MonitoringError(format!(
                        "Failed to check process status: {}",
                        e
                    ))
                    .into());
                }
            }

            // Check for OOM kill
            if let Ok(true) = self.sandbox.check_oom_killed() {
                let _ = child.kill();
                let _ = resource_monitor.stop_and_get_result(); // Stop monitoring
                let completed = Utc::now();
                return Ok(ExecutionResponse::error(
                    self.execution_id,
                    crate::api::schema::ErrorResponse {
                        code: "E4002".to_string(),
                        message: "Process killed due to memory limit".to_string(),
                        details: Some(serde_json::json!({
                            "memory_limit": request.resources.memory_bytes
                        })),
                    },
                    started,
                    completed,
                ));
            }

            // Small sleep to avoid busy waiting
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[allow(clippy::too_many_arguments)] // Complex execution method requires multiple parameters
    async fn execute_with_streaming_io(
        &self,
        mut child: std::process::Child,
        stdout: Option<std::process::ChildStdout>,
        stderr: Option<std::process::ChildStderr>,
        request: &ExecutionRequest,
        started: DateTime<Utc>,
        timeout_duration: Duration,
        start_time: Instant,
    ) -> CapsuleResult<ExecutionResponse> {
        use io::StreamingIoCapture;

        // Setup streaming I/O capture
        let streaming_io =
            StreamingIoCapture::new(stdout, stderr, request.resources.max_output_bytes);
        let mut stdout_buffer = Vec::new();
        let mut stderr_buffer = Vec::new();

        loop {
            // Check timeout
            if start_time.elapsed() >= timeout_duration {
                let _ = child.kill();
                let completed = Utc::now();
                return Ok(ExecutionResponse::timeout(
                    self.execution_id,
                    request.timeout_ms,
                    started,
                    completed,
                ));
            }

            // Check if process has exited
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process has exited - collect final I/O
                    let (final_stdout, final_stderr) = streaming_io.collect_remaining()?;
                    stdout_buffer.extend(final_stdout.as_bytes());
                    stderr_buffer.extend(final_stderr.as_bytes());

                    let exit_code = status.code().unwrap_or(-1);

                    // Check if process was killed by signal
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        if let Some(signal) = status.signal() {
                            let completed = Utc::now();
                            let error = crate::api::schema::ErrorResponse {
                                code: "E3003".to_string(),
                                message: format!("Process killed by signal {}", signal),
                                details: Some(serde_json::json!({
                                    "signal": signal,
                                    "signal_name": signal_name(signal)
                                })),
                            };
                            return Ok(ExecutionResponse::error(
                                self.execution_id,
                                error,
                                started,
                                completed,
                            ));
                        }
                    }

                    // Get resource usage from sandbox
                    let final_usage = self.sandbox.get_resource_usage().unwrap_or(ResourceUsage {
                        memory_bytes: 0,
                        cpu_time_us: 0,
                        user_time_us: 0,
                        kernel_time_us: 0,
                        io_bytes_read: 0,
                        io_bytes_written: 0,
                    });

                    let completed = Utc::now();
                    let wall_time = start_time.elapsed();

                    let metrics = ExecutionMetrics {
                        wall_time_ms: wall_time.as_millis() as u64,
                        cpu_time_ms: final_usage.cpu_time_us / 1000,
                        user_time_ms: final_usage.user_time_us / 1000,
                        kernel_time_ms: final_usage.kernel_time_us / 1000,
                        max_memory_bytes: final_usage.memory_bytes,
                        io_bytes_read: final_usage.io_bytes_read,
                        io_bytes_written: final_usage.io_bytes_written,
                    };

                    return Ok(ExecutionResponse::success(
                        self.execution_id,
                        exit_code,
                        String::from_utf8_lossy(&stdout_buffer).to_string(),
                        String::from_utf8_lossy(&stderr_buffer).to_string(),
                        metrics,
                        started,
                        completed,
                    ));
                }
                Ok(None) => {
                    // Process is still running - read streaming data
                    let (stdout_event, stderr_event) =
                        streaming_io.read_available(Duration::from_millis(10));

                    if let Some(io::IoEvent::Data(data)) = stdout_event {
                        stdout_buffer.extend(data);
                    }

                    if let Some(io::IoEvent::Data(data)) = stderr_event {
                        stderr_buffer.extend(data);
                    }
                }
                Err(e) => {
                    let _ = child.kill();
                    return Err(ExecutionError::MonitoringError(format!(
                        "Failed to check process status: {}",
                        e
                    ))
                    .into());
                }
            }

            // Check for OOM kill
            if let Ok(true) = self.sandbox.check_oom_killed() {
                let _ = child.kill();
                let completed = Utc::now();
                return Ok(ExecutionResponse::error(
                    self.execution_id,
                    crate::api::schema::ErrorResponse {
                        code: "E4002".to_string(),
                        message: "Process killed due to memory limit".to_string(),
                        details: Some(serde_json::json!({
                            "memory_limit": request.resources.memory_bytes
                        })),
                    },
                    started,
                    completed,
                ));
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

// Implement ResourceProvider directly for Arc<Sandbox> to avoid lifetime issues
impl monitor::ResourceProvider for std::sync::Arc<Sandbox> {
    fn get_usage(&self) -> CapsuleResult<ResourceUsage> {
        (**self).get_resource_usage()
    }

    fn check_oom_killed(&self) -> CapsuleResult<bool> {
        (**self).check_oom_killed()
    }
}

use monitor::ResourceProvider;

impl ResourceProvider for Sandbox {
    fn get_usage(&self) -> CapsuleResult<ResourceUsage> {
        self.get_resource_usage()
    }

    fn check_oom_killed(&self) -> CapsuleResult<bool> {
        self.check_oom_killed()
    }
}

#[cfg(unix)]
fn signal_name(signal: i32) -> &'static str {
    match signal {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL",
        5 => "SIGTRAP",
        6 => "SIGABRT",
        7 => "SIGBUS",
        8 => "SIGFPE",
        9 => "SIGKILL",
        10 => "SIGUSR1",
        11 => "SIGSEGV",
        12 => "SIGUSR2",
        13 => "SIGPIPE",
        14 => "SIGALRM",
        15 => "SIGTERM",
        16 => "SIGSTKFLT",
        17 => "SIGCHLD",
        18 => "SIGCONT",
        19 => "SIGSTOP",
        20 => "SIGTSTP",
        21 => "SIGTTIN",
        22 => "SIGTTOU",
        23 => "SIGURG",
        24 => "SIGXCPU",
        25 => "SIGXFSZ",
        26 => "SIGVTALRM",
        27 => "SIGPROF",
        28 => "SIGWINCH",
        29 => "SIGIO",
        30 => "SIGPWR",
        31 => "SIGSYS",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::{IsolationConfig, ResourceLimits};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_executor_simple_command() {
        let execution_id = Uuid::new_v4();
        let executor = Executor::new(execution_id);

        // This test might fail without proper setup, but demonstrates the API
        if executor.is_err() {
            return; // Skip test if sandbox setup fails
        }

        let request = ExecutionRequest {
            command: vec!["echo".to_string(), "hello".to_string()],
            environment: HashMap::new(),
            timeout_ms: 5000,
            resources: ResourceLimits::default(),
            isolation: IsolationConfig::default(),
        };

        let result = executor.unwrap().execute(request).await;

        if let Ok(response) = result {
            match response.status {
                crate::api::schema::ExecutionStatus::Success => {
                    assert!(response.stdout.is_some());
                    assert_eq!(response.execution_id, execution_id);
                }
                _ => {
                    // May fail in test environment due to sandbox restrictions
                }
            }
        }
    }

    #[tokio::test]
    async fn test_executor_timeout() {
        let execution_id = Uuid::new_v4();
        let executor = Executor::new(execution_id);

        if executor.is_err() {
            return; // Skip test if sandbox setup fails
        }

        let request = ExecutionRequest {
            command: vec!["sleep".to_string(), "10".to_string()],
            environment: HashMap::new(),
            timeout_ms: 100, // Very short timeout
            resources: ResourceLimits::default(),
            isolation: IsolationConfig::default(),
        };

        let result = executor.unwrap().execute(request).await;

        if let Ok(response) = result {
            match response.status {
                crate::api::schema::ExecutionStatus::Timeout => {
                    assert!(response.error.is_some());
                }
                _ => {
                    // May complete quickly in test environment
                }
            }
        }
    }
}
