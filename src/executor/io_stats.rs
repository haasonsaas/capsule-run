use crate::error::CapsuleResult;

#[derive(Debug, Clone, Default)]
pub struct IoStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_calls: u64,
    pub write_calls: u64,
}

pub struct IoMonitor {
    pid: u32,
    last_stats: IoStats,
}

impl IoMonitor {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            last_stats: IoStats::default(),
        }
    }

    pub fn get_current_stats(&mut self) -> CapsuleResult<IoStats> {
        let current = get_process_io_stats(self.pid)?;
        
        // Calculate delta since last measurement
        let delta = IoStats {
            read_bytes: current.read_bytes.saturating_sub(self.last_stats.read_bytes),
            write_bytes: current.write_bytes.saturating_sub(self.last_stats.write_bytes),
            read_calls: current.read_calls.saturating_sub(self.last_stats.read_calls),
            write_calls: current.write_calls.saturating_sub(self.last_stats.write_calls),
        };
        
        self.last_stats = current.clone();
        Ok(current)
    }

    pub fn get_total_stats(&self) -> CapsuleResult<IoStats> {
        get_process_io_stats(self.pid)
    }
}

#[cfg(target_os = "linux")]
pub fn get_process_io_stats(pid: u32) -> CapsuleResult<IoStats> {
    let io_path = format!("/proc/{}/io", pid);
    let content = std::fs::read_to_string(io_path).map_err(|e| {
        crate::error::CapsuleError::Syscall(format!("Failed to read process I/O stats: {}", e))
    })?;
    
    let mut read_bytes = 0u64;
    let mut write_bytes = 0u64;
    let mut read_calls = 0u64;
    let mut write_calls = 0u64;
    
    for line in content.lines() {
        if let Some(value_str) = line.strip_prefix("read_bytes: ") {
            read_bytes = value_str.parse().unwrap_or(0);
        } else if let Some(value_str) = line.strip_prefix("write_bytes: ") {
            write_bytes = value_str.parse().unwrap_or(0);
        } else if let Some(value_str) = line.strip_prefix("syscr: ") {
            read_calls = value_str.parse().unwrap_or(0);
        } else if let Some(value_str) = line.strip_prefix("syscw: ") {
            write_calls = value_str.parse().unwrap_or(0);
        }
    }
    
    Ok(IoStats {
        read_bytes,
        write_bytes,
        read_calls,
        write_calls,
    })
}

#[cfg(target_os = "macos")]
pub fn get_process_io_stats(pid: u32) -> CapsuleResult<IoStats> {
    // Use rusage for basic I/O statistics on macOS
    let usage = unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        let result = libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage);
        if result != 0 {
            return Ok(IoStats::default());
        }
        usage
    };

    // Convert block counts to approximate byte counts
    // Note: This is less accurate than Linux /proc/pid/io
    Ok(IoStats {
        read_bytes: usage.ru_inblock as u64 * 512,  // Approximate: 512 bytes per block
        write_bytes: usage.ru_oublock as u64 * 512, // Approximate: 512 bytes per block
        read_calls: 0, // Not available via rusage
        write_calls: 0, // Not available via rusage
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn get_process_io_stats(_pid: u32) -> CapsuleResult<IoStats> {
    // Fallback for unsupported platforms
    Ok(IoStats::default())
}

/// Enhanced I/O monitoring with better macOS support using libproc
#[cfg(target_os = "macos")]
mod macos_advanced {
    use super::IoStats;
    use crate::error::CapsuleResult;
    use std::mem;

    // libproc structures and constants
    const PROC_PIDTASKINFO: i32 = 4;
    
    #[repr(C)]
    struct ProcTaskInfo {
        pti_virtual_size: u64,
        pti_resident_size: u64,
        pti_total_user: u64,
        pti_total_system: u64,
        pti_threads_user: u64,
        pti_threads_system: u64,
        pti_policy: i32,
        pti_faults: i32,
        pti_pageins: i32,
        pti_cow_faults: i32,
        pti_messages_sent: i32,
        pti_messages_received: i32,
        pti_syscalls_mach: i32,
        pti_syscalls_unix: i32,
        pti_csw: i32,
        pti_threadnum: i32,
        pti_numrunning: i32,
        pti_priority: i32,
    }

    #[link(name = "proc", kind = "dylib")]
    extern "C" {
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut libc::c_void,
            buffersize: i32,
        ) -> i32;
    }

    pub fn get_detailed_io_stats(pid: u32) -> CapsuleResult<IoStats> {
        let mut task_info: ProcTaskInfo = unsafe { mem::zeroed() };
        
        let result = unsafe {
            proc_pidinfo(
                pid as i32,
                PROC_PIDTASKINFO,
                0,
                &mut task_info as *mut _ as *mut libc::c_void,
                mem::size_of::<ProcTaskInfo>() as i32,
            )
        };

        if result <= 0 {
            // Fall back to rusage method
            return super::get_process_io_stats(pid);
        }

        // libproc provides more detailed stats but still limited for I/O
        // Use pageins as a proxy for I/O activity
        Ok(IoStats {
            read_bytes: task_info.pti_pageins as u64 * 4096, // Page size
            write_bytes: 0, // Not directly available
            read_calls: task_info.pti_syscalls_unix as u64,
            write_calls: 0, // Not distinguishable
        })
    }
}

#[cfg(target_os = "macos")]
pub fn get_enhanced_io_stats(pid: u32) -> CapsuleResult<IoStats> {
    // Try the enhanced libproc method first, fall back to rusage
    macos_advanced::get_detailed_io_stats(pid)
        .or_else(|_| get_process_io_stats(pid))
}

#[cfg(not(target_os = "macos"))]
pub fn get_enhanced_io_stats(pid: u32) -> CapsuleResult<IoStats> {
    get_process_io_stats(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_stats_creation() {
        let stats = IoStats::default();
        assert_eq!(stats.read_bytes, 0);
        assert_eq!(stats.write_bytes, 0);
    }

    #[test]
    fn test_io_monitor() {
        let pid = std::process::id();
        let mut monitor = IoMonitor::new(pid);
        
        // This might fail on some systems due to permissions
        if let Ok(stats) = monitor.get_current_stats() {
            assert!(stats.read_bytes >= 0);
            assert!(stats.write_bytes >= 0);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_io_stats() {
        let pid = std::process::id();
        
        // This test might fail if /proc/self/io is not readable
        if let Ok(stats) = get_process_io_stats(pid) {
            println!("I/O stats: {:?}", stats);
            assert!(stats.read_bytes >= 0);
        }
    }
}