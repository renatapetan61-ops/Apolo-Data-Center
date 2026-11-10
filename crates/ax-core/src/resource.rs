use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceId(pub Uuid);
impl ResourceId { pub fn new() -> Self { Self(Uuid::new_v4()) } }
impl Default for ResourceId { fn default() -> Self { Self::new() } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceleratorKind { Cpu, NvidiaCuda, AmdRocm, IntelGpu, AppleMetal, Other }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceState { Discovered, Healthy, Draining, Offline, Quarantined }

/// Inventory record populated by a trusted node probe.
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
    pub fn is_schedulable(&self) -> bool { self.hardware_verified && self.state == ResourceState::Healthy }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unverified_resource_is_not_schedulable() {
        let r = ComputeResource { id: ResourceId::new(), node_id: "test-node".into(), kind: AcceleratorKind::Cpu, device_name: "probe fixture".into(), memory_bytes: 1024, compute_units: 2, state: ResourceState::Healthy, hardware_verified: false };
        assert!(!r.is_schedulable());
    }
}
