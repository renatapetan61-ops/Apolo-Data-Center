use crate::{DataFormat, IngestionError, IngestionResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct IngestionConfig {
    pub max_payload_bytes: usize,
    pub queue_capacity: usize,
    pub max_entries_per_hour: usize,
}

impl IngestionConfig {
    pub fn validate(&self) -> IngestionResult<()> {
        if self.max_payload_bytes == 0
            || self.queue_capacity == 0
            || self.max_entries_per_hour == 0
        {
            return Err(IngestionError::InvalidConfiguration);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct IngestionInput {
    pub project_id: String,
    pub filename: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
    /// Server-assigned Unix timestamp. Client timestamps must not be trusted.
    pub received_at_unix: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestionReceipt {
    pub project_id: String,
    pub filename: String,
    pub format: DataFormat,
    pub size_bytes: u64,
    pub sha256: String,
    pub queue_depth: usize,
    pub received_at_unix: i64,
}

#[derive(Debug, Clone)]
pub struct QueuedEntry {
    pub receipt: IngestionReceipt,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Default)]
struct State {
    queue: VecDeque<QueuedEntry>,
    seen: HashSet<(String, String)>,
    recent_entries: VecDeque<i64>,
    last_received_at: Option<i64>,
}

#[derive(Clone)]
pub struct IngestionService {
    config: IngestionConfig,
    state: Arc<Mutex<State>>,
}

impl IngestionService {
    pub fn new(config: IngestionConfig) -> IngestionResult<Self> {
        config.validate()?;
        Ok(Self {
            config,
            state: Arc::new(Mutex::new(State::default())),
        })
    }

    pub fn ingest(&self, input: IngestionInput) -> IngestionResult<IngestionReceipt> {
        if input.project_id.trim().is_empty() || input.filename.trim().is_empty() {
            return Err(IngestionError::MissingMetadata);
        }
        if input.received_at_unix <= 0 {
            return Err(IngestionError::InvalidConfiguration);
        }
        if input.bytes.is_empty() {
            return Err(IngestionError::EmptyPayload);
        }
        if input.bytes.len() > self.config.max_payload_bytes {
            return Err(IngestionError::PayloadTooLarge);
        }

        let format = DataFormat::from_filename(&input.filename)?;
        format.validate_payload(&input.bytes)?;
        let hash = format!("{:x}", Sha256::digest(&input.bytes));

        let mut state = self.state.lock().map_err(|_| IngestionError::StateUnavailable)?;
        if state.queue.len() >= self.config.queue_capacity {
            return Err(IngestionError::QueueFull);
        }
        if let Some(last) = state.last_received_at {
            if input.received_at_unix < last {
                return Err(IngestionError::InvalidConfiguration);
            }
        }

        let window_start = input.received_at_unix.saturating_sub(3_600);
        while state.recent_entries.front().is_some_and(|timestamp| *timestamp <= window_start) {
            state.recent_entries.pop_front();
        }
        if state.recent_entries.len() >= self.config.max_entries_per_hour {
            return Err(IngestionError::RateLimitExceeded);
        }

        let dedupe_key = (input.project_id.clone(), hash.clone());
        if state.seen.contains(&dedupe_key) {
            return Err(IngestionError::Duplicate);
        }

        let receipt = IngestionReceipt {
            project_id: input.project_id,
            filename: input.filename,
            format,
            size_bytes: input.bytes.len() as u64,
            sha256: hash,
            queue_depth: state.queue.len() + 1,
            received_at_unix: input.received_at_unix,
        };
        state.queue.push_back(QueuedEntry {
            receipt: receipt.clone(),
            content_type: input.content_type,
            bytes: input.bytes,
        });
        state.seen.insert(dedupe_key);
        state.recent_entries.push_back(input.received_at_unix);
        state.last_received_at = Some(input.received_at_unix);
        Ok(receipt)
    }

    pub fn dequeue(&self) -> IngestionResult<Option<QueuedEntry>> {
        let mut state = self.state.lock().map_err(|_| IngestionError::StateUnavailable)?;
        Ok(state.queue.pop_front())
    }

    pub fn queue_depth(&self) -> IngestionResult<usize> {
        let state = self.state.lock().map_err(|_| IngestionError::StateUnavailable)?;
        Ok(state.queue.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(capacity: usize, rate: usize) -> IngestionService {
        IngestionService::new(IngestionConfig {
            max_payload_bytes: 1024,
            queue_capacity: capacity,
            max_entries_per_hour: rate,
        }).unwrap()
    }

    fn input(project: &str, filename: &str, bytes: &[u8], timestamp: i64) -> IngestionInput {
        IngestionInput {
            project_id: project.into(),
            filename: filename.into(),
            content_type: "application/json".into(),
            bytes: bytes.to_vec(),
            received_at_unix: timestamp,
        }
    }

    #[test]
    fn accepts_json_and_dequeues_payload() {
        let service = service(4, 10);
        let receipt = service.ingest(input("p1", "data.json", br#"{"ok":true}"#, 100)).unwrap();
        assert_eq!(receipt.format, DataFormat::Json);
        assert_eq!(service.dequeue().unwrap().unwrap().bytes, br#"{"ok":true}"#);
    }

    #[test]
    fn rejects_duplicate_within_project_but_not_across_projects() {
        let service = service(4, 10);
        service.ingest(input("p1", "a.json", br#"{"x":1}"#, 100)).unwrap();
        assert_eq!(service.ingest(input("p1", "b.json", br#"{"x":1}"#, 101)).unwrap_err(), IngestionError::Duplicate);
        assert!(service.ingest(input("p2", "b.json", br#"{"x":1}"#, 102)).is_ok());
    }

    #[test]
    fn queue_capacity_applies_backpressure() {
        let service = service(1, 10);
        service.ingest(input("p1", "a.json", br#"{"x":1}"#, 100)).unwrap();
        assert_eq!(service.ingest(input("p1", "b.json", br#"{"x":2}"#, 101)).unwrap_err(), IngestionError::QueueFull);
    }

    #[test]
    fn enforces_hourly_entry_limit() {
        let service = service(10, 1);
        service.ingest(input("p1", "a.json", br#"{"x":1}"#, 100)).unwrap();
        assert_eq!(service.ingest(input("p1", "b.json", br#"{"x":2}"#, 101)).unwrap_err(), IngestionError::RateLimitExceeded);
    }

    #[test]
    fn rejects_invalid_json_payload() {
        let service = service(4, 10);
        assert_eq!(service.ingest(input("p1", "bad.json", b"not-json", 100)).unwrap_err(), IngestionError::InvalidPayload);
    }

    #[test]
    fn rejects_payload_above_configured_limit() {
        let service = service(4, 10);
        let mut item = input("p1", "data.txt", b"abc", 100);
        item.bytes = vec![b'a'; 1025];
        assert_eq!(service.ingest(item).unwrap_err(), IngestionError::PayloadTooLarge);
    }
}
