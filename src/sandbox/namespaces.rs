use crate::error::{CapsuleResult, SandboxError};
use nix::sched::{CloneFlags, unshare};
use nix::unistd::{getuid, getgid, Uid, Gid};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

pub struct NamespaceManager {
    uid: Uid,
    gid: Gid,
}

impl NamespaceManager {
    pub fn new() -> Self {
        Self {
            uid: getuid(),
            gid: getgid(),
        }
    }

    pub fn setup_namespaces(&self, enable_network: bool) -> CapsuleResult<()> {
        let mut flags = CloneFlags::CLONE_NEWUSER
            | CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWIPC
            | CloneFlags::CLONE_NEWUTS;

        if !enable_network {
            flags |= CloneFlags::CLONE_NEWNET;
        }

        unshare(flags).map_err(|e| {
            SandboxError::NamespaceCreation {
                namespace: format!("unshare failed: {}", e),
            }
        })?;

        self.setup_user_namespace()?;

        Ok(())
    }

    fn setup_user_namespace(&self) -> CapsuleResult<()> {
        let pid = std::process::id();
        
        self.write_uid_map(pid)?;
        self.write_gid_map(pid)?;

        Ok(())
    }

    fn write_uid_map(&self, pid: u32) -> CapsuleResult<()> {
        let uid_map_path = format!("/proc/{}/uid_map", pid);
        let uid_map_content = format!("0 {} 1\n", self.uid.as_raw());

        let mut file = OpenOptions::new()
            .write(true)
            .mode(0o644)
            .open(&uid_map_path)
            .map_err(|e| {
                SandboxError::UserMapping(format!("Failed to open uid_map: {}", e))
            })?;

        file.write_all(uid_map_content.as_bytes())
            .map_err(|e| {
                SandboxError::UserMapping(format!("Failed to write uid_map: {}", e))
            })?;

        Ok(())
    }

    fn write_gid_map(&self, pid: u32) -> CapsuleResult<()> {
        self.deny_setgroups(pid)?;

        let gid_map_path = format!("/proc/{}/gid_map", pid);
        let gid_map_content = format!("0 {} 1\n", self.gid.as_raw());

        let mut file = OpenOptions::new()
            .write(true)
            .mode(0o644)
            .open(&gid_map_path)
            .map_err(|e| {
                SandboxError::UserMapping(format!("Failed to open gid_map: {}", e))
            })?;

        file.write_all(gid_map_content.as_bytes())
            .map_err(|e| {
                SandboxError::UserMapping(format!("Failed to write gid_map: {}", e))
            })?;

        Ok(())
    }

    fn deny_setgroups(&self, pid: u32) -> CapsuleResult<()> {
        let setgroups_path = format!("/proc/{}/setgroups", pid);
        
        let mut file = OpenOptions::new()
            .write(true)
            .mode(0o644)
            .open(&setgroups_path)
            .map_err(|e| {
                SandboxError::UserMapping(format!("Failed to open setgroups: {}", e))
            })?;

        file.write_all(b"deny\n")
            .map_err(|e| {
                SandboxError::UserMapping(format!("Failed to deny setgroups: {}", e))
            })?;

        Ok(())
    }

    pub fn enter_namespaces() -> CapsuleResult<()> {
        use nix::sys::wait::{waitpid, WaitStatus};
        use nix::unistd::{fork, ForkResult, Pid};

        match unsafe { fork() }.map_err(|e| {
            SandboxError::NamespaceCreation {
                namespace: format!("fork failed: {}", e),
            }
        })? {
            ForkResult::Parent { child } => {
                match waitpid(child, None).map_err(|e| {
                    SandboxError::NamespaceCreation {
                        namespace: format!("waitpid failed: {}", e),
                    }
                })? {
                    WaitStatus::Exited(_, code) => {
                        std::process::exit(code);
                    }
                    WaitStatus::Signaled(_, signal, _) => {
                        std::process::exit(128 + signal as i32);
                    }
                    _ => {
                        std::process::exit(1);
                    }
                }
            }
            ForkResult::Child => {
                Self::setup_pid_namespace()?;
                Ok(())
            }
        }
    }

    fn setup_pid_namespace() -> CapsuleResult<()> {
        use nix::sys::signal::{self, Signal};
        use nix::unistd::Pid;
        use std::collections::HashMap;

        extern "C" fn handle_signal(_: libc::c_int) {
            unsafe {
                let mut status: libc::c_int = 0;
                while libc::waitpid(-1, &mut status, libc::WNOHANG) > 0 {}
            }
        }

        unsafe {
            signal::signal(Signal::SIGCHLD, signal::SigHandler::Handler(handle_signal))
                .map_err(|e| {
                    SandboxError::NamespaceCreation {
                        namespace: format!("signal handler setup failed: {}", e),
                    }
                })?;
        }

        Ok(())
    }
}

impl Default for NamespaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespace_manager_creation() {
        let manager = NamespaceManager::new();
        assert!(manager.uid.as_raw() >= 0);
        assert!(manager.gid.as_raw() >= 0);
    }

    #[test]
    fn test_user_namespace_files() {
        let pid = std::process::id();
        let uid_map_path = format!("/proc/{}/uid_map", pid);
        let gid_map_path = format!("/proc/{}/gid_map", pid);
        let setgroups_path = format!("/proc/{}/setgroups", pid);
        
        assert!(std::path::Path::new(&uid_map_path).exists());
        assert!(std::path::Path::new(&gid_map_path).exists());
        assert!(std::path::Path::new(&setgroups_path).exists());
    }
}