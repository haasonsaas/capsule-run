use crate::api::schema::{ExecutionRequest, ResourceLimits, IsolationConfig};
use crate::error::{CapsuleError, CapsuleResult};
use std::path::Path;

const MAX_MEMORY_BYTES: u64 = 2_147_483_648; // 2 GB
const MAX_TIMEOUT_MS: u64 = 600_000; // 10 minutes
const MAX_OUTPUT_BYTES: usize = 10_485_760; // 10 MB
const MAX_COMMAND_LENGTH: usize = 1000;
const MAX_ENV_VARS: usize = 100;
const MAX_ENV_VALUE_LENGTH: usize = 4096;

pub fn validate_execution_request(request: &ExecutionRequest) -> CapsuleResult<()> {
    validate_command(&request.command)?;
    validate_environment(&request.environment)?;
    validate_timeout(request.timeout_ms)?;
    validate_resources(&request.resources)?;
    validate_isolation(&request.isolation)?;
    Ok(())
}

fn validate_command(command: &[String]) -> CapsuleResult<()> {
    if command.is_empty() {
        return Err(CapsuleError::Config("Command cannot be empty".to_string()));
    }

    if command.len() > MAX_COMMAND_LENGTH {
        return Err(CapsuleError::Config(format!(
            "Command too long: {} arguments (max: {})",
            command.len(),
            MAX_COMMAND_LENGTH
        )));
    }

    for (i, arg) in command.iter().enumerate() {
        if arg.is_empty() {
            return Err(CapsuleError::Config(format!(
                "Command argument {} cannot be empty",
                i
            )));
        }

        if arg.len() > 4096 {
            return Err(CapsuleError::Config(format!(
                "Command argument {} too long: {} characters (max: 4096)",
                i,
                arg.len()
            )));
        }

        if arg.contains('\0') {
            return Err(CapsuleError::Config(format!(
                "Command argument {} contains null byte",
                i
            )));
        }
    }

    let executable = &command[0];
    if executable.starts_with('/') && !is_safe_path(executable) {
        return Err(CapsuleError::Config(format!(
            "Executable path '{}' is not allowed",
            executable
        )));
    }

    Ok(())
}

fn validate_environment(env: &std::collections::HashMap<String, String>) -> CapsuleResult<()> {
    if env.len() > MAX_ENV_VARS {
        return Err(CapsuleError::Config(format!(
            "Too many environment variables: {} (max: {})",
            env.len(),
            MAX_ENV_VARS
        )));
    }

    for (key, value) in env {
        if key.is_empty() {
            return Err(CapsuleError::Config(
                "Environment variable key cannot be empty".to_string(),
            ));
        }

        if key.len() > 256 {
            return Err(CapsuleError::Config(format!(
                "Environment variable key '{}' too long (max: 256 characters)",
                key
            )));
        }

        if value.len() > MAX_ENV_VALUE_LENGTH {
            return Err(CapsuleError::Config(format!(
                "Environment variable '{}' value too long: {} characters (max: {})",
                key,
                value.len(),
                MAX_ENV_VALUE_LENGTH
            )));
        }

        if key.contains('=') || key.contains('\0') {
            return Err(CapsuleError::Config(format!(
                "Environment variable key '{}' contains invalid characters",
                key
            )));
        }

        if value.contains('\0') {
            return Err(CapsuleError::Config(format!(
                "Environment variable '{}' value contains null byte",
                key
            )));
        }

        if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(CapsuleError::Config(format!(
                "Environment variable key '{}' contains invalid characters (only alphanumeric and underscore allowed)",
                key
            )));
        }
    }

    Ok(())
}

fn validate_timeout(timeout_ms: u64) -> CapsuleResult<()> {
    if timeout_ms == 0 {
        return Err(CapsuleError::Config(
            "Timeout must be greater than 0".to_string(),
        ));
    }

    if timeout_ms > MAX_TIMEOUT_MS {
        return Err(CapsuleError::Config(format!(
            "Timeout too long: {}ms (max: {}ms)",
            timeout_ms, MAX_TIMEOUT_MS
        )));
    }

    Ok(())
}

fn validate_resources(resources: &ResourceLimits) -> CapsuleResult<()> {
    if resources.memory_bytes == 0 {
        return Err(CapsuleError::Config(
            "Memory limit must be greater than 0".to_string(),
        ));
    }

    if resources.memory_bytes > MAX_MEMORY_BYTES {
        return Err(CapsuleError::Config(format!(
            "Memory limit too high: {} bytes (max: {} bytes)",
            resources.memory_bytes, MAX_MEMORY_BYTES
        )));
    }

    if resources.memory_bytes < 1_048_576 {
        return Err(CapsuleError::Config(
            "Memory limit too low: minimum 1MB required".to_string(),
        ));
    }

    if resources.cpu_shares == 0 {
        return Err(CapsuleError::Config(
            "CPU shares must be greater than 0".to_string(),
        ));
    }

    if resources.cpu_shares > 10240 {
        return Err(CapsuleError::Config(format!(
            "CPU shares too high: {} (max: 10240)",
            resources.cpu_shares
        )));
    }

    if resources.max_output_bytes > MAX_OUTPUT_BYTES {
        return Err(CapsuleError::Config(format!(
            "Output limit too high: {} bytes (max: {} bytes)",
            resources.max_output_bytes, MAX_OUTPUT_BYTES
        )));
    }

    if resources.max_pids == 0 {
        return Err(CapsuleError::Config(
            "PID limit must be greater than 0".to_string(),
        ));
    }

    if resources.max_pids > 1000 {
        return Err(CapsuleError::Config(format!(
            "PID limit too high: {} (max: 1000)",
            resources.max_pids
        )));
    }

    Ok(())
}

fn validate_isolation(isolation: &IsolationConfig) -> CapsuleResult<()> {
    validate_path(&isolation.working_directory, "Working directory")?;

    for path in &isolation.readonly_paths {
        validate_path(path, "Read-only path")?;
    }

    for path in &isolation.writable_paths {
        validate_path(path, "Writable path")?;
    }

    for bind_mount in &isolation.bind_mounts {
        validate_path(&bind_mount.source, "Bind mount source")?;
        validate_path(&bind_mount.destination, "Bind mount destination")?;
    }

    if isolation.readonly_paths.len() + isolation.writable_paths.len() > 50 {
        return Err(CapsuleError::Config(
            "Too many path configurations (max: 50 total)".to_string(),
        ));
    }

    if isolation.bind_mounts.len() > 20 {
        return Err(CapsuleError::Config(format!(
            "Too many bind mounts: {} (max: 20)",
            isolation.bind_mounts.len()
        )));
    }

    Ok(())
}

fn validate_path(path: &str, path_type: &str) -> CapsuleResult<()> {
    if path.is_empty() {
        return Err(CapsuleError::Config(format!("{} cannot be empty", path_type)));
    }

    if !path.starts_with('/') {
        return Err(CapsuleError::Config(format!(
            "{} must be absolute: {}",
            path_type, path
        )));
    }

    if path.len() > 4096 {
        return Err(CapsuleError::Config(format!(
            "{} too long: {} characters (max: 4096)",
            path_type,
            path.len()
        )));
    }

    if path.contains('\0') {
        return Err(CapsuleError::Config(format!(
            "{} contains null byte: {}",
            path_type, path
        )));
    }

    if !is_safe_path(path) {
        return Err(CapsuleError::Config(format!(
            "{} is not safe: {}",
            path_type, path
        )));
    }

    Ok(())
}

fn is_safe_path(path: &str) -> bool {
    let path = Path::new(path);
    
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => return false,
            std::path::Component::Normal(name) => {
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') && name_str != "." {
                    continue;
                }
            }
            _ => continue,
        }
    }

    let dangerous_paths = [
        "/proc/sys",
        "/proc/sysrq-trigger",
        "/proc/kcore",
        "/proc/kmem",
        "/proc/mem",
        "/sys/kernel",
        "/sys/devices",
        "/dev/mem",
        "/dev/kmem",
        "/dev/port",
        "/boot",
        "/etc/passwd",
        "/etc/shadow",
        "/etc/sudoers",
        "/etc/ssh",
        "/root",
        "/home",
    ];

    let path_str = path.to_string_lossy();
    for dangerous in &dangerous_paths {
        if path_str.starts_with(dangerous) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_validate_command_empty() {
        let result = validate_command(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_command_valid() {
        let result = validate_command(&["python".to_string(), "-c".to_string(), "print('hello')".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_command_null_byte() {
        let result = validate_command(&["python\0".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_environment_valid() {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin".to_string());
        let result = validate_environment(&env);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_environment_invalid_key() {
        let mut env = HashMap::new();
        env.insert("PATH=".to_string(), "/usr/bin".to_string());
        let result = validate_environment(&env);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_dangerous() {
        let result = validate_path("/proc/sys/kernel", "Test path");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_parent_dir() {
        let result = validate_path("/some/../path", "Test path");
        assert!(result.is_err());
    }
}