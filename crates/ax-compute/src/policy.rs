use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ExecutionPolicy {
    /// Exact canonical executable paths approved by the node operator.
    pub allowed_programs: Vec<PathBuf>,
    /// Jobs may only use directories under this canonical root.
    pub sandbox_root: PathBuf,
    pub max_timeout_seconds: u64,
    pub max_output_bytes_per_stream: usize,
}

impl ExecutionPolicy {
    pub fn validate(&self) -> bool {
        !self.allowed_programs.is_empty()
            && !self.sandbox_root.as_os_str().is_empty()
            && self.max_timeout_seconds > 0
            && self.max_output_bytes_per_stream > 0
    }
}
