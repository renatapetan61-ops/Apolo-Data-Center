//! APOLO AX core domain crate.
//! Canonical resource and capacity types; physical and logical capacity stay separate.
pub mod capacity;
pub mod error;
pub mod profile;
pub mod resource;
pub mod topology;

pub use capacity::{ByteCount, CapacitySnapshot};
pub use error::{AxError, AxResult};
pub use profile::{AxProfile, ProfileCode, ResourceRequest};
pub use resource::{AcceleratorKind, ComputeResource, ResourceId, ResourceState};
pub use topology::{NodeInventory, NodeState};
