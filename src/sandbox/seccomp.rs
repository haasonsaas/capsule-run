use crate::error::{CapsuleResult, SandboxError};
use libseccomp::{ScmpAction, ScmpArgCompare, ScmpCompareOp, ScmpFilterContext, ScmpSyscall};
use std::sync::{Arc, Mutex};

// Wrapper to make ScmpFilterContext thread-safe
struct ThreadSafeFilterContext {
    inner: ScmpFilterContext,
}

// SAFETY: We ensure thread safety by using a Mutex around operations
unsafe impl Send for ThreadSafeFilterContext {}
unsafe impl Sync for ThreadSafeFilterContext {}

pub struct SeccompFilter {
    ctx: Arc<Mutex<ThreadSafeFilterContext>>,
}

impl SeccompFilter {
    pub fn new() -> CapsuleResult<Self> {
        let ctx = ScmpFilterContext::new_filter(ScmpAction::KillProcess).map_err(|e| {
            SandboxError::SeccompSetup(format!("Failed to create seccomp context: {}", e))
        })?;

        Ok(Self {
            ctx: Arc::new(Mutex::new(ThreadSafeFilterContext { inner: ctx })),
        })
    }

    pub fn setup_allowlist(&mut self) -> CapsuleResult<()> {
        let mut ctx = self.ctx.lock().unwrap();

        // Define a minimal set of allowed syscalls for sandboxed execution
        let mut allowed_syscalls = Vec::new();

        // Essential I/O operations (8 syscalls)
        allowed_syscalls.extend_from_slice(&[
            libc::SYS_read,
            libc::SYS_write,
            libc::SYS_readv,
            libc::SYS_writev,
            libc::SYS_close,
            libc::SYS_lseek,
            libc::SYS_dup,
            libc::SYS_dup3,
        ]);

        // Minimal file operations (12 syscalls) - modern syscalls only
        allowed_syscalls.extend_from_slice(&[
            libc::SYS_openat,
            libc::SYS_fstat,
            libc::SYS_newfstatat,
            libc::SYS_getcwd,
            libc::SYS_chdir,
            libc::SYS_mkdirat,
            libc::SYS_unlinkat,
            libc::SYS_renameat2,
            libc::SYS_fchmod,
            libc::SYS_ftruncate,
            libc::SYS_fsync,
            libc::SYS_pipe2,
        ]);

        // Directory operations (1 syscall)
        allowed_syscalls.push(libc::SYS_getdents64);

        // Essential memory management (5 syscalls)
        allowed_syscalls.extend_from_slice(&[
            libc::SYS_mmap,
            libc::SYS_munmap,
            libc::SYS_mprotect,
            libc::SYS_madvise,
            libc::SYS_brk,
        ]);

        // Minimal process/thread info (5 syscalls)
        allowed_syscalls.extend_from_slice(&[
            libc::SYS_getpid,
            libc::SYS_getuid,
            libc::SYS_getgid,
            libc::SYS_gettid,
            libc::SYS_set_tid_address,
        ]);

        // Time operations (2 syscalls)
        allowed_syscalls.extend_from_slice(&[libc::SYS_clock_gettime, libc::SYS_nanosleep]);

        // Essential signal handling (4 syscalls)
        allowed_syscalls.extend_from_slice(&[
            libc::SYS_rt_sigaction,
            libc::SYS_rt_sigprocmask,
            libc::SYS_rt_sigreturn,
            libc::SYS_sigaltstack,
        ]);

        // Process execution and control (4 syscalls)
        allowed_syscalls.extend_from_slice(&[
            libc::SYS_execve,
            libc::SYS_wait4,
            libc::SYS_exit,
            libc::SYS_exit_group,
        ]);

        // Essential polling (3 syscalls)
        allowed_syscalls.extend_from_slice(&[
            libc::SYS_ppoll,
            libc::SYS_epoll_create1,
            libc::SYS_epoll_pwait,
        ]);

        // Resource limits (2 syscalls)
        allowed_syscalls.extend_from_slice(&[libc::SYS_prlimit64, libc::SYS_getrlimit]);

        // Thread synchronization (1 syscall)
        allowed_syscalls.push(libc::SYS_futex);

        // fcntl for file descriptor operations (1 syscall)
        allowed_syscalls.push(libc::SYS_fcntl);

        // Additional essential syscalls for compatibility (6 syscalls)
        allowed_syscalls.extend_from_slice(&[
            libc::SYS_ioctl,       // Terminal operations
            libc::SYS_getrandom,   // Secure random numbers
            libc::SYS_sched_yield, // Thread yielding
            libc::SYS_kill,        // Send signals to own process
            libc::SYS_tgkill,      // Thread-targeted signals
            libc::SYS_geteuid,     // Get effective UID
        ]);

        // Total: ~55 syscalls - a reasonable balance between security and functionality

        for &syscall in &allowed_syscalls {
            ctx.inner
                .add_rule(ScmpAction::Allow, ScmpSyscall::from(syscall as i32))
                .map_err(|e| {
                    SandboxError::SeccompSetup(format!(
                        "Failed to add syscall rule for {}: {}",
                        syscall, e
                    ))
                })?;
        }

        // Add conditional rules for more dangerous syscalls
        Self::add_conditional_rules(&mut ctx)?;

        Ok(())
    }

    fn add_conditional_rules(
        ctx: &mut std::sync::MutexGuard<ThreadSafeFilterContext>,
    ) -> CapsuleResult<()> {
        // Allow clone only for thread creation (CLONE_THREAD flag)
        ctx.inner
            .add_rule_conditional(
                ScmpAction::Allow,
                ScmpSyscall::from(libc::SYS_clone as i32),
                &[ScmpArgCompare::new(
                    0,
                    ScmpCompareOp::MaskedEqual(libc::CLONE_THREAD as u64),
                    libc::CLONE_THREAD as u64,
                )],
            )
            .map_err(|e| SandboxError::SeccompSetup(format!("Failed to add clone rule: {}", e)))?;

        // Allow prctl for specific operations only
        // PR_SET_NAME (15) - allow setting thread name
        ctx.inner
            .add_rule_conditional(
                ScmpAction::Allow,
                ScmpSyscall::from(libc::SYS_prctl as i32),
                &[ScmpArgCompare::new(0, ScmpCompareOp::Equal, 15)],
            )
            .map_err(|e| {
                SandboxError::SeccompSetup(format!("Failed to add prctl PR_SET_NAME rule: {}", e))
            })?;

        // PR_GET_NAME (16) - allow getting thread name
        ctx.inner
            .add_rule_conditional(
                ScmpAction::Allow,
                ScmpSyscall::from(libc::SYS_prctl as i32),
                &[ScmpArgCompare::new(0, ScmpCompareOp::Equal, 16)],
            )
            .map_err(|e| {
                SandboxError::SeccompSetup(format!("Failed to add prctl PR_GET_NAME rule: {}", e))
            })?;

        // Allow socket operations only for AF_UNIX
        ctx.inner
            .add_rule_conditional(
                ScmpAction::Allow,
                ScmpSyscall::from(libc::SYS_socket as i32),
                &[ScmpArgCompare::new(
                    0,
                    ScmpCompareOp::Equal,
                    libc::AF_UNIX as u64,
                )],
            )
            .map_err(|e| {
                SandboxError::SeccompSetup(format!("Failed to add socket AF_UNIX rule: {}", e))
            })?;

        Ok(())
    }

    pub fn apply(&self) -> CapsuleResult<()> {
        let ctx = self.ctx.lock().unwrap();
        ctx.inner.load().map_err(|e| {
            SandboxError::SeccompSetup(format!("Failed to load seccomp filter: {}", e))
        })?;

        Ok(())
    }

    pub fn with_network_access(self) -> CapsuleResult<Self> {
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

        {
            let mut ctx = self.ctx.lock().unwrap();
            for &syscall in &network_syscalls {
                ctx.inner
                    .add_rule(ScmpAction::Allow, ScmpSyscall::from(syscall as i32))
                    .map_err(|e| {
                        SandboxError::SeccompSetup(format!(
                            "Failed to add network syscall rule for {}: {}",
                            syscall, e
                        ))
                    })?;
            }
        }

        Ok(self)
    }
}

impl Default for SeccompFilter {
    fn default() -> Self {
        Self::new().expect("Failed to create default seccomp filter")
    }
}
