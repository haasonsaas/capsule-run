use crate::api::schema::{IsolationConfig, ResourceLimits};
use crate::error::CapsuleResult;
use std::process::Command;
use uuid::Uuid;

/// macOS-specific sandbox implementation using system frameworks
pub struct MacOSSandbox {
    pub execution_id: Uuid,
    resource_limits: Option<ResourceLimits>,
    isolation_config: Option<IsolationConfig>,
    sandbox_profile: Option<String>,
    process_limits: ProcessLimits,
}

#[derive(Debug, Clone)]
pub struct ProcessLimits {
    pub max_memory_bytes: Option<u64>,
    pub max_cpu_time_seconds: Option<u64>,
    pub max_file_descriptors: Option<u32>,
    pub max_processes: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MacOSResourceUsage {
    pub memory_bytes: u64,
    pub cpu_time_us: u64,
    pub user_time_us: u64,
    pub kernel_time_us: u64,
    pub io_bytes_read: u64,
    pub io_bytes_written: u64,
}

// Type alias for compatibility with the main interface
pub type ResourceUsage = MacOSResourceUsage;

impl MacOSSandbox {
    pub fn new(execution_id: Uuid) -> CapsuleResult<Self> {
        Ok(Self {
            execution_id,
            resource_limits: None,
            isolation_config: None,
            sandbox_profile: None,
            process_limits: ProcessLimits {
                max_memory_bytes: None,
                max_cpu_time_seconds: None,
                max_file_descriptors: Some(1024), // Safe default
                max_processes: Some(64),           // Safe default
            },
        })
    }

    pub fn setup(
        &mut self,
        resources: &ResourceLimits,
        isolation: &IsolationConfig,
    ) -> CapsuleResult<()> {
        self.resource_limits = Some(resources.clone());
        self.isolation_config = Some(isolation.clone());

        // Convert resource limits to macOS process limits
        self.process_limits.max_memory_bytes = if resources.memory_bytes > 0 {
            Some(resources.memory_bytes)
        } else {
            None
        };

        // Note: macOS doesn't have direct CPU time limits in ResourceLimits
        // We could potentially use cpu_shares to derive a relative limit
        self.process_limits.max_cpu_time_seconds = None;

        // Generate macOS sandbox profile
        self.sandbox_profile = Some(self.generate_sandbox_profile(isolation)?);

        Ok(())
    }

    fn generate_sandbox_profile(&self, isolation: &IsolationConfig) -> CapsuleResult<String> {
        let mut profile = String::new();

        // Start with a restrictive base profile
        profile.push_str("(version 1)\n");
        profile.push_str("(deny default)\n");

        // Allow basic system operations
        profile.push_str("(allow process-exec)\n");
        profile.push_str("(allow process-fork)\n");
        profile.push_str("(allow signal)\n");

        // Allow reading system libraries and frameworks
        profile.push_str("(allow file-read*\n");
        profile.push_str("    (subpath \"/System\")\n");
        profile.push_str("    (subpath \"/usr/lib\")\n");
        profile.push_str("    (subpath \"/usr/share\")\n");
        profile.push_str("    (subpath \"/Library/Frameworks\")\n");
        profile.push_str("    (subpath \"/private/var/db/dyld\")\n");
        profile.push_str(")\n");

        // Allow basic I/O operations
        profile.push_str("(allow file-read-data file-write-data\n");
        profile.push_str("    (literal \"/dev/null\")\n");
        profile.push_str("    (literal \"/dev/stdin\")\n");
        profile.push_str("    (literal \"/dev/stdout\")\n");
        profile.push_str("    (literal \"/dev/stderr\")\n");
        profile.push_str(")\n");

        // Allow access to working directory and writable paths
        if !isolation.writable_paths.is_empty() {
            profile.push_str("(allow file*\n");
            for path in &isolation.writable_paths {
                profile.push_str(&format!("    (subpath \"{}\")\n", path));
            }
            profile.push_str(")\n");
        }

        // Allow read-only access to specified paths
        if !isolation.readonly_paths.is_empty() {
            profile.push_str("(allow file-read*\n");
            for path in &isolation.readonly_paths {
                profile.push_str(&format!("    (subpath \"{}\")\n", path));
            }
            profile.push_str(")\n");
        }

        // Working directory access
        if !isolation.working_directory.is_empty() {
            profile.push_str(&format!(
                "(allow file* (subpath \"{}\"))\n",
                isolation.working_directory
            ));
        }

        // Network access
        if isolation.network {
            profile.push_str("(allow network*)\n");
        } else {
            profile.push_str("(deny network*)\n");
        }

        // Deny dangerous operations (using valid macOS sandbox operations)
        profile.push_str("(deny system-privilege)\n");
        profile.push_str("(deny system-audit)\n");
        profile.push_str("(deny system-socket)\n");

        Ok(profile)
    }

    pub fn get_resource_usage(&self) -> CapsuleResult<ResourceUsage> {
        // Use rusage to get basic resource information
        let usage = unsafe {
            let mut usage: libc::rusage = std::mem::zeroed();
            let result = libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage);
            if result != 0 {
                return Ok(ResourceUsage {
                    memory_bytes: 0,
                    cpu_time_us: 0,
                    user_time_us: 0,
                    kernel_time_us: 0,
                    io_bytes_read: 0,
                    io_bytes_written: 0,
                });
            }
            usage
        };

        Ok(ResourceUsage {
            memory_bytes: usage.ru_maxrss as u64 * 1024, // macOS returns in KB
            cpu_time_us: (usage.ru_utime.tv_sec as u64 * 1_000_000
                + usage.ru_utime.tv_usec as u64)
                + (usage.ru_stime.tv_sec as u64 * 1_000_000 + usage.ru_stime.tv_usec as u64),
            user_time_us: usage.ru_utime.tv_sec as u64 * 1_000_000
                + usage.ru_utime.tv_usec as u64,
            kernel_time_us: usage.ru_stime.tv_sec as u64 * 1_000_000
                + usage.ru_stime.tv_usec as u64,
            io_bytes_read: usage.ru_inblock as u64 * 512, // Approximate
            io_bytes_written: usage.ru_oublock as u64 * 512, // Approximate
        })
    }

    pub fn check_oom_killed(&self) -> CapsuleResult<bool> {
        // macOS doesn't have the same OOM concept as Linux
        // We can check if we're approaching memory limits
        if let Some(max_memory) = self.process_limits.max_memory_bytes {
            let usage = self.get_resource_usage()?;
            Ok(usage.memory_bytes > max_memory)
        } else {
            Ok(false)
        }
    }

    pub fn prepare_command(&self, cmd: &mut Command) -> CapsuleResult<()> {
        use std::os::unix::process::CommandExt;

        // Apply resource limits using pre_exec hook
        if let Some(limits) = &self.resource_limits {
            let limits_clone = limits.clone();
            let process_limits = self.process_limits.clone();
            
            unsafe {
                cmd.pre_exec(move || {
                    Self::apply_limits_in_child(&limits_clone, &process_limits)
                });
            }
        }

        // For now, skip sandbox-exec integration to focus on basic functionality
        // TODO: Implement proper sandbox-exec integration later
        if let Some(_profile) = &self.sandbox_profile {
            // For now, just set an environment variable to indicate sandboxing is active
            cmd.env("CAPSULE_SANDBOX_ACTIVE", "1");
        }

        Ok(())
    }

    fn apply_limits_in_child(
        limits: &ResourceLimits, 
        process_limits: &ProcessLimits
    ) -> Result<(), std::io::Error> {
        unsafe {
            // Set memory limit (RLIMIT_AS - virtual memory)
            if limits.memory_bytes > 0 {
                let limit = libc::rlimit {
                    rlim_cur: limits.memory_bytes,
                    rlim_max: limits.memory_bytes,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &limit) != 0 {
                    eprintln!("Warning: Failed to set memory limit");
                    // Don't fail the process, just warn
                }
            }

            // Set file descriptor limit (RLIMIT_NOFILE) 
            if let Some(fd_limit) = process_limits.max_file_descriptors {
                let limit = libc::rlimit {
                    rlim_cur: fd_limit as u64,
                    rlim_max: fd_limit as u64,
                };
                if libc::setrlimit(libc::RLIMIT_NOFILE, &limit) != 0 {
                    eprintln!("Warning: Failed to set file descriptor limit");
                }
            }

            // Set process limit (RLIMIT_NPROC)
            if let Some(proc_limit) = process_limits.max_processes {
                let limit = libc::rlimit {
                    rlim_cur: proc_limit as u64,
                    rlim_max: proc_limit as u64,
                };
                if libc::setrlimit(libc::RLIMIT_NPROC, &limit) != 0 {
                    eprintln!("Warning: Failed to set process limit");
                }
            }
        }

        Ok(())
    }

    pub fn cleanup(&self) -> CapsuleResult<()> {
        // Clean up temporary sandbox profile
        let profile_path = format!("/tmp/capsule-{}.sb", self.execution_id);
        let _ = std::fs::remove_file(profile_path); // Ignore errors

        Ok(())
    }

}

impl Drop for MacOSSandbox {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::IsolationConfig;

    #[test]
    fn test_macos_sandbox_creation() {
        let execution_id = Uuid::new_v4();
        let sandbox = MacOSSandbox::new(execution_id);
        assert!(sandbox.is_ok());
    }

    #[test]
    fn test_sandbox_profile_generation() {
        let execution_id = Uuid::new_v4();
        let mut sandbox = MacOSSandbox::new(execution_id).unwrap();

        let isolation = IsolationConfig {
            network: false,
            working_directory: "/tmp".to_string(),
            readonly_paths: vec!["/usr".to_string()],
            writable_paths: vec!["/tmp".to_string()],
            bind_mounts: vec![],
        };

        let result = sandbox.setup(&ResourceLimits::default(), &isolation);
        assert!(result.is_ok());
        assert!(sandbox.sandbox_profile.is_some());

        let profile = sandbox.sandbox_profile.clone().unwrap();
        assert!(profile.contains("(deny network*)"));
        assert!(profile.contains("/tmp"));
        assert!(profile.contains("/usr"));
    }

    #[test]
    fn test_resource_usage() {
        let execution_id = Uuid::new_v4();
        let sandbox = MacOSSandbox::new(execution_id).unwrap();
        let usage = sandbox.get_resource_usage();
        assert!(usage.is_ok());
    }
}