pub mod namespaces;
pub mod cgroups;
pub mod seccomp;
pub mod filesystem;

use crate::api::schema::{ResourceLimits, IsolationConfig};
use crate::error::{CapsuleResult, SandboxError};
use uuid::Uuid;

pub use namespaces::NamespaceManager;
pub use cgroups::{CgroupManager, ResourceUsage};
pub use seccomp::SeccompFilter;
pub use filesystem::FilesystemManager;

pub struct Sandbox {
    pub execution_id: Uuid,
    pub namespace_manager: NamespaceManager,
    pub cgroup_manager: CgroupManager,
    pub filesystem_manager: FilesystemManager,
    pub seccomp_filter: SeccompFilter,
}

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

    pub fn setup(&mut self, resources: &ResourceLimits, isolation: &IsolationConfig) -> CapsuleResult<()> {
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
        use caps::{CapSet, clear};

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

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}