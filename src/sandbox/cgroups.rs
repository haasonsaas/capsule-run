use crate::api::schema::ResourceLimits;
use crate::error::{CapsuleResult, SandboxError};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use uuid::Uuid;

pub struct CgroupManager {
    cgroup_path: PathBuf,
    execution_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_bytes: u64,
    pub cpu_time_us: u64,
    pub user_time_us: u64,
    pub kernel_time_us: u64,
    pub io_bytes_read: u64,
    pub io_bytes_written: u64,
}

impl CgroupManager {
    pub fn new(execution_id: Uuid) -> CapsuleResult<Self> {
        let cgroup_base = Self::find_cgroup_mount()?;
        let cgroup_path = cgroup_base
            .join("capsule-run")
            .join(execution_id.to_string());

        Ok(Self {
            cgroup_path,
            execution_id,
        })
    }

    pub fn setup(&self, limits: &ResourceLimits) -> CapsuleResult<()> {
        self.create_cgroup()?;
        self.set_memory_limit(limits.memory_bytes)?;
        self.set_cpu_limit(limits.cpu_shares)?;
        self.set_pids_limit(limits.max_pids)?;
        self.set_io_limits()?;
        self.add_current_process()?;
        Ok(())
    }

    pub fn cleanup(&self) -> CapsuleResult<()> {
        if self.cgroup_path.exists() {
            fs::remove_dir_all(&self.cgroup_path).map_err(|e| {
                SandboxError::CgroupSetup(format!(
                    "Failed to cleanup cgroup {}: {}",
                    self.cgroup_path.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }

    pub fn get_usage(&self) -> CapsuleResult<ResourceUsage> {
        let memory = self.get_memory_usage()?;
        let (cpu_time, user_time, kernel_time) = self.get_cpu_usage()?;
        let (io_read, io_written) = self.get_io_usage()?;

        Ok(ResourceUsage {
            memory_bytes: memory,
            cpu_time_us: cpu_time,
            user_time_us: user_time,
            kernel_time_us: kernel_time,
            io_bytes_read: io_read,
            io_bytes_written: io_written,
        })
    }

    fn find_cgroup_mount() -> CapsuleResult<PathBuf> {
        let mounts = fs::read_to_string("/proc/mounts").map_err(|e| {
            SandboxError::CgroupSetup(format!("Failed to read /proc/mounts: {}", e))
        })?;

        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[2] == "cgroup2" {
                return Ok(PathBuf::from(parts[1]));
            }
        }

        Err(SandboxError::CgroupSetup("cgroups v2 not mounted".to_string()).into())
    }

    fn create_cgroup(&self) -> CapsuleResult<()> {
        if let Some(parent) = self.cgroup_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                SandboxError::CgroupSetup(format!(
                    "Failed to create parent cgroup directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        fs::create_dir_all(&self.cgroup_path).map_err(|e| {
            SandboxError::CgroupSetup(format!(
                "Failed to create cgroup directory {}: {}",
                self.cgroup_path.display(),
                e
            ))
        })?;

        let controllers = "+memory +cpu +pids +io";
        self.write_cgroup_file("cgroup.subtree_control", controllers)?;

        Ok(())
    }

    fn set_memory_limit(&self, limit_bytes: u64) -> CapsuleResult<()> {
        self.write_cgroup_file("memory.max", &limit_bytes.to_string())?;
        self.write_cgroup_file("memory.swap.max", "0")?; // Disable swap

        let low_limit = limit_bytes / 2;
        self.write_cgroup_file("memory.low", &low_limit.to_string())?;

        Ok(())
    }

    fn set_cpu_limit(&self, cpu_shares: u32) -> CapsuleResult<()> {
        self.write_cgroup_file("cpu.weight", &cpu_shares.to_string())?;
        Ok(())
    }

    fn set_pids_limit(&self, max_pids: u32) -> CapsuleResult<()> {
        self.write_cgroup_file("pids.max", &max_pids.to_string())?;
        Ok(())
    }

    fn set_io_limits(&self) -> CapsuleResult<()> {
        self.write_cgroup_file("io.weight", "100")?;
        Ok(())
    }

    fn add_current_process(&self) -> CapsuleResult<()> {
        let pid = std::process::id();
        self.write_cgroup_file("cgroup.procs", &pid.to_string())?;
        Ok(())
    }

    fn write_cgroup_file(&self, filename: &str, content: &str) -> CapsuleResult<()> {
        let file_path = self.cgroup_path.join(filename);

        let mut file = OpenOptions::new()
            .write(true)
            .create(false)
            .open(&file_path)
            .map_err(|e| {
                SandboxError::CgroupSetup(format!(
                    "Failed to open cgroup file {}: {}",
                    file_path.display(),
                    e
                ))
            })?;

        file.write_all(content.as_bytes()).map_err(|e| {
            SandboxError::CgroupSetup(format!(
                "Failed to write to cgroup file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        Ok(())
    }

    fn read_cgroup_file(&self, filename: &str) -> CapsuleResult<String> {
        let file_path = self.cgroup_path.join(filename);

        let mut content = String::new();
        let mut file = File::open(&file_path).map_err(|e| {
            SandboxError::CgroupSetup(format!(
                "Failed to open cgroup file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        file.read_to_string(&mut content).map_err(|e| {
            SandboxError::CgroupSetup(format!(
                "Failed to read cgroup file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        Ok(content.trim().to_string())
    }

    fn get_memory_usage(&self) -> CapsuleResult<u64> {
        let content = self.read_cgroup_file("memory.current")?;
        content.parse::<u64>().map_err(|e| {
            SandboxError::CgroupSetup(format!("Failed to parse memory usage: {}", e)).into()
        })
    }

    fn get_cpu_usage(&self) -> CapsuleResult<(u64, u64, u64)> {
        let content = self.read_cgroup_file("cpu.stat")?;

        let mut usage_usec = 0u64;
        let mut user_usec = 0u64;
        let mut system_usec = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                match parts[0] {
                    "usage_usec" => {
                        usage_usec = parts[1].parse().unwrap_or(0);
                    }
                    "user_usec" => {
                        user_usec = parts[1].parse().unwrap_or(0);
                    }
                    "system_usec" => {
                        system_usec = parts[1].parse().unwrap_or(0);
                    }
                    _ => {}
                }
            }
        }

        Ok((usage_usec, user_usec, system_usec))
    }

    fn get_io_usage(&self) -> CapsuleResult<(u64, u64)> {
        let content = self.read_cgroup_file("io.stat")?;

        let mut bytes_read = 0u64;
        let mut bytes_written = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                for part in &parts[1..] {
                    if let Some((key, value)) = part.split_once('=') {
                        match key {
                            "rbytes" => {
                                bytes_read += value.parse().unwrap_or(0);
                            }
                            "wbytes" => {
                                bytes_written += value.parse().unwrap_or(0);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok((bytes_read, bytes_written))
    }

    pub fn get_events_fd(&self) -> CapsuleResult<File> {
        let events_path = self.cgroup_path.join("memory.events");
        File::open(&events_path).map_err(|e| {
            SandboxError::CgroupSetup(format!(
                "Failed to open events file {}: {}",
                events_path.display(),
                e
            ))
            .into()
        })
    }

    pub fn check_oom_killed(&self) -> CapsuleResult<bool> {
        let content = self.read_cgroup_file("memory.events")?;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == "oom_kill" {
                let count: u64 = parts[1].parse().unwrap_or(0);
                return Ok(count > 0);
            }
        }

        Ok(false)
    }
}

impl Drop for CgroupManager {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cgroup_manager_creation() {
        let execution_id = Uuid::new_v4();
        let result = CgroupManager::new(execution_id);

        match result {
            Ok(manager) => {
                assert!(manager
                    .cgroup_path
                    .to_string_lossy()
                    .contains(&execution_id.to_string()));
            }
            Err(_) => {}
        }
    }

    #[test]
    fn test_find_cgroup_mount() {
        let result = CgroupManager::find_cgroup_mount();

        match result {
            Ok(path) => {
                assert!(path.exists());
            }
            Err(_) => {}
        }
    }
}
