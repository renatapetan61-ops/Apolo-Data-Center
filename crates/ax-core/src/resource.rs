use crate::{AxError, AxResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceId(pub Uuid);

impl ResourceId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for ResourceId {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceleratorKind {
    Cpu,
    NvidiaCuda,
    AmdRocm,
    IntelGpu,
    AppleMetal,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceState {
    Discovered,
    Healthy,
    Draining,
    Offline,
    Quarantined,
}

/// Individual accelerator discovered by a trusted node agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeResource {
    pub id: ResourceId,
    pub node_id: String,
    pub kind: AcceleratorKind,
    pub device_name: String,
    pub memory_bytes: u64,
    pub compute_units: u32,
    pub state: ResourceState,
    pub hardware_verified: bool,
}

impl ComputeResource {
    pub fn validate(&self) -> AxResult<()> {
        if self.node_id.trim().is_empty() {
            return Err(AxError::InvalidIdentifier("node_id cannot be empty"));
        }
        if self.device_name.trim().is_empty() {
            return Err(AxError::InvalidResource("device_name cannot be empty"));
        }
        if self.memory_bytes == 0 || self.compute_units == 0 {
            return Err(AxError::InvalidResource("memory and compute_units must be positive"));
        }
        Ok(())
    }

    pub fn is_schedulable(&self) -> bool {
        self.hardware_verified && self.state == ResourceState::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(verified: bool, state: ResourceState) -> ComputeResource {
        ComputeResource {
            id: ResourceId::new(),
            node_id: "node-test-01".into(),
            kind: AcceleratorKind::Cpu,
            device_name: "CPU fixture".into(),
            memory_bytes: 1024,
            compute_units: 2,
            state,
            hardware_verified: verified,
        }
    }

    #[test]
    fn unverified_resource_is_not_schedulable() {
        assert!(!fixture(false, ResourceState::Healthy).is_schedulable());
    }

    #[test]
    fn offline_resource_is_not_schedulable() {
        assert!(!fixture(true, ResourceState::Offline).is_schedulable());
    }

    #[test]
    fn valid_resource_passes_validation() {
        assert!(fixture(true, ResourceState::Healthy).validate().is_ok());
    }

    #[test]
    fn resource_without_memory_is_rejected() {
        let mut resource = fixture(true, ResourceState::Healthy);
        resource.memory_bytes = 0;
        assert!(resource.validate().is_err());
    }
}
