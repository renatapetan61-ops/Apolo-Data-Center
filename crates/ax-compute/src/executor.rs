use crate::{ComputeError, ComputeJob, ComputeResult, ExecutionPolicy};
use apolo_ax_scheduler::AxScheduler;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub execution_id: String,
    pub job_id: String,
    pub node_id: String,
    pub started_at_unix: i64,
    pub elapsed_millis: u128,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

fn canonical(path: &Path) -> ComputeResult<PathBuf> {
    fs::canonicalize(path).map_err(ComputeError::Io)
}

fn read_bounded<R: Read + Send + 'static>(
    mut reader: R,
    limit: usize,
) -> thread::JoinHandle<(Vec<u8>, bool)> {
    thread::spawn(move || {
        let mut output = Vec::with_capacity(limit.min(8192));
        let mut truncated = false;
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    let remaining = limit.saturating_sub(output.len());
                    let kept = count.min(remaining);
                    output.extend_from_slice(&buffer[..kept]);
                    truncated |= kept < count;
                }
            }
        }
        (output, truncated)
    })
}

/// Executes approved native processes on the local node after verifying that
/// the AX Scheduler holds the job's resource reservation.
#[derive(Debug, Clone)]
pub struct ProcessExecutor {
    policy: ExecutionPolicy,
}

impl ProcessExecutor {
    pub fn new(policy: ExecutionPolicy) -> ComputeResult<Self> {
        if !policy.validate() {
            return Err(ComputeError::InvalidPolicy(
                "allowlist, sandbox root, timeout and output limit are required",
            ));
        }
        let sandbox_root = canonical(&policy.sandbox_root)?;
        let mut allowed_programs = Vec::new();
        for program in &policy.allowed_programs {
            allowed_programs.push(canonical(program)?);
        }
        Ok(Self {
            policy: ExecutionPolicy {
                allowed_programs,
                sandbox_root,
                ..policy
            },
        })
    }

    /// Runs the reserved job and releases its reservation after the execution
    /// attempt finishes. Validation failures before execution leave it intact.
    pub fn execute(
        &self,
        scheduler: &AxScheduler,
        job: &ComputeJob,
        started_at_unix: i64,
    ) -> ComputeResult<ExecutionReceipt> {
        if !job.validate() || started_at_unix <= 0 {
            return Err(ComputeError::InvalidJob(
                "fields, timestamp or environment are invalid",
            ));
        }
        if job.timeout_seconds > self.policy.max_timeout_seconds {
            return Err(ComputeError::InvalidJob(
                "requested timeout exceeds node policy",
            ));
        }

        let reservation = scheduler
            .get_reservation(&job.reservation_id)
            .map_err(|error| ComputeError::InvalidReservation(error.to_string()))?;
        if reservation.job_id != job.job_id {
            return Err(ComputeError::InvalidReservation(
                "reservation belongs to another job".into(),
            ));
        }

        let result = self.execute_process(job, started_at_unix, &reservation.node_id);
        let release = scheduler.release(&job.reservation_id).map_err(|error| {
            ComputeError::InvalidReservation(format!("execution finished but reservation release failed: {error}"))
        });
        match (result, release) {
            (Ok(receipt), Ok(_)) => Ok(receipt),
            (Err(error), Ok(_)) => Err(error),
            (_, Err(error)) => Err(error),
        }
    }

    fn execute_process(
        &self,
        job: &ComputeJob,
        started_at_unix: i64,
        node_id: &str,
    ) -> ComputeResult<ExecutionReceipt> {
        let program = canonical(&job.program)?;
        if !self.policy.allowed_programs.contains(&program) {
            return Err(ComputeError::ExecutableNotAllowed);
        }

        let working_directory = canonical(&job.working_directory)?;
        if !working_directory.starts_with(&self.policy.sandbox_root) {
            return Err(ComputeError::WorkingDirectoryOutsideSandbox);
        }

        let mut command = Command::new(program);
        command
            .args(&job.args)
            .current_dir(working_directory)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in &job.environment {
            command.env(key, value);
        }

        let started = Instant::now();
        let mut child = command.spawn()?;
        let stdout = child.stdout.take().ok_or(ComputeError::OutputReaderFailed)?;
        let stderr = child.stderr.take().ok_or(ComputeError::OutputReaderFailed)?;
        let stdout_reader = read_bounded(stdout, self.policy.max_output_bytes_per_stream);
        let stderr_reader = read_bounded(stderr, self.policy.max_output_bytes_per_stream);

        let timeout = Duration::from_secs(job.timeout_seconds);
        let (status, timed_out) = loop {
            if let Some(status) = child.try_wait()? {
                break (status, false);
            }
            if started.elapsed() >= timeout {
                let _ = child.kill();
                break (child.wait()?, true);
            }
            thread::sleep(Duration::from_millis(10));
        };

        let (stdout, stdout_truncated) = stdout_reader
            .join()
            .map_err(|_| ComputeError::OutputReaderFailed)?;
        let (stderr, stderr_truncated) = stderr_reader
            .join()
            .map_err(|_| ComputeError::OutputReaderFailed)?;
        Ok(ExecutionReceipt {
            execution_id: Uuid::new_v4().to_string(),
            job_id: job.job_id.clone(),
            node_id: node_id.to_owned(),
            started_at_unix,
            elapsed_millis: started.elapsed().as_millis(),
            exit_code: status.code(),
            timed_out,
            stdout,
            stderr,
            stdout_truncated,
            stderr_truncated,
        })
    }
}
