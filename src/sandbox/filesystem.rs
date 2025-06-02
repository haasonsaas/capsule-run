use crate::api::schema::{BindMount, IsolationConfig};
use crate::error::{CapsuleResult, SandboxError};
use nix::mount::{mount, umount2, MntFlags, MsFlags};
use nix::unistd::{chdir, pivot_root};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct FilesystemManager {
    root_path: PathBuf,
    old_root_path: PathBuf,
    execution_id: Uuid,
}

impl FilesystemManager {
    pub fn new(execution_id: Uuid) -> CapsuleResult<Self> {
        let root_path = PathBuf::from("/tmp").join(format!("capsule-{}", execution_id));
        let old_root_path = root_path.join("old_root");

        Ok(Self {
            root_path,
            old_root_path,
            execution_id,
        })
    }

    pub fn setup_isolation(&self, config: &IsolationConfig) -> CapsuleResult<()> {
        self.create_root_filesystem()?;
        self.setup_essential_mounts()?;
        self.setup_readonly_paths(&config.readonly_paths)?;
        self.setup_writable_paths(&config.writable_paths)?;
        self.setup_bind_mounts(&config.bind_mounts)?;
        self.perform_pivot_root()?;
        self.setup_working_directory(&config.working_directory)?;
        self.cleanup_old_root()?;

        Ok(())
    }

    fn create_root_filesystem(&self) -> CapsuleResult<()> {
        fs::create_dir_all(&self.root_path).map_err(|e| {
            SandboxError::FilesystemSetup(format!(
                "Failed to create root directory {}: {}",
                self.root_path.display(),
                e
            ))
        })?;

        fs::create_dir_all(&self.old_root_path).map_err(|e| {
            SandboxError::FilesystemSetup(format!(
                "Failed to create old_root directory {}: {}",
                self.old_root_path.display(),
                e
            ))
        })?;

        let essential_dirs = [
            "bin",
            "sbin",
            "usr",
            "lib",
            "lib64",
            "etc",
            "dev",
            "proc",
            "sys",
            "tmp",
            "var",
            "workspace",
        ];

        for dir in &essential_dirs {
            let dir_path = self.root_path.join(dir);
            fs::create_dir_all(&dir_path).map_err(|e| {
                SandboxError::FilesystemSetup(format!(
                    "Failed to create directory {}: {}",
                    dir_path.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }

    fn setup_essential_mounts(&self) -> CapsuleResult<()> {
        // Mount essential system directories as read-only
        let readonly_mounts = [
            ("/bin", "bin"),
            ("/sbin", "sbin"),
            ("/usr", "usr"),
            ("/lib", "lib"),
            ("/lib64", "lib64"),
            ("/etc", "etc"),
        ];

        for (source, target) in &readonly_mounts {
            if Path::new(source).exists() {
                let target_path = self.root_path.join(target);
                self.bind_mount_readonly(source, &target_path)?;
            }
        }

        // Mount /dev with device nodes
        self.setup_dev_filesystem()?;

        // Mount /proc with restricted access
        let proc_path = self.root_path.join("proc");
        mount(
            Some("proc"),
            &proc_path,
            Some("proc"),
            MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
            Some("hidepid=2,gid=proc"),
        )
        .map_err(|e| SandboxError::FilesystemSetup(format!("Failed to mount /proc: {}", e)))?;

        // Mount /sys as read-only
        let sys_path = self.root_path.join("sys");
        mount(
            Some("sysfs"),
            &sys_path,
            Some("sysfs"),
            MsFlags::MS_RDONLY | MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
            None::<&str>,
        )
        .map_err(|e| SandboxError::FilesystemSetup(format!("Failed to mount /sys: {}", e)))?;

        // Mount /tmp as tmpfs
        let tmp_path = self.root_path.join("tmp");
        mount(
            Some("tmpfs"),
            &tmp_path,
            Some("tmpfs"),
            MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
            Some("size=64M,mode=1777"),
        )
        .map_err(|e| SandboxError::FilesystemSetup(format!("Failed to mount /tmp: {}", e)))?;

        // Mount /var as tmpfs
        let var_path = self.root_path.join("var");
        mount(
            Some("tmpfs"),
            &var_path,
            Some("tmpfs"),
            MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
            Some("size=32M,mode=755"),
        )
        .map_err(|e| SandboxError::FilesystemSetup(format!("Failed to mount /var: {}", e)))?;

        Ok(())
    }

    fn setup_dev_filesystem(&self) -> CapsuleResult<()> {
        let dev_path = self.root_path.join("dev");

        // Mount /dev as tmpfs
        mount(
            Some("tmpfs"),
            &dev_path,
            Some("tmpfs"),
            MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
            Some("size=5M,mode=755"),
        )
        .map_err(|e| SandboxError::FilesystemSetup(format!("Failed to mount /dev: {}", e)))?;

        // Create essential device nodes
        let essential_devices = [
            ("null", 1, 3),
            ("zero", 1, 5),
            ("full", 1, 7),
            ("random", 1, 8),
            ("urandom", 1, 9),
        ];

        for (name, major, minor) in &essential_devices {
            let device_path = dev_path.join(name);
            let device_number = nix::sys::stat::makedev(*major, *minor);

            nix::unistd::mknod(
                &device_path,
                nix::sys::stat::SFlag::S_IFCHR,
                nix::sys::stat::Mode::S_IRUSR
                    | nix::sys::stat::Mode::S_IWUSR
                    | nix::sys::stat::Mode::S_IRGRP
                    | nix::sys::stat::Mode::S_IROTH,
                device_number,
            )
            .map_err(|e| {
                SandboxError::FilesystemSetup(format!("Failed to create device {}: {}", name, e))
            })?;
        }

        // Create stdin, stdout, stderr symlinks
        let stdio_links = [("stdin", "0"), ("stdout", "1"), ("stderr", "2")];

        for (link_name, target) in &stdio_links {
            let link_path = dev_path.join(link_name);
            let target_path = format!("/proc/self/fd/{}", target);

            std::os::unix::fs::symlink(&target_path, &link_path).map_err(|e| {
                SandboxError::FilesystemSetup(format!(
                    "Failed to create {} symlink: {}",
                    link_name, e
                ))
            })?;
        }

        Ok(())
    }

    fn setup_readonly_paths(&self, readonly_paths: &[String]) -> CapsuleResult<()> {
        for path in readonly_paths {
            let source = Path::new(path);
            if source.exists() {
                let target = self.root_path.join(path.strip_prefix('/').unwrap_or(path));
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        SandboxError::FilesystemSetup(format!(
                            "Failed to create parent directory for {}: {}",
                            target.display(),
                            e
                        ))
                    })?;
                }
                self.bind_mount_readonly(source, &target)?;
            }
        }
        Ok(())
    }

    fn setup_writable_paths(&self, writable_paths: &[String]) -> CapsuleResult<()> {
        for path in writable_paths {
            let source = Path::new(path);
            if source.exists() {
                let target = self.root_path.join(path.strip_prefix('/').unwrap_or(path));
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        SandboxError::FilesystemSetup(format!(
                            "Failed to create parent directory for {}: {}",
                            target.display(),
                            e
                        ))
                    })?;
                }
                self.bind_mount_writable(source, &target)?;
            }
        }
        Ok(())
    }

    fn setup_bind_mounts(&self, bind_mounts: &[BindMount]) -> CapsuleResult<()> {
        for bind_mount in bind_mounts {
            let source = Path::new(&bind_mount.source);
            let target = self.root_path.join(
                bind_mount
                    .destination
                    .strip_prefix('/')
                    .unwrap_or(&bind_mount.destination),
            );

            if source.exists() {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        SandboxError::FilesystemSetup(format!(
                            "Failed to create parent directory for bind mount {}: {}",
                            target.display(),
                            e
                        ))
                    })?;
                }

                if bind_mount.readonly {
                    self.bind_mount_readonly(source, &target)?;
                } else {
                    self.bind_mount_writable(source, &target)?;
                }
            }
        }
        Ok(())
    }

    fn bind_mount_readonly(&self, source: &Path, target: &Path) -> CapsuleResult<()> {
        // Create target if it doesn't exist
        if source.is_dir() {
            fs::create_dir_all(target).map_err(|e| {
                SandboxError::FilesystemSetup(format!(
                    "Failed to create target directory {}: {}",
                    target.display(),
                    e
                ))
            })?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    SandboxError::FilesystemSetup(format!(
                        "Failed to create parent directory for {}: {}",
                        target.display(),
                        e
                    ))
                })?;
            }
            fs::File::create(target).map_err(|e| {
                SandboxError::FilesystemSetup(format!(
                    "Failed to create target file {}: {}",
                    target.display(),
                    e
                ))
            })?;
        }

        // Bind mount
        mount(
            Some(source),
            target,
            None::<&str>,
            MsFlags::MS_BIND,
            None::<&str>,
        )
        .map_err(|e| {
            SandboxError::FilesystemSetup(format!(
                "Failed to bind mount {} to {}: {}",
                source.display(),
                target.display(),
                e
            ))
        })?;

        // Remount as readonly
        mount(
            None::<&str>,
            target,
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REMOUNT | MsFlags::MS_RDONLY,
            None::<&str>,
        )
        .map_err(|e| {
            SandboxError::FilesystemSetup(format!(
                "Failed to remount {} as readonly: {}",
                target.display(),
                e
            ))
        })?;

        Ok(())
    }

    fn bind_mount_writable(&self, source: &Path, target: &Path) -> CapsuleResult<()> {
        // Create target if it doesn't exist
        if source.is_dir() {
            fs::create_dir_all(target).map_err(|e| {
                SandboxError::FilesystemSetup(format!(
                    "Failed to create target directory {}: {}",
                    target.display(),
                    e
                ))
            })?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    SandboxError::FilesystemSetup(format!(
                        "Failed to create parent directory for {}: {}",
                        target.display(),
                        e
                    ))
                })?;
            }
            fs::File::create(target).map_err(|e| {
                SandboxError::FilesystemSetup(format!(
                    "Failed to create target file {}: {}",
                    target.display(),
                    e
                ))
            })?;
        }

        // Bind mount as writable
        mount(
            Some(source),
            target,
            None::<&str>,
            MsFlags::MS_BIND,
            None::<&str>,
        )
        .map_err(|e| {
            SandboxError::FilesystemSetup(format!(
                "Failed to bind mount {} to {}: {}",
                source.display(),
                target.display(),
                e
            ))
        })?;

        Ok(())
    }

    fn perform_pivot_root(&self) -> CapsuleResult<()> {
        pivot_root(&self.root_path, &self.old_root_path)
            .map_err(|e| SandboxError::FilesystemSetup(format!("Failed to pivot root: {}", e)))?;

        // Change to new root
        chdir("/").map_err(|e| {
            SandboxError::FilesystemSetup(format!("Failed to chdir to new root: {}", e))
        })?;

        Ok(())
    }

    fn setup_working_directory(&self, working_dir: &str) -> CapsuleResult<()> {
        let working_path = Path::new(working_dir);

        // Create working directory if it doesn't exist
        if !working_path.exists() {
            fs::create_dir_all(working_path).map_err(|e| {
                SandboxError::FilesystemSetup(format!(
                    "Failed to create working directory {}: {}",
                    working_dir, e
                ))
            })?;
        }

        // Change to working directory
        chdir(working_path).map_err(|e| {
            SandboxError::FilesystemSetup(format!(
                "Failed to change to working directory {}: {}",
                working_dir, e
            ))
        })?;

        Ok(())
    }

    fn cleanup_old_root(&self) -> CapsuleResult<()> {
        // Unmount old root
        umount2("/old_root", MntFlags::MNT_DETACH).map_err(|e| {
            SandboxError::FilesystemSetup(format!("Failed to unmount old root: {}", e))
        })?;

        // Remove old root directory
        fs::remove_dir("/old_root").map_err(|e| {
            SandboxError::FilesystemSetup(format!("Failed to remove old root directory: {}", e))
        })?;

        Ok(())
    }

    pub fn cleanup(&self) -> CapsuleResult<()> {
        if self.root_path.exists() {
            fs::remove_dir_all(&self.root_path).map_err(|e| {
                SandboxError::FilesystemSetup(format!(
                    "Failed to cleanup filesystem {}: {}",
                    self.root_path.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }
}

impl Drop for FilesystemManager {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_manager_creation() {
        let execution_id = Uuid::new_v4();
        let manager = FilesystemManager::new(execution_id);
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert!(manager
            .root_path
            .to_string_lossy()
            .contains(&execution_id.to_string()));
    }

    #[test]
    fn test_essential_directories() {
        let execution_id = Uuid::new_v4();
        let manager = FilesystemManager::new(execution_id).unwrap();

        // This test would require root privileges to actually create the filesystem
        // In a real test environment, we'd check the directory structure
        assert!(manager.root_path.is_absolute());
        assert!(manager.old_root_path.is_absolute());
    }
}
