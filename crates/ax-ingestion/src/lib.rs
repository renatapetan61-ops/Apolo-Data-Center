//! AX Data Ingestion: validated, bounded, project-scoped in-memory ingestion queue.
//! Durable distributed queues and persistent deduplication are future adapters.
mod engine;
mod error;
mod format;

pub use engine::{IngestionConfig, IngestionInput, IngestionReceipt, IngestionService, QueuedEntry};
pub use error::{IngestionError, IngestionResult};
pub use format::DataFormat;
