//! APOLO AX core domain crate.
//! Canonical resource and capacity types; physical and logical capacity stay separate.
pub mod capacity;
pub mod error;
pub mod resource;
pub use capacity::{ByteCount, CapacitySnapshot};
pub use error::{AxError, AxResult};
pub use resource::{AcceleratorKind, ComputeResource, ResourceId, ResourceState};
