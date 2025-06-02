use crate::error::{CapsuleResult, SandboxError};
use libseccomp::{ScmpAction, ScmpFilterContext, ScmpSyscall};

pub struct SeccompFilter {
    ctx: ScmpFilterContext,
}

impl SeccompFilter {
    pub fn new() -> CapsuleResult<Self> {
        let ctx = ScmpFilterContext::new_filter(ScmpAction::Kill).map_err(|e| {
            SandboxError::SeccompSetup(format!("Failed to create seccomp context: {}", e))
        })?;

        Ok(Self { ctx })
    }

    pub fn setup_allowlist(&mut self) -> CapsuleResult<()> {
        let allowed_syscalls = [
            // Essential I/O operations
            libc::SYS_read,
            libc::SYS_write,
            libc::SYS_readv,
            libc::SYS_writev,
            libc::SYS_pread64,
            libc::SYS_pwrite64,
            libc::SYS_close,
            libc::SYS_lseek,
            // File operations
            libc::SYS_open,
            libc::SYS_openat,
            libc::SYS_creat,
            libc::SYS_access,
            libc::SYS_faccessat,
            libc::SYS_stat,
            libc::SYS_fstat,
            libc::SYS_lstat,
            libc::SYS_fstatat,
            libc::SYS_newfstatat,
            libc::SYS_readlink,
            libc::SYS_readlinkat,
            libc::SYS_getcwd,
            libc::SYS_chdir,
            libc::SYS_fchdir,
            libc::SYS_mkdir,
            libc::SYS_mkdirat,
            libc::SYS_rmdir,
            libc::SYS_unlink,
            libc::SYS_unlinkat,
            libc::SYS_rename,
            libc::SYS_renameat,
            libc::SYS_renameat2,
            libc::SYS_link,
            libc::SYS_linkat,
            libc::SYS_symlink,
            libc::SYS_symlinkat,
            libc::SYS_chmod,
            libc::SYS_fchmod,
            libc::SYS_fchmodat,
            libc::SYS_chown,
            libc::SYS_fchown,
            libc::SYS_lchown,
            libc::SYS_fchownat,
            libc::SYS_truncate,
            libc::SYS_ftruncate,
            libc::SYS_fallocate,
            libc::SYS_fsync,
            libc::SYS_fdatasync,
            libc::SYS_sync,
            libc::SYS_syncfs,
            // Memory management
            libc::SYS_mmap,
            libc::SYS_munmap,
            libc::SYS_mprotect,
            libc::SYS_madvise,
            libc::SYS_brk,
            libc::SYS_mremap,
            libc::SYS_msync,
            libc::SYS_mlock,
            libc::SYS_munlock,
            libc::SYS_mlockall,
            libc::SYS_munlockall,
            // Process management
            libc::SYS_exit,
            libc::SYS_exit_group,
            libc::SYS_getpid,
            libc::SYS_getppid,
            libc::SYS_getpgid,
            libc::SYS_setpgid,
            libc::SYS_getsid,
            libc::SYS_setsid,
            libc::SYS_getuid,
            libc::SYS_geteuid,
            libc::SYS_getgid,
            libc::SYS_getegid,
            libc::SYS_getgroups,
            libc::SYS_wait4,
            libc::SYS_waitid,
            // Signal handling
            libc::SYS_rt_sigaction,
            libc::SYS_rt_sigprocmask,
            libc::SYS_rt_sigreturn,
            libc::SYS_rt_sigsuspend,
            libc::SYS_rt_sigpending,
            libc::SYS_rt_sigtimedwait,
            libc::SYS_rt_sigqueueinfo,
            libc::SYS_kill,
            libc::SYS_tkill,
            libc::SYS_tgkill,
            libc::SYS_alarm,
            // Time operations
            libc::SYS_time,
            libc::SYS_clock_gettime,
            libc::SYS_clock_getres,
            libc::SYS_gettimeofday,
            libc::SYS_nanosleep,
            libc::SYS_clock_nanosleep,
            // I/O multiplexing
            libc::SYS_select,
            libc::SYS_pselect6,
            libc::SYS_poll,
            libc::SYS_ppoll,
            libc::SYS_epoll_create,
            libc::SYS_epoll_create1,
            libc::SYS_epoll_ctl,
            libc::SYS_epoll_wait,
            libc::SYS_epoll_pwait,
            // Pipes and FIFOs
            libc::SYS_pipe,
            libc::SYS_pipe2,
            libc::SYS_dup,
            libc::SYS_dup2,
            libc::SYS_dup3,
            // Directory operations
            libc::SYS_getdents,
            libc::SYS_getdents64,
            // Resource limits
            libc::SYS_getrlimit,
            libc::SYS_setrlimit,
            libc::SYS_prlimit64,
            libc::SYS_getrusage,
            // Thread operations (limited)
            libc::SYS_futex,
            libc::SYS_set_thread_area,
            libc::SYS_get_thread_area,
            libc::SYS_set_tid_address,
            libc::SYS_gettid,
            // Architecture-specific
            libc::SYS_arch_prctl,
            // Filesystem info
            libc::SYS_statfs,
            libc::SYS_fstatfs,
            libc::SYS_statvfs,
            libc::SYS_fstatvfs,
            // fcntl operations
            libc::SYS_fcntl,
            // ioctl (restricted)
            libc::SYS_ioctl,
        ];

        for &syscall in &allowed_syscalls {
            self.ctx
                .add_rule(ScmpAction::Allow, ScmpSyscall::new(syscall as i32))
                .map_err(|e| {
                    SandboxError::SeccompSetup(format!(
                        "Failed to add syscall rule for {}: {}",
                        syscall, e
                    ))
                })?;
        }

        // Add conditional rules for more dangerous syscalls
        self.add_conditional_rules()?;

        Ok(())
    }

    fn add_conditional_rules(&mut self) -> CapsuleResult<()> {
        use libseccomp::{ScmpArgCompare, ScmpCompareOp};

        // Allow clone only for thread creation (CLONE_THREAD flag)
        self.ctx
            .add_rule_conditional(
                ScmpAction::Allow,
                ScmpSyscall::new(libc::SYS_clone as i32),
                &[ScmpArgCompare::new(
                    0,
                    ScmpCompareOp::MaskedEqual,
                    libc::CLONE_THREAD as u64,
                    libc::CLONE_THREAD as u64,
                )],
            )
            .map_err(|e| SandboxError::SeccompSetup(format!("Failed to add clone rule: {}", e)))?;

        // Allow prctl for specific operations only
        // PR_SET_NAME (15) - allow setting thread name
        self.ctx
            .add_rule_conditional(
                ScmpAction::Allow,
                ScmpSyscall::new(libc::SYS_prctl as i32),
                &[ScmpArgCompare::new(0, ScmpCompareOp::Equal, 15, 0)],
            )
            .map_err(|e| {
                SandboxError::SeccompSetup(format!("Failed to add prctl PR_SET_NAME rule: {}", e))
            })?;

        // PR_GET_NAME (16) - allow getting thread name
        self.ctx
            .add_rule_conditional(
                ScmpAction::Allow,
                ScmpSyscall::new(libc::SYS_prctl as i32),
                &[ScmpArgCompare::new(0, ScmpCompareOp::Equal, 16, 0)],
            )
            .map_err(|e| {
                SandboxError::SeccompSetup(format!("Failed to add prctl PR_GET_NAME rule: {}", e))
            })?;

        // Allow socket operations only for AF_UNIX
        self.ctx
            .add_rule_conditional(
                ScmpAction::Allow,
                ScmpSyscall::new(libc::SYS_socket as i32),
                &[ScmpArgCompare::new(
                    0,
                    ScmpCompareOp::Equal,
                    libc::AF_UNIX as u64,
                    0,
                )],
            )
            .map_err(|e| {
                SandboxError::SeccompSetup(format!("Failed to add socket AF_UNIX rule: {}", e))
            })?;

        Ok(())
    }

    pub fn apply(&self) -> CapsuleResult<()> {
        self.ctx.load().map_err(|e| {
            SandboxError::SeccompSetup(format!("Failed to load seccomp filter: {}", e))
        })?;

        Ok(())
    }

    pub fn with_network_access(mut self) -> CapsuleResult<Self> {
        // Add network-related syscalls when network access is enabled
        let network_syscalls = [
            libc::SYS_socket,
            libc::SYS_bind,
            libc::SYS_listen,
            libc::SYS_accept,
            libc::SYS_accept4,
            libc::SYS_connect,
            libc::SYS_getsockname,
            libc::SYS_getpeername,
            libc::SYS_sendto,
            libc::SYS_recvfrom,
            libc::SYS_sendmsg,
            libc::SYS_recvmsg,
            libc::SYS_shutdown,
            libc::SYS_setsockopt,
            libc::SYS_getsockopt,
        ];

        for &syscall in &network_syscalls {
            self.ctx
                .add_rule(ScmpAction::Allow, ScmpSyscall::new(syscall as i32))
                .map_err(|e| {
                    SandboxError::SeccompSetup(format!(
                        "Failed to add network syscall rule for {}: {}",
                        syscall, e
                    ))
                })?;
        }

        Ok(self)
    }
}

impl Default for SeccompFilter {
    fn default() -> Self {
        Self::new().expect("Failed to create default seccomp filter")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seccomp_filter_creation() {
        let result = SeccompFilter::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_seccomp_allowlist_setup() {
        let mut filter = SeccompFilter::new().unwrap();
        let result = filter.setup_allowlist();
        assert!(result.is_ok());
    }

    #[test]
    fn test_seccomp_with_network() {
        let filter = SeccompFilter::new().unwrap();
        let result = filter.with_network_access();
        assert!(result.is_ok());
    }
}
