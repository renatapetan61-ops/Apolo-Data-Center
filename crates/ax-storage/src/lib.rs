//! AX Storage: chunked object persistence with integrity verification.
//! The filesystem adapter is for local development and tests. Distributed
//! S3-compatible storage and transactional quota accounting are later adapters.
mod engine;
mod error;
mod manifest;
mod object_store;

pub use engine::{StorageEngine, StorageReceipt};
pub use error::{StorageError, StorageResult};
pub use manifest::{ObjectId, ObjectManifest, ObjectMetadata, StoredChunk};
pub use object_store::{LocalFsObjectStore, ObjectStore};
