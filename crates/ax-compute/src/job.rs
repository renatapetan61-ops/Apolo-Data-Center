use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

/// A process workload. Program and arguments are passed directly to exec;
/// no shell interpolation is performed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeJob {
    pub job_id: String,
    pub reservation_id: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub working_directory: PathBuf,
    pub timeout_seconds: u64,
}

impl ComputeJob {
    pub fn validate(&self) -> bool {
        !self.job_id.trim().is_empty()
            && !self.reservation_id.trim().is_empty()
            && self.timeout_seconds > 0
            && !self.program.as_os_str().is_empty()
            && !self.working_directory.as_os_str().is_empty()
            && self.environment.keys().all(|key| {
                !key.is_empty()
                    && key.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                    && !key.starts_with("LD_")
                    && key != "PATH"
            })
    }
}
