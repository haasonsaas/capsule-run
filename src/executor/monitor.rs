use crate::error::{CapsuleResult, ExecutionError};
use crate::sandbox::ResourceUsage;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc;

pub struct ResourceMonitor {
    stop_flag: Arc<AtomicBool>,
    monitor_handle: Option<thread::JoinHandle<CapsuleResult<MonitoringResult>>>,
    peak_usage: Arc<Mutex<ResourceUsage>>,
}

#[derive(Debug, Clone)]
pub struct MonitoringResult {
    pub peak_memory: u64,
    pub total_cpu_time: u64,
    pub wall_time: Duration,
    pub oom_killed: bool,
}

pub trait ResourceProvider: Send + Sync {
    fn get_usage(&self) -> CapsuleResult<ResourceUsage>;
    fn check_oom_killed(&self) -> CapsuleResult<bool>;
}

impl ResourceMonitor {
    pub fn new<P: ResourceProvider + 'static>(
        provider: Arc<P>,
        monitoring_interval: Duration,
    ) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let peak_usage = Arc::new(Mutex::new(ResourceUsage {
            memory_bytes: 0,
            cpu_time_us: 0,
            user_time_us: 0,
            kernel_time_us: 0,
            io_bytes_read: 0,
            io_bytes_written: 0,
        }));

        let monitor_handle = {
            let stop_flag = Arc::clone(&stop_flag);
            let peak_usage = Arc::clone(&peak_usage);
            let start_time = Instant::now();

            Some(thread::spawn(move || {
                Self::monitoring_loop(provider, stop_flag, peak_usage, monitoring_interval, start_time)
            }))
        };

        Self {
            stop_flag,
            monitor_handle,
            peak_usage,
        }
    }

    pub fn stop_and_get_result(mut self) -> CapsuleResult<MonitoringResult> {
        self.stop_flag.store(true, Ordering::Relaxed);

        if let Some(handle) = self.monitor_handle.take() {
            handle.join().map_err(|_| {
                ExecutionError::MonitoringError("Monitor thread panicked".to_string())
            })?
        } else {
            Err(ExecutionError::MonitoringError("Monitor not running".to_string()).into())
        }
    }

    pub fn get_current_usage(&self) -> CapsuleResult<ResourceUsage> {
        let peak_usage = self.peak_usage.lock().map_err(|_| {
            ExecutionError::MonitoringError("Failed to lock peak usage".to_string())
        })?;
        Ok(peak_usage.clone())
    }

    fn monitoring_loop<P: ResourceProvider>(
        provider: Arc<P>,
        stop_flag: Arc<AtomicBool>,
        peak_usage: Arc<Mutex<ResourceUsage>>,
        monitoring_interval: Duration,
        start_time: Instant,
    ) -> CapsuleResult<MonitoringResult> {
        let mut max_memory = 0u64;
        let mut final_cpu_time = 0u64;
        let mut oom_killed = false;

        while !stop_flag.load(Ordering::Relaxed) {
            match provider.get_usage() {
                Ok(usage) => {
                    if usage.memory_bytes > max_memory {
                        max_memory = usage.memory_bytes;
                    }
                    final_cpu_time = usage.cpu_time_us;

                    // Update peak usage
                    if let Ok(mut peak) = peak_usage.lock() {
                        if usage.memory_bytes > peak.memory_bytes {
                            peak.memory_bytes = usage.memory_bytes;
                        }
                        peak.cpu_time_us = usage.cpu_time_us;
                        peak.user_time_us = usage.user_time_us;
                        peak.kernel_time_us = usage.kernel_time_us;
                        if usage.io_bytes_read > peak.io_bytes_read {
                            peak.io_bytes_read = usage.io_bytes_read;
                        }
                        if usage.io_bytes_written > peak.io_bytes_written {
                            peak.io_bytes_written = usage.io_bytes_written;
                        }
                    }
                }
                Err(_) => {
                    // Continue monitoring even if we can't get usage
                }
            }

            // Check for OOM kills
            if let Ok(killed) = provider.check_oom_killed() {
                if killed {
                    oom_killed = true;
                    break;
                }
            }

            thread::sleep(monitoring_interval);
        }

        let wall_time = start_time.elapsed();

        Ok(MonitoringResult {
            peak_memory: max_memory,
            total_cpu_time: final_cpu_time,
            wall_time,
            oom_killed,
        })
    }
}

pub struct ProcessMonitor {
    pid: u32,
    stop_flag: Arc<AtomicBool>,
    monitor_handle: Option<thread::JoinHandle<CapsuleResult<ProcessStatus>>>,
}

#[derive(Debug, Clone)]
pub enum ProcessStatus {
    Running,
    Exited(i32),
    Signaled(i32),
    Stopped(i32),
    Unknown,
}

impl ProcessMonitor {
    pub fn new(pid: u32) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let monitor_handle = {
            let stop_flag = Arc::clone(&stop_flag);
            Some(thread::spawn(move || {
                Self::monitor_process(pid, stop_flag)
            }))
        };

        Self {
            pid,
            stop_flag,
            monitor_handle,
        }
    }

    pub fn stop_and_get_status(mut self) -> CapsuleResult<ProcessStatus> {
        self.stop_flag.store(true, Ordering::Relaxed);

        if let Some(handle) = self.monitor_handle.take() {
            handle.join().map_err(|_| {
                ExecutionError::MonitoringError("Process monitor thread panicked".to_string())
            })?
        } else {
            Ok(ProcessStatus::Unknown)
        }
    }

    fn monitor_process(
        pid: u32,
        stop_flag: Arc<AtomicBool>,
    ) -> CapsuleResult<ProcessStatus> {
        #[cfg(target_os = "linux")]
        {
            use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
            use nix::unistd::Pid;

        let nix_pid = Pid::from_raw(pid as i32);

        loop {
            if stop_flag.load(Ordering::Relaxed) {
                return Ok(ProcessStatus::Running);
            }

            match waitpid(nix_pid, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(_, exit_code)) => {
                    return Ok(ProcessStatus::Exited(exit_code));
                }
                Ok(WaitStatus::Signaled(_, signal, _)) => {
                    return Ok(ProcessStatus::Signaled(signal as i32));
                }
                Ok(WaitStatus::Stopped(_, signal)) => {
                    return Ok(ProcessStatus::Stopped(signal as i32));
                }
                Ok(WaitStatus::StillAlive) => {
                    // Process is still running
                }
                Ok(_) => {
                    // Other status types
                }
                Err(nix::errno::Errno::ECHILD) => {
                    // Process has already been reaped
                    return Ok(ProcessStatus::Unknown);
                }
                Err(_) => {
                    // Other errors
                    return Ok(ProcessStatus::Unknown);
                }
            }

            thread::sleep(Duration::from_millis(10));
        }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            // Simple polling implementation for non-Linux platforms
            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    return Ok(ProcessStatus::Running);
                }
                
                if !Self::check_process_exists_simple(pid) {
                    return Ok(ProcessStatus::Exited(0));
                }
                
                thread::sleep(Duration::from_millis(100));
            }
        }
    }

    pub fn is_alive(&self) -> bool {
        self.check_process_exists(self.pid)
    }

    fn check_process_exists(&self, pid: u32) -> bool {
        Self::check_process_exists_simple(pid)
    }
    
    fn check_process_exists_simple(pid: u32) -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new(&format!("/proc/{}", pid)).exists()
        }
        #[cfg(not(target_os = "linux"))]
        {
            // On non-Linux, try to send signal 0 to check if process exists
            unsafe {
                libc::kill(pid as i32, 0) == 0
            }
        }
    }
}

pub struct TimeoutMonitor {
    timeout_duration: Duration,
    start_time: Instant,
    receiver: mpsc::Receiver<()>,
    _handle: thread::JoinHandle<()>,
}

impl TimeoutMonitor {
    pub fn new(timeout_duration: Duration) -> (Self, mpsc::Sender<()>) {
        let (sender, receiver) = mpsc::channel();
        let start_time = Instant::now();
        
        let timeout_sender = sender.clone();
        let handle = thread::spawn(move || {
            thread::sleep(timeout_duration);
            let _ = timeout_sender.send(());
        });

        (
            Self {
                timeout_duration,
                start_time,
                receiver,
                _handle: handle,
            },
            sender,
        )
    }

    pub fn check_timeout(&self) -> bool {
        self.receiver.try_recv().is_ok()
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn remaining(&self) -> Duration {
        self.timeout_duration.saturating_sub(self.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockResourceProvider {
        memory: u64,
        cpu_time: u64,
    }

    impl ResourceProvider for MockResourceProvider {
        fn get_usage(&self) -> CapsuleResult<ResourceUsage> {
            Ok(ResourceUsage {
                memory_bytes: self.memory,
                cpu_time_us: self.cpu_time,
                user_time_us: self.cpu_time / 2,
                kernel_time_us: self.cpu_time / 2,
                io_bytes_read: 1024,
                io_bytes_written: 512,
            })
        }

        fn check_oom_killed(&self) -> CapsuleResult<bool> {
            Ok(false)
        }
    }

    #[test]
    fn test_resource_monitor() {
        let provider = Arc::new(MockResourceProvider {
            memory: 1024 * 1024,
            cpu_time: 1000,
        });

        let monitor = ResourceMonitor::new(provider, Duration::from_millis(10));
        
        thread::sleep(Duration::from_millis(50));
        
        let result = monitor.stop_and_get_result().unwrap();
        assert!(result.peak_memory > 0);
        assert!(result.wall_time >= Duration::from_millis(50));
    }

    #[test]
    fn test_timeout_monitor() {
        let (monitor, _sender) = TimeoutMonitor::new(Duration::from_millis(100));
        
        assert!(!monitor.check_timeout());
        
        thread::sleep(Duration::from_millis(150));
        
        assert!(monitor.check_timeout());
    }

    #[test]
    fn test_process_monitor() {
        // Test with current process (should be running)
        let monitor = ProcessMonitor::new(std::process::id());
        assert!(monitor.is_alive());
        
        let status = monitor.stop_and_get_status().unwrap();
        match status {
            ProcessStatus::Running => {},
            _ => panic!("Expected running status"),
        }
    }
}