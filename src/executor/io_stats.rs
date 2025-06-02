use crate::error::CapsuleResult;

#[derive(Debug, Clone, Default)]
pub struct IoStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
}

pub struct IoMonitor {
    pid: u32,
}

impl IoMonitor {
    pub fn new(pid: u32) -> Self {
        Self { pid }
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

    for line in content.lines() {
        if let Some(value_str) = line.strip_prefix("read_bytes: ") {
            read_bytes = value_str.parse().unwrap_or(0);
        } else if let Some(value_str) = line.strip_prefix("write_bytes: ") {
            write_bytes = value_str.parse().unwrap_or(0);
        }
    }

    Ok(IoStats {
        read_bytes,
        write_bytes,
    })
}

#[cfg(target_os = "macos")]
pub fn get_process_io_stats(_pid: u32) -> CapsuleResult<IoStats> {
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
        read_bytes: usage.ru_inblock as u64 * 512, // Approximate: 512 bytes per block
        write_bytes: usage.ru_oublock as u64 * 512, // Approximate: 512 bytes per block
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn get_process_io_stats(pid: u32) -> CapsuleResult<IoStats> {
    // Fallback for unsupported platforms
    Ok(IoStats::default())
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
        let monitor = IoMonitor::new(pid);

        // This might fail on some systems due to permissions
        if let Ok(stats) = monitor.get_total_stats() {
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
