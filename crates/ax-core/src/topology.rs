use crate::{
    AxError, AxResult, ByteCount, ComputeResource, ResourceRequest, ResourceState,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    Registering,
    Healthy,
    Draining,
    Offline,
    Quarantined,
}

/// Inventory snapshot reported by an authenticated AX node agent.
/// observed_at_unix is seconds since Unix epoch, supplied by the agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeInventory {
    pub node_id: String,
    pub state: NodeState,
    pub cpu_cores: u32,
    pub memory_total_bytes: ByteCount,
    pub memory_used_bytes: ByteCount,
    pub storage_total_bytes: ByteCount,
    pub storage_used_bytes: ByteCount,
    pub storage_reserved_bytes: ByteCount,
    pub network_mbps: u64,
    pub accelerators: Vec<ComputeResource>,
    pub observed_at_unix: i64,
    pub hardware_verified: bool,
}

impl NodeInventory {
    pub fn validate(&self) -> AxResult<()> {
        if self.node_id.trim().is_empty() {
            return Err(AxError::InvalidIdentifier("node_id cannot be empty"));
        }
        if self.observed_at_unix <= 0 {
            return Err(AxError::InvalidResource("observed_at_unix must be positive"));
        }
        if self.cpu_cores == 0 || self.memory_total_bytes == ByteCount::ZERO {
            return Err(AxError::InvalidResource("node CPU and memory must be positive"));
        }
        if self.memory_used_bytes > self.memory_total_bytes {
            return Err(AxError::InvalidCapacity("node memory usage exceeds total"));
        }
        self.storage_total_bytes.checked_sub(
            self.storage_used_bytes.checked_add(self.storage_reserved_bytes)?,
        )?;

        let mut ids = HashSet::new();
        for accelerator in &self.accelerators {
            accelerator.validate()?;
            if accelerator.node_id != self.node_id {
                return Err(AxError::InvalidResource("accelerator belongs to a different node"));
            }
            if !ids.insert(accelerator.id) {
                return Err(AxError::InvalidResource("duplicate accelerator id in node inventory"));
            }
        }
        Ok(())
    }

    pub fn is_schedulable(&self) -> bool {
        self.hardware_verified && self.state == NodeState::Healthy && self.validate().is_ok()
    }

    /// Checks observed capacity only; does not reserve it.
    pub fn can_satisfy(&self, request: &ResourceRequest) -> AxResult<bool> {
        request.validate()?;
        self.validate()?;
        if !self.is_schedulable() {
            return Ok(false);
        }
        let available_memory = self.memory_total_bytes.checked_sub(self.memory_used_bytes)?;
        let available_storage = self.storage_total_bytes.checked_sub(
            self.storage_used_bytes.checked_add(self.storage_reserved_bytes)?,
        )?;
        if self.cpu_cores < request.cpu_cores
            || available_memory < request.memory_bytes
            || available_storage < request.storage_bytes
            || self.network_mbps < request.network_mbps
        {
            return Ok(false);
        }

        if request.accelerator_count == 0 {
            return Ok(true);
        }
        let kind = request.accelerator_kind.expect("validated accelerator kind");
        let available = self.accelerators.iter()
            .filter(|device| {
                device.kind == kind
                    && device.state == ResourceState::Healthy
                    && device.hardware_verified
            })
            .count();
        Ok(available >= request.accelerator_count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AcceleratorKind, ResourceId};

    fn node() -> NodeInventory {
        NodeInventory {
            node_id: "node-a".into(),
            state: NodeState::Healthy,
            cpu_cores: 16,
            memory_total_bytes: ByteCount::from_bytes(64_000),
            memory_used_bytes: ByteCount::from_bytes(8_000),
            storage_total_bytes: ByteCount::from_bytes(1_000_000),
            storage_used_bytes: ByteCount::from_bytes(100_000),
            storage_reserved_bytes: ByteCount::from_bytes(50_000),
            network_mbps: 10_000,
            accelerators: vec![],
            observed_at_unix: 1_800_000_000,
            hardware_verified: true,
        }
    }

    fn request() -> ResourceRequest {
        ResourceRequest {
            cpu_cores: 8,
            memory_bytes: ByteCount::from_bytes(32_000),
            accelerator_count: 0,
            accelerator_kind: None,
            storage_bytes: ByteCount::from_bytes(500_000),
            network_mbps: 1_000,
        }
    }

    #[test]
    fn healthy_verified_node_satisfies_request() {
        assert!(node().can_satisfy(&request()).unwrap());
    }

    #[test]
    fn unverified_node_cannot_be_scheduled() {
        let mut n = node();
        n.hardware_verified = false;
        assert!(!n.can_satisfy(&request()).unwrap());
    }

    #[test]
    fn storage_overcommit_is_rejected() {
        let mut n = node();
        n.storage_reserved_bytes = ByteCount::from_bytes(950_000);
        assert!(n.validate().is_err());
    }

    #[test]
    fn accelerator_count_is_checked_by_kind_and_health() {
        let mut n = node();
        n.accelerators.push(ComputeResource {
            id: ResourceId::new(),
            node_id: "node-a".into(),
            kind: AcceleratorKind::NvidiaCuda,
            device_name: "GPU fixture".into(),
            memory_bytes: 24_000,
            compute_units: 80,
            state: ResourceState::Healthy,
            hardware_verified: true,
        });
        let mut r = request();
        r.accelerator_count = 1;
        r.accelerator_kind = Some(AcceleratorKind::NvidiaCuda);
        assert!(n.can_satisfy(&r).unwrap());
        r.accelerator_count = 2;
        assert!(!n.can_satisfy(&r).unwrap());
    }
}
