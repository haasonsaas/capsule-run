use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants are part of API design but not yet used
pub enum CapsuleError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Sandbox setup failed: {0}")]
    SandboxSetup(#[from] SandboxError),

    #[error("Execution failed: {0}")]
    Execution(#[from] ExecutionError),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Security violation: {0}")]
    Security(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("System call error: {0}")]
    Syscall(String),
}

#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants are part of API design but not yet used
pub enum SandboxError {
    #[error("Failed to create namespace: {namespace}")]
    NamespaceCreation { namespace: String },

    #[error("Failed to setup cgroups: {0}")]
    CgroupSetup(String),

    #[error("Failed to apply seccomp filter: {0}")]
    SeccompSetup(String),

    #[error("Failed to setup filesystem isolation: {0}")]
    FilesystemSetup(String),

    #[error("Failed to drop capabilities: {0}")]
    CapabilityDrop(String),

    #[error("User namespace mapping failed: {0}")]
    UserMapping(String),
}

#[derive(Error, Debug)]
#[allow(dead_code)] // Some variants are part of API design but not yet used
pub enum ExecutionError {
    #[error("Command timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("Command killed by signal {signal}")]
    Signal { signal: i32 },

    #[error("Process spawning failed: {0}")]
    SpawnFailed(String),

    #[error("I/O capture failed: {0}")]
    IoCaptureError(String),

    #[error("Resource monitoring failed: {0}")]
    MonitoringError(String),

    #[error("Output size limit exceeded: {limit} bytes")]
    OutputSizeLimit { limit: usize },
}

pub type CapsuleResult<T> = Result<T, CapsuleError>;

#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are part of API design but not yet used
pub struct ErrorCode {
    pub code: &'static str,
    pub message: String,
    pub category: ErrorCategory,
}

#[derive(Debug, Clone)]
pub enum ErrorCategory {
    Configuration,
    Security,
    Resource,
    Execution,
    System,
}

impl ErrorCode {
    pub fn new(code: &'static str, message: String, category: ErrorCategory) -> Self {
        Self {
            code,
            message,
            category,
        }
    }
}

impl From<CapsuleError> for ErrorCode {
    fn from(error: CapsuleError) -> Self {
        match error {
            CapsuleError::Config(msg) => ErrorCode::new("E1001", msg, ErrorCategory::Configuration),
            CapsuleError::SandboxSetup(SandboxError::NamespaceCreation { namespace }) => {
                ErrorCode::new(
                    "E2001",
                    format!("Failed to create {} namespace", namespace),
                    ErrorCategory::Security,
                )
            }
            CapsuleError::SandboxSetup(SandboxError::CgroupSetup(msg)) => {
                ErrorCode::new("E2002", msg, ErrorCategory::Resource)
            }
            CapsuleError::SandboxSetup(SandboxError::SeccompSetup(msg)) => {
                ErrorCode::new("E2003", msg, ErrorCategory::Security)
            }
            CapsuleError::SandboxSetup(SandboxError::FilesystemSetup(msg)) => {
                ErrorCode::new("E2004", msg, ErrorCategory::Security)
            }
            CapsuleError::SandboxSetup(SandboxError::CapabilityDrop(msg)) => {
                ErrorCode::new("E2005", msg, ErrorCategory::Security)
            }
            CapsuleError::SandboxSetup(SandboxError::UserMapping(msg)) => {
                ErrorCode::new("E2006", msg, ErrorCategory::Security)
            }
            CapsuleError::Execution(ExecutionError::Timeout { timeout_ms }) => ErrorCode::new(
                "E3001",
                format!("Command exceeded timeout limit of {}ms", timeout_ms),
                ErrorCategory::Execution,
            ),
            CapsuleError::Execution(ExecutionError::Signal { signal }) => ErrorCode::new(
                "E3002",
                format!("Command killed by signal {}", signal),
                ErrorCategory::Execution,
            ),
            CapsuleError::Execution(ExecutionError::SpawnFailed(msg)) => {
                ErrorCode::new("E3003", msg, ErrorCategory::Execution)
            }
            CapsuleError::Execution(ExecutionError::IoCaptureError(msg)) => {
                ErrorCode::new("E3004", msg, ErrorCategory::System)
            }
            CapsuleError::Execution(ExecutionError::MonitoringError(msg)) => {
                ErrorCode::new("E3005", msg, ErrorCategory::System)
            }
            CapsuleError::Execution(ExecutionError::OutputSizeLimit { limit }) => ErrorCode::new(
                "E3006",
                format!("Output exceeded size limit of {} bytes", limit),
                ErrorCategory::Resource,
            ),
            CapsuleError::ResourceLimit(msg) => {
                ErrorCode::new("E4001", msg, ErrorCategory::Resource)
            }
            CapsuleError::Security(msg) => ErrorCode::new("E5001", msg, ErrorCategory::Security),
            CapsuleError::Io(err) => ErrorCode::new(
                "E6001",
                format!("I/O operation failed: {}", err),
                ErrorCategory::System,
            ),
            CapsuleError::Json(err) => ErrorCode::new(
                "E6002",
                format!("JSON parsing failed: {}", err),
                ErrorCategory::Configuration,
            ),
            CapsuleError::Syscall(msg) => ErrorCode::new("E6003", msg, ErrorCategory::System),
        }
    }
}
