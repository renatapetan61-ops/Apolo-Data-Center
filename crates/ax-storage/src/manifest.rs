use apolo_ax_core::ByteCount;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectId(pub Uuid);

impl ObjectId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl Default for ObjectId {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredChunk {
    pub index: u32,
    pub size_bytes: ByteCount,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub project_id: String,
    pub filename: String,
    pub content_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectManifest {
    pub object_id: ObjectId,
    pub metadata: ObjectMetadata,
    pub size_bytes: ByteCount,
    pub sha256: String,
    pub chunks: Vec<StoredChunk>,
    pub created_at_unix: i64,
}
