use crate::error::{CapsuleResult, ExecutionError};
use std::io::{Read, Write};
use std::process::{ChildStdout, ChildStderr};
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::time::Duration;

pub struct IoCapture {
    stdout_handle: Option<thread::JoinHandle<CapsuleResult<Vec<u8>>>>,
    stderr_handle: Option<thread::JoinHandle<CapsuleResult<Vec<u8>>>>,
    max_output_size: usize,
}

impl IoCapture {
    pub fn new(
        stdout: Option<ChildStdout>,
        stderr: Option<ChildStderr>,
        max_output_size: usize,
    ) -> Self {
        let stdout_handle = stdout.map(|stdout| {
            let max_size = max_output_size;
            thread::spawn(move || Self::capture_stream(stdout, max_size, "stdout"))
        });

        let stderr_handle = stderr.map(|stderr| {
            let max_size = max_output_size;
            thread::spawn(move || Self::capture_stream(stderr, max_size, "stderr"))
        });

        Self {
            stdout_handle,
            stderr_handle,
            max_output_size,
        }
    }

    pub fn wait_for_completion(self) -> CapsuleResult<(String, String)> {
        let stdout = if let Some(handle) = self.stdout_handle {
            handle.join().map_err(|_| {
                ExecutionError::IoCaptureError("stdout capture thread panicked".to_string())
            })??
        } else {
            Vec::new()
        };

        let stderr = if let Some(handle) = self.stderr_handle {
            handle.join().map_err(|_| {
                ExecutionError::IoCaptureError("stderr capture thread panicked".to_string())
            })??
        } else {
            Vec::new()
        };

        let stdout_str = String::from_utf8_lossy(&stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&stderr).to_string();

        Ok((stdout_str, stderr_str))
    }

    fn capture_stream<R: Read>(
        mut stream: R,
        max_size: usize,
        stream_name: &str,
    ) -> CapsuleResult<Vec<u8>> {
        let mut buffer = Vec::new();
        let mut temp_buffer = [0u8; 4096];

        loop {
            match stream.read(&mut temp_buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    if buffer.len() + n > max_size {
                        return Err(ExecutionError::OutputSizeLimit { limit: max_size }.into());
                    }
                    buffer.extend_from_slice(&temp_buffer[..n]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    return Err(ExecutionError::IoCaptureError(format!(
                        "Failed to read from {}: {}",
                        stream_name, e
                    )).into());
                }
            }
        }

        Ok(buffer)
    }
}

pub struct StreamingIoCapture {
    stdout_receiver: Option<mpsc::Receiver<IoEvent>>,
    stderr_receiver: Option<mpsc::Receiver<IoEvent>>,
    stdout_handle: Option<thread::JoinHandle<()>>,
    stderr_handle: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub enum IoEvent {
    Data(Vec<u8>),
    Error(String),
    Eof,
}

impl StreamingIoCapture {
    pub fn new(
        stdout: Option<ChildStdout>,
        stderr: Option<ChildStderr>,
        max_output_size: usize,
    ) -> Self {
        let (stdout_receiver, stdout_handle) = if let Some(stdout) = stdout {
            let (tx, rx) = mpsc::channel();
            let handle = thread::spawn(move || {
                Self::stream_capture(stdout, tx, max_output_size, "stdout");
            });
            (Some(rx), Some(handle))
        } else {
            (None, None)
        };

        let (stderr_receiver, stderr_handle) = if let Some(stderr) = stderr {
            let (tx, rx) = mpsc::channel();
            let handle = thread::spawn(move || {
                Self::stream_capture(stderr, tx, max_output_size, "stderr");
            });
            (Some(rx), Some(handle))
        } else {
            (None, None)
        };

        Self {
            stdout_receiver,
            stderr_receiver,
            stdout_handle,
            stderr_handle,
        }
    }

    pub fn read_available(&self, timeout: Duration) -> (Option<IoEvent>, Option<IoEvent>) {
        let stdout_event = self.stdout_receiver.as_ref()
            .and_then(|rx| rx.recv_timeout(timeout).ok());
        
        let stderr_event = self.stderr_receiver.as_ref()
            .and_then(|rx| rx.recv_timeout(Duration::from_millis(1)).ok());

        (stdout_event, stderr_event)
    }

    pub fn collect_remaining(self) -> CapsuleResult<(String, String)> {
        let mut stdout_data = Vec::new();
        let mut stderr_data = Vec::new();

        // Collect remaining data from stdout
        if let Some(rx) = self.stdout_receiver {
            while let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                match event {
                    IoEvent::Data(data) => stdout_data.extend(data),
                    IoEvent::Error(err) => {
                        return Err(ExecutionError::IoCaptureError(format!("stdout error: {}", err)).into());
                    }
                    IoEvent::Eof => break,
                }
            }
        }

        // Collect remaining data from stderr
        if let Some(rx) = self.stderr_receiver {
            while let Ok(event) = rx.recv_timeout(Duration::from_millis(100)) {
                match event {
                    IoEvent::Data(data) => stderr_data.extend(data),
                    IoEvent::Error(err) => {
                        return Err(ExecutionError::IoCaptureError(format!("stderr error: {}", err)).into());
                    }
                    IoEvent::Eof => break,
                }
            }
        }

        // Wait for threads to finish
        if let Some(handle) = self.stdout_handle {
            let _ = handle.join();
        }
        if let Some(handle) = self.stderr_handle {
            let _ = handle.join();
        }

        let stdout_str = String::from_utf8_lossy(&stdout_data).to_string();
        let stderr_str = String::from_utf8_lossy(&stderr_data).to_string();

        Ok((stdout_str, stderr_str))
    }

    fn stream_capture<R: Read>(
        mut stream: R,
        sender: mpsc::Sender<IoEvent>,
        max_size: usize,
        stream_name: &str,
    ) {
        let mut total_size = 0;
        let mut buffer = [0u8; 1024];

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => {
                    let _ = sender.send(IoEvent::Eof);
                    break;
                }
                Ok(n) => {
                    total_size += n;
                    if total_size > max_size {
                        let _ = sender.send(IoEvent::Error(format!(
                            "Output size limit exceeded: {} bytes",
                            max_size
                        )));
                        break;
                    }
                    
                    let data = buffer[..n].to_vec();
                    if sender.send(IoEvent::Data(data)).is_err() {
                        break; // Receiver dropped
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    let _ = sender.send(IoEvent::Error(format!(
                        "Failed to read from {}: {}",
                        stream_name, e
                    )));
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn test_io_capture_simple() {
        let mut child = Command::new("echo")
            .arg("hello world")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn echo command");

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let capture = IoCapture::new(stdout, stderr, 1024);
        let (stdout_str, stderr_str) = capture.wait_for_completion().unwrap();

        child.wait().expect("Failed to wait for child");

        assert_eq!(stdout_str.trim(), "hello world");
        assert!(stderr_str.is_empty());
    }

    #[test]
    fn test_output_size_limit() {
        let mut child = Command::new("yes")
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn yes command");

        let stdout = child.stdout.take();
        let capture = IoCapture::new(stdout, None, 100); // Small limit

        let result = capture.wait_for_completion();
        child.kill().expect("Failed to kill child");
        let _ = child.wait();

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Output exceeded size limit"));
        }
    }
}