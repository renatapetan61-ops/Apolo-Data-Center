use crate::{
    manifest::{ObjectId, ObjectManifest, ObjectMetadata, StoredChunk},
    object_store::ObjectStore,
    StorageError, StorageResult,
};
use apolo_ax_core::ByteCount;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

const MANIFEST_FILE: &str = "manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageReceipt {
    pub manifest: ObjectManifest,
    pub physical_bytes_written: ByteCount,
}

pub struct StorageEngine<S: ObjectStore> {
    store: S,
    quota_bytes: ByteCount,
    chunk_size_bytes: usize,
    used_bytes: Arc<Mutex<u128>>,
}

impl<S: ObjectStore> StorageEngine<S> {
    pub fn new(store: S, quota_bytes: ByteCount, chunk_size_bytes: usize) -> StorageResult<Self> {
        if chunk_size_bytes == 0 {
            return Err(StorageError::InvalidConfiguration("chunk size must be positive"));
        }
        Ok(Self {
            store,
            quota_bytes,
            chunk_size_bytes,
            used_bytes: Arc::new(Mutex::new(0)),
        })
    }

    pub fn used_bytes(&self) -> StorageResult<ByteCount> {
        let used = self.used_bytes.lock().map_err(|_| StorageError::AccountingUnavailable)?;
        Ok(ByteCount::from_bytes(*used))
    }

    pub fn put(
        &self,
        metadata: ObjectMetadata,
        bytes: &[u8],
        created_at_unix: i64,
    ) -> StorageResult<StorageReceipt> {
        if metadata.project_id.trim().is_empty() || metadata.filename.trim().is_empty() {
            return Err(StorageError::InvalidConfiguration("project id and filename are required"));
        }
        if created_at_unix <= 0 {
            return Err(StorageError::InvalidConfiguration("creation timestamp must be positive"));
        }

        let size = bytes.len() as u128;
        let mut used = self.used_bytes.lock().map_err(|_| StorageError::AccountingUnavailable)?;
        let next = used.checked_add(size).ok_or(StorageError::SizeOverflow)?;
        if next > self.quota_bytes.as_bytes() {
            return Err(StorageError::QuotaExceeded);
        }

        let object_id = ObjectId::new();
        let prefix = format!("objects/{}", object_id.0);
        let mut chunks = Vec::new();

        for (index, chunk) in bytes.chunks(self.chunk_size_bytes).enumerate() {
            let index = u32::try_from(index).map_err(|_| StorageError::SizeOverflow)?;
            let key = format!("{prefix}/chunks/{index:08}");
            if let Err(error) = self.store.put(&key, chunk) {
                let _ = self.store.delete_prefix(&prefix);
                return Err(error);
            }
            chunks.push(StoredChunk {
                index,
                size_bytes: ByteCount::from_bytes(chunk.len() as u128),
                sha256: digest(chunk),
            });
        }

        let manifest = ObjectManifest {
            object_id,
            metadata,
            size_bytes: ByteCount::from_bytes(size),
            sha256: digest(bytes),
            chunks,
            created_at_unix,
        };
        let encoded = serde_json::to_vec(&manifest)?;
        if let Err(error) = self.store.put(&format!("{prefix}/{MANIFEST_FILE}"), &encoded) {
            let _ = self.store.delete_prefix(&prefix);
            return Err(error);
        }

        *used = next;
        Ok(StorageReceipt {
            manifest,
            physical_bytes_written: ByteCount::from_bytes(size),
        })
    }

    pub fn get(&self, object_id: ObjectId) -> StorageResult<Vec<u8>> {
        let prefix = format!("objects/{}", object_id.0);
        let encoded = self.store.get(&format!("{prefix}/{MANIFEST_FILE}"))?;
        let manifest: ObjectManifest = serde_json::from_slice(&encoded)?;
        if manifest.object_id != object_id {
            return Err(StorageError::IntegrityMismatch);
        }

        let mut output = Vec::with_capacity(
            usize::try_from(manifest.size_bytes.as_bytes()).map_err(|_| StorageError::SizeOverflow)?,
        );
        for chunk in &manifest.chunks {
            let key = format!("{prefix}/chunks/{:08}", chunk.index);
            let data = self.store.get(&key)?;
            if data.len() as u128 != chunk.size_bytes.as_bytes() || digest(&data) != chunk.sha256 {
                return Err(StorageError::IntegrityMismatch);
            }
            output.extend_from_slice(&data);
        }
        if output.len() as u128 != manifest.size_bytes.as_bytes() || digest(&output) != manifest.sha256 {
            return Err(StorageError::IntegrityMismatch);
        }
        Ok(output)
    }

    pub fn delete(&self, object_id: ObjectId) -> StorageResult<()> {
        let prefix = format!("objects/{}", object_id.0);
        let encoded = self.store.get(&format!("{prefix}/{MANIFEST_FILE}"))?;
        let manifest: ObjectManifest = serde_json::from_slice(&encoded)?;
        if manifest.object_id != object_id {
            return Err(StorageError::IntegrityMismatch);
        }
        let mut used = self.used_bytes.lock().map_err(|_| StorageError::AccountingUnavailable)?;
        let remaining = used.checked_sub(manifest.size_bytes.as_bytes()).ok_or(StorageError::SizeOverflow)?;
        self.store.delete_prefix(&prefix)?;
        *used = remaining;
        Ok(())
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalFsObjectStore;
    use tempfile::tempdir;

    fn metadata() -> ObjectMetadata {
        ObjectMetadata {
            project_id: "project-001".into(),
            filename: "training-data.jsonl".into(),
            content_type: "application/jsonl".into(),
        }
    }

    #[test]
    fn writes_chunks_and_reconstructs_original_object() {
        let dir = tempdir().unwrap();
        let store = LocalFsObjectStore::new(dir.path()).unwrap();
        let engine = StorageEngine::new(store, ByteCount::from_bytes(1024), 4).unwrap();
        let data = b"apolo-data-center";
        let receipt = engine.put(metadata(), data, 1_800_000_000).unwrap();
        assert!(receipt.manifest.chunks.len() > 1);
        assert_eq!(engine.get(receipt.manifest.object_id).unwrap(), data);
        assert_eq!(engine.used_bytes().unwrap().as_bytes(), data.len() as u128);
    }

    #[test]
    fn quota_is_enforced_before_writing() {
        let dir = tempdir().unwrap();
        let store = LocalFsObjectStore::new(dir.path()).unwrap();
        let engine = StorageEngine::new(store, ByteCount::from_bytes(3), 2).unwrap();
        assert!(matches!(
            engine.put(metadata(), b"four", 1_800_000_000),
            Err(StorageError::QuotaExceeded)
        ));
        assert_eq!(engine.used_bytes().unwrap().as_bytes(), 0);
    }

    #[test]
    fn delete_releases_local_quota_accounting() {
        let dir = tempdir().unwrap();
        let store = LocalFsObjectStore::new(dir.path()).unwrap();
        let engine = StorageEngine::new(store, ByteCount::from_bytes(100), 8).unwrap();
        let receipt = engine.put(metadata(), b"payload", 1_800_000_000).unwrap();
        engine.delete(receipt.manifest.object_id).unwrap();
        assert_eq!(engine.used_bytes().unwrap().as_bytes(), 0);
        assert!(matches!(engine.get(receipt.manifest.object_id), Err(StorageError::NotFound)));
    }

    #[test]
    fn rejects_zero_chunk_size() {
        let dir = tempdir().unwrap();
        let store = LocalFsObjectStore::new(dir.path()).unwrap();
        assert!(StorageEngine::new(store, ByteCount::from_bytes(100), 0).is_err());
    }
}
