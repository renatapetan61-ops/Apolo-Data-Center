//! AX Compute: local, policy-constrained workload execution.
//!
//! The initial adapter launches native processes. It is not yet a container,
//! VM, GPU, remote-node or multi-tenant isolation runtime.
mod error;
mod executor;
mod job;
mod policy;

pub use error::{ComputeError, ComputeResult};
pub use executor::{ExecutionReceipt, ProcessExecutor};
pub use job::ComputeJob;
pub use policy::ExecutionPolicy;

#[cfg(test)]
mod tests {
    use super::*;
    use apolo_ax_core::{ByteCount, NodeInventory, NodeState, ResourceRequest};
    use apolo_ax_scheduler::AxScheduler;
    use std::{collections::BTreeMap, path::PathBuf};

    fn job(reservation_id: &str) -> ComputeJob {
        ComputeJob {
            job_id: "job-test".into(),
            reservation_id: reservation_id.into(),
            program: PathBuf::from("/bin/echo"),
            args: vec!["hello".into()],
            environment: BTreeMap::new(),
            working_directory: std::env::temp_dir(),
            timeout_seconds: 2,
        }
    }

    fn scheduler() -> AxScheduler {
        let scheduler = AxScheduler::new();
        scheduler.register_node(NodeInventory {
            node_id: "node-test".into(),
            state: NodeState::Healthy,
            cpu_cores: 2,
            memory_total_bytes: ByteCount::from_bytes(1024 * 1024),
            memory_used_bytes: ByteCount::ZERO,
            storage_total_bytes: ByteCount::from_bytes(1024 * 1024),
            storage_used_bytes: ByteCount::ZERO,
            storage_reserved_bytes: ByteCount::ZERO,
            network_mbps: 1000,
            accelerators: vec![],
            observed_at_unix: 1_800_000_000,
            hardware_verified: true,
        }).unwrap();
        scheduler
    }

    fn request() -> ResourceRequest {
        ResourceRequest {
            cpu_cores: 1,
            memory_bytes: ByteCount::from_bytes(1024),
            accelerator_count: 0,
            accelerator_kind: None,
            storage_bytes: ByteCount::from_bytes(1024),
            network_mbps: 1,
        }
    }

    #[test]
    fn refuses_to_execute_without_scheduler_reservation() {
        let scheduler = scheduler();
        let policy = ExecutionPolicy {
            allowed_programs: vec![PathBuf::from("/bin/echo")],
            sandbox_root: std::env::temp_dir(),
            max_timeout_seconds: 5,
            max_output_bytes_per_stream: 1024,
        };
        let executor = ProcessExecutor::new(policy).unwrap();
        assert!(matches!(executor.execute(&scheduler, &job("missing"), 1_800_000_001), Err(ComputeError::InvalidReservation(_))));
    }

    #[test]
    fn job_rejects_dangerous_environment_overrides() {
        let mut value = job("reservation");
        value.environment.insert("LD_PRELOAD".into(), "/tmp/evil.so".into());
        assert!(!value.validate());
        value.environment.clear();
        value.environment.insert("PATH".into(), "/tmp".into());
        assert!(!value.validate());
    }

    #[test]
    fn job_rejects_zero_timeout() {
        let mut value = job("reservation");
        value.timeout_seconds = 0;
        assert!(!value.validate());
    }

    #[test]
    fn process_runs_only_with_active_reservation() {
        let scheduler = scheduler();
        let reservation = scheduler.reserve("job-test", request(), 1_800_000_001).unwrap();
        let policy = ExecutionPolicy {
            allowed_programs: vec![PathBuf::from("/bin/echo")],
            sandbox_root: std::env::temp_dir(),
            max_timeout_seconds: 5,
            max_output_bytes_per_stream: 1024,
        };
        let executor = ProcessExecutor::new(policy).unwrap();
        let receipt = executor.execute(&scheduler, &job(&reservation.reservation_id), 1_800_000_002).unwrap();
        assert_eq!(receipt.exit_code, Some(0));
        assert!(!receipt.timed_out);
        assert!(String::from_utf8_lossy(&receipt.stdout).contains("hello"));
        assert!(scheduler.get_reservation(&reservation.reservation_id).is_err());
    }
}
