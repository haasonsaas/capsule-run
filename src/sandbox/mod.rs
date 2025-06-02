#[cfg(target_os = "linux")]
pub mod cgroups;
#[cfg(target_os = "linux")]
pub mod filesystem;
#[cfg(target_os = "linux")]
pub mod namespaces;
#[cfg(target_os = "linux")]
pub mod seccomp;

#[cfg(target_os = "linux")]
use crate::api::schema::{IsolationConfig, ResourceLimits};
#[cfg(target_os = "linux")]
use crate::error::{CapsuleResult, SandboxError};
#[cfg(target_os = "linux")]
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub use cgroups::{CgroupManager, ResourceUsage};
#[cfg(target_os = "linux")]
pub use filesystem::FilesystemManager;
#[cfg(target_os = "linux")]
pub use namespaces::NamespaceManager;
#[cfg(target_os = "linux")]
pub use seccomp::SeccompFilter;

// Stub implementations for non-Linux platforms
#[cfg(not(target_os = "linux"))]
#[allow(dead_code)] // Stub for non-Linux platforms
pub struct NamespaceManager;
#[cfg(not(target_os = "linux"))]
#[allow(dead_code)] // Stub for non-Linux platforms
pub struct CgroupManager;
#[cfg(not(target_os = "linux"))]
#[allow(dead_code)] // Stub for non-Linux platforms
pub struct SeccompFilter;
#[cfg(not(target_os = "linux"))]
#[allow(dead_code)] // Stub for non-Linux platforms
pub struct FilesystemManager;

#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_bytes: u64,
    pub cpu_time_us: u64,
    pub user_time_us: u64,
    pub kernel_time_us: u64,
    pub io_bytes_read: u64,
    pub io_bytes_written: u64,
}

#[cfg(target_os = "linux")]
pub struct Sandbox {
    pub execution_id: Uuid,
    pub namespace_manager: NamespaceManager,
    pub cgroup_manager: CgroupManager,
    pub filesystem_manager: FilesystemManager,
    pub seccomp_filter: SeccompFilter,
}

#[cfg(not(target_os = "linux"))]
#[allow(dead_code)] // Fields are part of API design but not yet used
pub struct Sandbox {
    pub execution_id: uuid::Uuid,
}

#[cfg(target_os = "linux")]
impl Sandbox {
    pub fn new(execution_id: Uuid) -> CapsuleResult<Self> {
        let namespace_manager = NamespaceManager::new();
        let cgroup_manager = CgroupManager::new(execution_id)?;
        let filesystem_manager = FilesystemManager::new(execution_id)?;
        let seccomp_filter = SeccompFilter::new()?;

        Ok(Self {
            execution_id,
            namespace_manager,
            cgroup_manager,
            filesystem_manager,
            seccomp_filter,
        })
    }

    pub fn setup(
        &mut self,
        resources: &ResourceLimits,
        isolation: &IsolationConfig,
    ) -> CapsuleResult<()> {
        // Stage 1: Setup privileged operations
        self.namespace_manager.setup_namespaces(isolation.network)?;
        self.cgroup_manager.setup(resources)?;

        // Setup filesystem isolation
        self.filesystem_manager.setup_isolation(isolation)?;

        // Setup seccomp filter
        self.seccomp_filter.setup_allowlist()?;

        if isolation.network {
            self.seccomp_filter = self.seccomp_filter.with_network_access()?;
        }

        // Stage 2: Enter namespace and apply security restrictions
        NamespaceManager::enter_namespaces()?;

        // Drop capabilities
        self.drop_capabilities()?;

        // Apply seccomp filter (must be last)
        self.seccomp_filter.apply()?;

        Ok(())
    }

    fn drop_capabilities(&self) -> CapsuleResult<()> {
        use caps::{clear, CapSet};

        // Clear all capability sets
        clear(None, CapSet::Effective).map_err(|e| {
            SandboxError::CapabilityDrop(format!("Failed to clear effective capabilities: {}", e))
        })?;

        clear(None, CapSet::Permitted).map_err(|e| {
            SandboxError::CapabilityDrop(format!("Failed to clear permitted capabilities: {}", e))
        })?;

        clear(None, CapSet::Inheritable).map_err(|e| {
            SandboxError::CapabilityDrop(format!("Failed to clear inheritable capabilities: {}", e))
        })?;

        Ok(())
    }

    pub fn get_resource_usage(&self) -> CapsuleResult<ResourceUsage> {
        self.cgroup_manager.get_usage()
    }

    pub fn check_oom_killed(&self) -> CapsuleResult<bool> {
        self.cgroup_manager.check_oom_killed()
    }

    pub fn cleanup(&self) -> CapsuleResult<()> {
        self.cgroup_manager.cleanup()?;
        self.filesystem_manager.cleanup()?;
        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
impl Sandbox {
    pub fn new(execution_id: uuid::Uuid) -> crate::error::CapsuleResult<Self> {
        Ok(Self { execution_id })
    }

    pub fn setup(
        &mut self,
        _resources: &crate::api::ResourceLimits,
        _isolation: &crate::api::IsolationConfig,
    ) -> crate::error::CapsuleResult<()> {
        Err(crate::error::CapsuleError::Config(
            "Sandbox functionality is only available on Linux".to_string(),
        ))
    }

    pub fn get_resource_usage(&self) -> crate::error::CapsuleResult<ResourceUsage> {
        Ok(ResourceUsage {
            memory_bytes: 0,
            cpu_time_us: 0,
            user_time_us: 0,
            kernel_time_us: 0,
            io_bytes_read: 0,
            io_bytes_written: 0,
        })
    }

    pub fn check_oom_killed(&self) -> crate::error::CapsuleResult<bool> {
        Ok(false)
    }

    pub fn cleanup(&self) -> crate::error::CapsuleResult<()> {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
