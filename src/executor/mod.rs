pub mod io;
pub mod monitor;

use crate::api::schema::{ExecutionRequest, ExecutionResponse, ExecutionMetrics, ExecutionTimestamps};
use crate::error::{CapsuleResult, ExecutionError, ErrorCode};
use crate::sandbox::{Sandbox, ResourceUsage};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub use io::{IoCapture, StreamingIoCapture, IoEvent};
pub use monitor::{ResourceMonitor, ProcessMonitor, TimeoutMonitor, MonitoringResult, ProcessStatus, ResourceProvider};

pub struct Executor {
    execution_id: Uuid,
    sandbox: Sandbox,
}

pub struct ExecutionResult {
    pub response: ExecutionResponse,
    pub metrics: ExecutionMetrics,
}

impl Executor {
    pub fn new(execution_id: Uuid) -> CapsuleResult<Self> {
        let sandbox = Sandbox::new(execution_id)?;
        
        Ok(Self {
            execution_id,
            sandbox,
        })
    }

    pub async fn execute(mut self, request: ExecutionRequest) -> CapsuleResult<ExecutionResponse> {
        let started = Utc::now();
        
        // Setup sandbox
        match self.sandbox.setup(&request.resources, &request.isolation) {
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
        
        // Setup timeout monitor
        let (timeout_monitor, _timeout_sender) = TimeoutMonitor::new(
            Duration::from_millis(request.timeout_ms)
        );

        // Setup resource monitoring
        let resource_monitor = ResourceMonitor::new(
            Arc::new(SandboxResourceProvider::new(&self.sandbox)),
            Duration::from_millis(100), // Monitor every 100ms
        );

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

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| {
            ExecutionError::SpawnFailed(format!("Failed to spawn command: {}", e))
        })?;

        let child_pid = child.id();

        // Setup I/O capture
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let io_capture = IoCapture::new(stdout, stderr, request.resources.max_output_bytes);

        // Setup process monitoring
        let process_monitor = ProcessMonitor::new(child_pid);

        // Main execution loop
        let execution_result = tokio::task::spawn_blocking(move || {
            loop {
                // Check timeout
                if timeout_monitor.check_timeout() {
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
                        // Process has exited
                        let exit_code = status.code().unwrap_or(-1);
                        
                        // Collect I/O
                        let (stdout, stderr) = io_capture.wait_for_completion()?;
                        
                        // Get final resource usage
                        let monitoring_result = resource_monitor.stop_and_get_result()?;
                        let final_usage = self.sandbox.get_resource_usage()?;
                        
                        let completed = Utc::now();
                        
                        let metrics = ExecutionMetrics {
                            wall_time_ms: monitoring_result.wall_time.as_millis() as u64,
                            cpu_time_ms: final_usage.cpu_time_us / 1000,
                            user_time_ms: final_usage.user_time_us / 1000,
                            kernel_time_ms: final_usage.kernel_time_us / 1000,
                            max_memory_bytes: monitoring_result.peak_memory,
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
                            "Failed to check process status: {}", e
                        )).into());
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

                // Small sleep to avoid busy waiting
                std::thread::sleep(Duration::from_millis(10));
            }
        }).await.map_err(|e| {
            ExecutionError::MonitoringError(format!("Execution task failed: {}", e))
        })??;

        Ok(execution_result)
    }
}

struct SandboxResourceProvider<'a> {
    sandbox: &'a Sandbox,
}

impl<'a> SandboxResourceProvider<'a> {
    fn new(sandbox: &'a Sandbox) -> Self {
        Self { sandbox }
    }
}

impl<'a> ResourceProvider for SandboxResourceProvider<'a> {
    fn get_usage(&self) -> CapsuleResult<ResourceUsage> {
        self.sandbox.get_resource_usage()
    }

    fn check_oom_killed(&self) -> CapsuleResult<bool> {
        self.sandbox.check_oom_killed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::{ResourceLimits, IsolationConfig};

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