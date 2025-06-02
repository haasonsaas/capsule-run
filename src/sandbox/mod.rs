#[cfg(target_os = "linux")]
pub mod cgroups;
#[cfg(target_os = "linux")]
pub mod filesystem;
#[cfg(target_os = "linux")]
pub mod namespaces;
#[cfg(all(target_os = "linux", feature = "seccomp"))]
pub mod seccomp;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::api::schema::{IsolationConfig, ResourceLimits};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::error::CapsuleResult;
#[cfg(target_os = "linux")]
use crate::error::SandboxError;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub use cgroups::{CgroupManager, ResourceUsage};
#[cfg(target_os = "linux")]
pub use filesystem::FilesystemManager;
#[cfg(target_os = "linux")]
pub use namespaces::NamespaceManager;
#[cfg(all(target_os = "linux", feature = "seccomp"))]
pub use seccomp::SeccompFilter;

#[cfg(target_os = "macos")]
pub use macos::{MacOSSandbox, ResourceUsage};

// Stub implementations for unsupported platforms (Windows, etc.)
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[allow(dead_code)] // Stub for unsupported platforms
pub struct NamespaceManager;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[allow(dead_code)] // Stub for unsupported platforms
pub struct CgroupManager;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[allow(dead_code)] // Stub for unsupported platforms
pub struct SeccompFilter;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[allow(dead_code)] // Stub for unsupported platforms
pub struct FilesystemManager;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
    #[allow(dead_code)] // Used for future tracking and debugging features
    pub execution_id: Uuid,
    pub namespace_manager: NamespaceManager,
    pub cgroup_manager: CgroupManager,
    pub filesystem_manager: FilesystemManager,
    #[cfg(feature = "seccomp")]
    pub seccomp_filter: SeccompFilter,
}

#[cfg(target_os = "macos")]
pub struct Sandbox {
    #[allow(dead_code)] // Used for future tracking and debugging features
    pub execution_id: Uuid,
    pub macos_sandbox: MacOSSandbox,
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
        #[cfg(feature = "seccomp")]
        let seccomp_filter = SeccompFilter::new()?;

        Ok(Self {
            execution_id,
            namespace_manager,
            cgroup_manager,
            filesystem_manager,
            #[cfg(feature = "seccomp")]
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
        #[cfg(feature = "seccomp")]
        self.seccomp_filter.setup_allowlist()?;

        #[cfg(feature = "seccomp")]
        if isolation.network {
            // Clone and replace the seccomp filter with network access
            let new_filter = SeccompFilter::new()?;
            // Setup basic allowlist first
            let mut new_filter = new_filter;
            new_filter.setup_allowlist()?;
            // Add network access
            self.seccomp_filter = new_filter.with_network_access()?;
        }

        // Stage 2: Enter namespace and apply security restrictions
        NamespaceManager::enter_namespaces()?;

        // Drop capabilities
        self.drop_capabilities()?;

        // Apply seccomp filter (must be last)
        #[cfg(feature = "seccomp")]
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

#[cfg(target_os = "macos")]
impl Sandbox {
    pub fn new(execution_id: Uuid) -> CapsuleResult<Self> {
        let macos_sandbox = MacOSSandbox::new(execution_id)?;
        Ok(Self {
            execution_id,
            macos_sandbox,
        })
    }

    pub fn setup(
        &mut self,
        resources: &ResourceLimits,
        isolation: &IsolationConfig,
    ) -> CapsuleResult<()> {
        self.macos_sandbox.setup(resources, isolation)
    }

    pub fn get_resource_usage(&self) -> CapsuleResult<ResourceUsage> {
        self.macos_sandbox.get_resource_usage()
    }

    pub fn check_oom_killed(&self) -> CapsuleResult<bool> {
        self.macos_sandbox.check_oom_killed()
    }

    pub fn cleanup(&self) -> CapsuleResult<()> {
        self.macos_sandbox.cleanup()
    }

    /// Prepare a command for execution with macOS sandbox restrictions
    pub fn prepare_command(&self, cmd: &mut std::process::Command) -> CapsuleResult<()> {
        self.macos_sandbox.prepare_command(cmd)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
            "Sandbox functionality is only available on Linux and macOS".to_string(),
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

    #[allow(dead_code)]
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

#[cfg(target_os = "macos")]
impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
