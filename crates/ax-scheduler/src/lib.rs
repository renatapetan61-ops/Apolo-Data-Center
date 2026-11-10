//! In-process AX scheduler with atomic reservations.
//!
//! This first implementation is single-process. It deliberately does not claim
//! durable or multi-replica coordination; those require a transactional backend.
use apolo_ax_core::{NodeInventory, ResourceId, ResourceRequest, ResourceState};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Mutex;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    #[error("invalid scheduler request: {0}")]
    InvalidRequest(&'static str),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("no verified healthy node can satisfy the request")]
    NoCapacity,
    #[error("job already has an active reservation: {0}")]
    DuplicateJob(String),
    #[error("reservation not found: {0}")]
    ReservationNotFound(String),
    #[error("scheduler state lock is poisoned")]
    LockPoisoned,
    #[error("capacity arithmetic overflow")]
    CapacityOverflow,
}

pub type SchedulerResult<T> = Result<T, SchedulerError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reservation {
    pub reservation_id: String,
    pub job_id: String,
    pub node_id: String,
    pub request: ResourceRequest,
    pub accelerator_ids: Vec<ResourceId>,
    pub created_at_unix: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Usage {
    cpu_cores: u32,
    memory_bytes: u128,
    storage_bytes: u128,
    network_mbps: u64,
    accelerator_ids: HashSet<ResourceId>,
}

#[derive(Debug, Default)]
struct State {
    nodes: BTreeMap<String, NodeInventory>,
    usage: HashMap<String, Usage>,
    reservations: HashMap<String, Reservation>,
    jobs: HashMap<String, String>,
}

/// Thread-safe scheduler for one process. All reserve/release operations are
/// serialized under one mutex, preventing concurrent over-allocation here.
#[derive(Debug, Default)]
pub struct AxScheduler {
    state: Mutex<State>,
}

impl AxScheduler {
    pub fn new() -> Self { Self::default() }

    pub fn register_node(&self, inventory: NodeInventory) -> SchedulerResult<()> {
        inventory.validate().map_err(|_| SchedulerError::InvalidRequest("invalid node inventory"))?;
        let mut state = self.state.lock().map_err(|_| SchedulerError::LockPoisoned)?;
        let usage = state.usage.get(&inventory.node_id).cloned().unwrap_or_default();
        if inventory.cpu_cores < usage.cpu_cores
            || inventory.memory_used_bytes.as_bytes().saturating_add(usage.memory_bytes) > inventory.memory_total_bytes.as_bytes()
            || inventory.storage_used_bytes.as_bytes()
                .saturating_add(inventory.storage_reserved_bytes.as_bytes())
                .saturating_add(usage.storage_bytes) > inventory.storage_total_bytes.as_bytes()
            || usage.accelerator_ids.iter().any(|reserved_id| {
                !inventory.accelerators.iter().any(|device| {
                    device.id == *reserved_id && device.is_schedulable()
                })
            })
        {
            return Err(SchedulerError::InvalidRequest("updated inventory conflicts with active reservations"));
        }
        state.usage.entry(inventory.node_id.clone()).or_default();
        state.nodes.insert(inventory.node_id.clone(), inventory);
        Ok(())
    }

    pub fn unregister_node(&self, node_id: &str) -> SchedulerResult<()> {
        let mut state = self.state.lock().map_err(|_| SchedulerError::LockPoisoned)?;
        if state.usage.get(node_id).is_some_and(|u| u.cpu_cores > 0 || u.memory_bytes > 0 || u.storage_bytes > 0 || !u.accelerator_ids.is_empty()) {
            return Err(SchedulerError::InvalidRequest("cannot unregister a node with active reservations"));
        }
        state.nodes.remove(node_id).ok_or_else(|| SchedulerError::NodeNotFound(node_id.into()))?;
        state.usage.remove(node_id);
        Ok(())
    }

    pub fn reserve(&self, job_id: impl Into<String>, request: ResourceRequest, now_unix: i64) -> SchedulerResult<Reservation> {
        let job_id = job_id.into();
        if job_id.trim().is_empty() || now_unix <= 0 || request.validate().is_err() {
            return Err(SchedulerError::InvalidRequest("job id, timestamp and resource request must be valid"));
        }
        let mut state = self.state.lock().map_err(|_| SchedulerError::LockPoisoned)?;
        if state.jobs.contains_key(&job_id) {
            return Err(SchedulerError::DuplicateJob(job_id));
        }

        // Best-fit: minimize leftover CPU, then memory, then node id for determinism.
        let mut candidates: Vec<(u128, u128, String, Vec<ResourceId>)> = Vec::new();
        for (node_id, node) in &state.nodes {
            let usage = state.usage.get(node_id).cloned().unwrap_or_default();
            if !node.is_schedulable() { continue; }
            let cpu_used = usage.cpu_cores;
            let mem_used = node.memory_used_bytes.as_bytes().saturating_add(usage.memory_bytes);
            let storage_used = node.storage_used_bytes.as_bytes()
                .saturating_add(node.storage_reserved_bytes.as_bytes())
                .saturating_add(usage.storage_bytes);
            if node.cpu_cores.saturating_sub(cpu_used) < request.cpu_cores
                || node.memory_total_bytes.as_bytes().saturating_sub(mem_used) < request.memory_bytes.as_bytes()
                || node.storage_total_bytes.as_bytes().saturating_sub(storage_used) < request.storage_bytes.as_bytes()
                || node.network_mbps.saturating_sub(usage.network_mbps) < request.network_mbps
            { continue; }

            let mut accelerator_ids = Vec::new();
            if request.accelerator_count > 0 {
                let kind = request.accelerator_kind.ok_or(SchedulerError::InvalidRequest("accelerator kind is required"))?;
                for device in &node.accelerators {
                    if device.kind == kind && device.state == ResourceState::Healthy && device.hardware_verified
                        && !usage.accelerator_ids.contains(&device.id)
                    {
                        accelerator_ids.push(device.id);
                        if accelerator_ids.len() == request.accelerator_count as usize { break; }
                    }
                }
                if accelerator_ids.len() != request.accelerator_count as usize { continue; }
            }
            let remaining_cpu = (node.cpu_cores - cpu_used - request.cpu_cores) as u128;
            let remaining_mem = node.memory_total_bytes.as_bytes() - mem_used - request.memory_bytes.as_bytes();
            candidates.push((remaining_cpu, remaining_mem, node_id.clone(), accelerator_ids));
        }
        candidates.sort_by(|a, b| (&a.0, &a.1, &a.2).cmp(&(&b.0, &b.1, &b.2)));
        let (_, _, node_id, accelerator_ids) = candidates.into_iter().next().ok_or(SchedulerError::NoCapacity)?;

        let usage = state.usage.entry(node_id.clone()).or_default();
        let next_cpu = usage.cpu_cores.checked_add(request.cpu_cores).ok_or(SchedulerError::CapacityOverflow)?;
        let next_memory = usage.memory_bytes.checked_add(request.memory_bytes.as_bytes()).ok_or(SchedulerError::CapacityOverflow)?;
        let next_storage = usage.storage_bytes.checked_add(request.storage_bytes.as_bytes()).ok_or(SchedulerError::CapacityOverflow)?;
        let next_network = usage.network_mbps.checked_add(request.network_mbps).ok_or(SchedulerError::CapacityOverflow)?;
        usage.cpu_cores = next_cpu;
        usage.memory_bytes = next_memory;
        usage.storage_bytes = next_storage;
        usage.network_mbps = next_network;
        usage.accelerator_ids.extend(accelerator_ids.iter().copied());

        let reservation = Reservation {
            reservation_id: Uuid::new_v4().to_string(),
            job_id: job_id.clone(),
            node_id,
            request,
            accelerator_ids,
            created_at_unix: now_unix,
        };
        state.jobs.insert(job_id, reservation.reservation_id.clone());
        state.reservations.insert(reservation.reservation_id.clone(), reservation.clone());
        Ok(reservation)
    }

    pub fn release(&self, reservation_id: &str) -> SchedulerResult<Reservation> {
        let mut state = self.state.lock().map_err(|_| SchedulerError::LockPoisoned)?;
        let reservation = state.reservations.get(reservation_id).cloned()
            .ok_or_else(|| SchedulerError::ReservationNotFound(reservation_id.into()))?;
        let usage = state.usage.get_mut(&reservation.node_id).ok_or(SchedulerError::InvalidRequest("reservation node usage is missing"))?;
        usage.cpu_cores = usage.cpu_cores.saturating_sub(reservation.request.cpu_cores);
        usage.memory_bytes = usage.memory_bytes.saturating_sub(reservation.request.memory_bytes.as_bytes());
        usage.storage_bytes = usage.storage_bytes.saturating_sub(reservation.request.storage_bytes.as_bytes());
        usage.network_mbps = usage.network_mbps.saturating_sub(reservation.request.network_mbps);
        for id in &reservation.accelerator_ids { usage.accelerator_ids.remove(id); }
        state.reservations.remove(reservation_id);
        state.jobs.remove(&reservation.job_id);
        Ok(reservation)
    }

    pub fn get_reservation(&self, reservation_id: &str) -> SchedulerResult<Reservation> {
        let state = self.state.lock().map_err(|_| SchedulerError::LockPoisoned)?;
        state.reservations.get(reservation_id).cloned()
            .ok_or_else(|| SchedulerError::ReservationNotFound(reservation_id.into()))
    }

    pub fn list_nodes(&self) -> SchedulerResult<Vec<NodeInventory>> {
        let state = self.state.lock().map_err(|_| SchedulerError::LockPoisoned)?;
        Ok(state.nodes.values().cloned().collect())
    }

    pub fn active_reservations(&self) -> SchedulerResult<Vec<Reservation>> {
        let state = self.state.lock().map_err(|_| SchedulerError::LockPoisoned)?;
        Ok(state.reservations.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apolo_ax_core::{ByteCount, NodeState};

    fn node(id: &str) -> NodeInventory {
        NodeInventory {
            node_id: id.into(), state: NodeState::Healthy, cpu_cores: 8,
            memory_total_bytes: ByteCount::from_bytes(256), memory_used_bytes: ByteCount::ZERO,
            storage_total_bytes: ByteCount::from_bytes(1_000), storage_used_bytes: ByteCount::ZERO,
            storage_reserved_bytes: ByteCount::ZERO, network_mbps: 10_000,
            accelerators: vec![], observed_at_unix: 1_800_000_000, hardware_verified: true,
        }
    }

    fn request(cpu: u32) -> ResourceRequest {
        ResourceRequest { cpu_cores: cpu, memory_bytes: ByteCount::from_bytes(16),
            accelerator_count: 0, accelerator_kind: None,
            storage_bytes: ByteCount::from_bytes(100), network_mbps: 100 }
    }

    #[test]
    fn reserves_and_releases_capacity() {
        let scheduler = AxScheduler::new();
        scheduler.register_node(node("n1")).unwrap();
        let reservation = scheduler.reserve("job-1", request(6), 1_800_000_001).unwrap();
        assert_eq!(reservation.node_id, "n1");
        assert_eq!(scheduler.release(&reservation.reservation_id).unwrap(), reservation);
        assert!(scheduler.reserve("job-1", request(6), 1_800_000_002).is_ok());
    }

    #[test]
    fn prevents_double_allocation() {
        let scheduler = AxScheduler::new();
        scheduler.register_node(node("n1")).unwrap();
        scheduler.reserve("job-1", request(6), 1_800_000_001).unwrap();
        assert_eq!(scheduler.reserve("job-2", request(6), 1_800_000_002), Err(SchedulerError::NoCapacity));
    }

    #[test]
    fn rejects_duplicate_active_job() {
        let scheduler = AxScheduler::new();
        scheduler.register_node(node("n1")).unwrap();
        scheduler.reserve("job-1", request(1), 1_800_000_001).unwrap();
        assert_eq!(scheduler.reserve("job-1", request(1), 1_800_000_002), Err(SchedulerError::DuplicateJob("job-1".into())));
    }

    #[test]
    fn refuses_unverified_nodes() {
        let scheduler = AxScheduler::new();
        let mut inventory = node("n1");
        inventory.hardware_verified = false;
        scheduler.register_node(inventory).unwrap();
        assert_eq!(scheduler.reserve("job-1", request(1), 1_800_000_001), Err(SchedulerError::NoCapacity));
    }

    #[test]
    fn inventory_update_cannot_overcommit_active_reservations() {
        let scheduler = AxScheduler::new();
        scheduler.register_node(node("n1")).unwrap();
        scheduler.reserve("job-1", request(4), 1_800_000_001).unwrap();
        let mut changed = node("n1");
        changed.cpu_cores = 2;
        assert!(scheduler.register_node(changed).is_err());
    }

    #[test]
    fn concurrent_reservations_never_overcommit_cpu() {
        let scheduler = std::sync::Arc::new(AxScheduler::new());
        scheduler.register_node(node("n1")).unwrap();
        let mut threads = Vec::new();
        for i in 0..16 {
            let scheduler = scheduler.clone();
            threads.push(std::thread::spawn(move || scheduler.reserve(format!("job-{i}"), request(1), 1_800_000_001).is_ok()));
        }
        let successes = threads.into_iter().map(|t| t.join().unwrap()).filter(|ok| *ok).count();
        assert_eq!(successes, 8);
    }
}
