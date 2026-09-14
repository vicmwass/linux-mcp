use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    pub timeout: Duration,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            max_stdout_bytes: 1024 * 1024, // 1 MB
            max_stderr_bytes: 1024 * 1024, // 1 MB
        }
    }
}