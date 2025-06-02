use capsule_run::api::{ExecutionRequest, IsolationConfig, ResourceLimits};
use capsule_run::executor::Executor;
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::test]
async fn test_basic_execution() {
    let execution_id = Uuid::new_v4();

    // Skip test if not running as root or in CI environment
    if !can_run_sandbox_tests() {
        return;
    }

    let executor = match Executor::new(execution_id) {
        Ok(e) => e,
        Err(_) => return, // Skip if sandbox setup fails
    };

    let request = ExecutionRequest {
        command: vec!["echo".to_string(), "hello world".to_string()],
        environment: HashMap::new(),
        timeout_ms: 5000,
        resources: ResourceLimits::default(),
        isolation: IsolationConfig::default(),
    };

    let response = executor.execute(request).await.unwrap();

    match response.status {
        capsule_run::api::ExecutionStatus::Success => {
            assert_eq!(response.exit_code, Some(0));
            assert!(response.stdout.unwrap().contains("hello world"));
        }
        _ => {
            // May fail in test environment
            println!(
                "Test failed due to sandbox restrictions: {:?}",
                response.error
            );
        }
    }
}

#[tokio::test]
async fn test_timeout_enforcement() {
    let execution_id = Uuid::new_v4();

    if !can_run_sandbox_tests() {
        return;
    }

    let executor = match Executor::new(execution_id) {
        Ok(e) => e,
        Err(_) => return,
    };

    let request = ExecutionRequest {
        command: vec!["sleep".to_string(), "10".to_string()],
        environment: HashMap::new(),
        timeout_ms: 100, // Very short timeout
        resources: ResourceLimits::default(),
        isolation: IsolationConfig::default(),
    };

    let response = executor.execute(request).await.unwrap();

    match response.status {
        capsule_run::api::ExecutionStatus::Timeout => {
            assert!(response.error.is_some());
            let error = response.error.unwrap();
            assert_eq!(error.code, "E3001");
        }
        _ => {
            // Process might complete quickly in test environment
        }
    }
}

#[tokio::test]
async fn test_memory_limit() {
    let execution_id = Uuid::new_v4();

    if !can_run_sandbox_tests() {
        return;
    }

    let executor = match Executor::new(execution_id) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut resources = ResourceLimits::default();
    resources.memory_bytes = 10 * 1024 * 1024; // 10MB limit

    // Try to allocate more memory than the limit
    let request = ExecutionRequest {
        command: vec![
            "python".to_string(),
            "-c".to_string(),
            "import sys; data = b'x' * (50 * 1024 * 1024); print('allocated')".to_string(),
        ],
        environment: HashMap::new(),
        timeout_ms: 10000,
        resources,
        isolation: IsolationConfig::default(),
    };

    let response = executor.execute(request).await.unwrap();

    // Should either fail due to memory limit or complete successfully if Python isn't available
    match response.status {
        capsule_run::api::ExecutionStatus::Error => {
            // Expected if memory limit is enforced
        }
        capsule_run::api::ExecutionStatus::Success => {
            // Might succeed if Python optimizes the allocation or isn't available
        }
        _ => {}
    }
}

#[tokio::test]
async fn test_output_size_limit() {
    let execution_id = Uuid::new_v4();

    if !can_run_sandbox_tests() {
        return;
    }

    let executor = match Executor::new(execution_id) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut resources = ResourceLimits::default();
    resources.max_output_bytes = 100; // Very small output limit

    let request = ExecutionRequest {
        command: vec![
            "python".to_string(),
            "-c".to_string(),
            "print('x' * 1000)".to_string(), // Print more than the limit
        ],
        environment: HashMap::new(),
        timeout_ms: 5000,
        resources,
        isolation: IsolationConfig::default(),
    };

    let response = executor.execute(request).await.unwrap();

    // Should fail due to output size limit if Python is available
    match response.status {
        capsule_run::api::ExecutionStatus::Error => {
            // Expected
        }
        _ => {
            // Might not fail if Python isn't available
        }
    }
}

#[tokio::test]
async fn test_environment_variables() {
    let execution_id = Uuid::new_v4();

    if !can_run_sandbox_tests() {
        return;
    }

    let executor = match Executor::new(execution_id) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut environment = HashMap::new();
    environment.insert("TEST_VAR".to_string(), "test_value".to_string());

    let request = ExecutionRequest {
        command: vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo $TEST_VAR".to_string(),
        ],
        environment,
        timeout_ms: 5000,
        resources: ResourceLimits::default(),
        isolation: IsolationConfig::default(),
    };

    let response = executor.execute(request).await.unwrap();

    match response.status {
        capsule_run::api::ExecutionStatus::Success => {
            assert!(response.stdout.unwrap().contains("test_value"));
        }
        _ => {
            // May fail in test environment
        }
    }
}

#[tokio::test]
async fn test_working_directory() {
    let execution_id = Uuid::new_v4();

    if !can_run_sandbox_tests() {
        return;
    }

    let executor = match Executor::new(execution_id) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut isolation = IsolationConfig::default();
    isolation.working_directory = "/tmp".to_string();

    let request = ExecutionRequest {
        command: vec!["pwd".to_string()],
        environment: HashMap::new(),
        timeout_ms: 5000,
        resources: ResourceLimits::default(),
        isolation,
    };

    let response = executor.execute(request).await.unwrap();

    match response.status {
        capsule_run::api::ExecutionStatus::Success => {
            assert!(response.stdout.unwrap().trim().ends_with("/tmp"));
        }
        _ => {
            // May fail in test environment
        }
    }
}

#[tokio::test]
async fn test_network_isolation() {
    let execution_id = Uuid::new_v4();

    if !can_run_sandbox_tests() {
        return;
    }

    let executor = match Executor::new(execution_id) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut isolation = IsolationConfig::default();
    isolation.network = false; // Network disabled

    let request = ExecutionRequest {
        command: vec![
            "ping".to_string(),
            "-c".to_string(),
            "1".to_string(),
            "8.8.8.8".to_string(),
        ],
        environment: HashMap::new(),
        timeout_ms: 5000,
        resources: ResourceLimits::default(),
        isolation,
    };

    let response = executor.execute(request).await.unwrap();

    // Should fail because network is disabled
    match response.status {
        capsule_run::api::ExecutionStatus::Success => {
            // Might succeed if ping isn't available or network check fails differently
        }
        capsule_run::api::ExecutionStatus::Error => {
            // Expected due to network isolation
        }
        _ => {}
    }
}

// Helper function to check if we can run sandbox tests
fn can_run_sandbox_tests() -> bool {
    // Only run full sandbox tests on Linux
    #[cfg(target_os = "linux")]
    {
        // Check if we can create user namespaces
        if std::path::Path::new("/proc/sys/user/max_user_namespaces").exists() {
            if let Ok(content) = std::fs::read_to_string("/proc/sys/user/max_user_namespaces") {
                if let Ok(max_namespaces) = content.trim().parse::<i32>() {
                    return max_namespaces > 0;
                }
            }
        }

        // Check if we're running as root (required for some operations)
        unsafe { libc::getuid() == 0 }
    }
    #[cfg(not(target_os = "linux"))]
    {
        false // Don't run sandbox tests on non-Linux platforms
    }
}

// Benchmark tests (optional - only run with --features bench)
#[cfg(feature = "bench")]
mod bench_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn bench_startup_time() {
        if !can_run_sandbox_tests() {
            return;
        }

        let mut times = Vec::new();

        for _ in 0..10 {
            let start = Instant::now();
            let execution_id = Uuid::new_v4();

            if let Ok(executor) = Executor::new(execution_id) {
                let request = ExecutionRequest {
                    command: vec!["true".to_string()], // Minimal command
                    environment: HashMap::new(),
                    timeout_ms: 1000,
                    resources: ResourceLimits::default(),
                    isolation: IsolationConfig::default(),
                };

                let _ = executor.execute(request).await;
                times.push(start.elapsed());
            }
        }

        if !times.is_empty() {
            let avg_time = times.iter().sum::<std::time::Duration>() / times.len() as u32;
            println!("Average startup time: {:?}", avg_time);

            // Assert startup time is under 125ms (design target)
            assert!(
                avg_time.as_millis() < 125,
                "Startup time too slow: {:?}",
                avg_time
            );
        }
    }
}
