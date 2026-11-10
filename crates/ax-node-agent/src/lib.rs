//! APOLO AX Node Agent control-plane boundary.
//!
//! This crate implements the authenticated registration/heartbeat state machine.
//! It intentionally does not open network sockets or claim hardware attestation;
//! a transport adapter and a platform-specific probe are separate components.

use apolo_ax_core::{NodeInventory, NodeState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub agent_id: String,
    pub credential: String,
    pub inventory: NodeInventory,
    pub observed_at_unix: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentHeartbeat {
    pub agent_id: String,
    pub sequence: u64,
    pub observed_at_unix: i64,
    pub inventory: NodeInventory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSnapshot {
    pub agent_id: String,
    pub last_sequence: u64,
    pub last_seen_unix: i64,
    pub inventory: NodeInventory,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AgentError {
    #[error("agent identifier is invalid")]
    InvalidAgentId,
    #[error("agent credential is invalid")]
    Unauthorized,
    #[error("agent is already registered")]
    AlreadyRegistered,
    #[error("agent is not registered")]
    NotRegistered,
    #[error("inventory does not belong to this agent")]
    InventoryIdentityMismatch,
    #[error("inventory validation failed")]
    InvalidInventory,
    #[error("heartbeat sequence is stale or replayed")]
    ReplayDetected,
    #[error("agent timestamp must increase monotonically")]
    StaleTimestamp,
    #[error("agent registry lock is poisoned")]
    LockPoisoned,
}

pub type AgentResult<T> = Result<T, AgentError>;

/// Implement this using a real credential store and constant-time secret checks.
/// Never log or persist the presented credential.
pub trait CredentialVerifier: Send + Sync {
    fn verify(&self, agent_id: &str, credential: &str) -> bool;
}

#[derive(Debug, Default)]
struct RegistryState {
    agents: BTreeMap<String, AgentSnapshot>,
}

/// In-process registry. A durable database and network/mTLS adapter are not
/// included in this first slice.
pub struct AgentRegistry {
    verifier: Arc<dyn CredentialVerifier>,
    heartbeat_ttl_seconds: i64,
    state: Mutex<RegistryState>,
}

impl AgentRegistry {
    pub fn new(
        verifier: Arc<dyn CredentialVerifier>,
        heartbeat_ttl_seconds: i64,
    ) -> AgentResult<Self> {
        if heartbeat_ttl_seconds <= 0 {
            return Err(AgentError::StaleTimestamp);
        }
        Ok(Self {
            verifier,
            heartbeat_ttl_seconds,
            state: Mutex::new(RegistryState::default()),
        })
    }

    pub fn register(&self, request: AgentRegistration) -> AgentResult<AgentSnapshot> {
        if request.agent_id.trim().is_empty() {
            return Err(AgentError::InvalidAgentId);
        }
        if !self.verifier.verify(&request.agent_id, &request.credential) {
            return Err(AgentError::Unauthorized);
        }
        if request.inventory.node_id != request.agent_id {
            return Err(AgentError::InventoryIdentityMismatch);
        }
        if request.observed_at_unix <= 0 || request.inventory.validate().is_err() {
            return Err(AgentError::InvalidInventory);
        }

        let mut inventory = request.inventory;
        // An agent's self-report is not hardware attestation.
        inventory.hardware_verified = false;
        inventory.state = NodeState::Registering;
        let snapshot = AgentSnapshot {
            agent_id: request.agent_id.clone(),
            last_sequence: 0,
            last_seen_unix: request.observed_at_unix,
            inventory,
        };

        let mut state = self.state.lock().map_err(|_| AgentError::LockPoisoned)?;
        if state.agents.contains_key(&request.agent_id) {
            return Err(AgentError::AlreadyRegistered);
        }
        state.agents.insert(request.agent_id, snapshot.clone());
        Ok(snapshot)
    }

    pub fn heartbeat(&self, request: AgentHeartbeat) -> AgentResult<AgentSnapshot> {
        if request.inventory.node_id != request.agent_id {
            return Err(AgentError::InventoryIdentityMismatch);
        }
        if request.observed_at_unix <= 0 || request.inventory.validate().is_err() {
            return Err(AgentError::InvalidInventory);
        }

        let mut state = self.state.lock().map_err(|_| AgentError::LockPoisoned)?;
        let current = state.agents.get(&request.agent_id).ok_or(AgentError::NotRegistered)?;
        if request.sequence <= current.last_sequence {
            return Err(AgentError::ReplayDetected);
        }
        if request.observed_at_unix <= current.last_seen_unix {
            return Err(AgentError::StaleTimestamp);
        }

        let mut inventory = request.inventory;
        // Trust is granted only by a future independent attestation service.
        inventory.hardware_verified = false;
        inventory.state = NodeState::Healthy;
        let snapshot = AgentSnapshot {
            agent_id: request.agent_id.clone(),
            last_sequence: request.sequence,
            last_seen_unix: request.observed_at_unix,
            inventory,
        };
        state.agents.insert(request.agent_id, snapshot.clone());
        Ok(snapshot)
    }

    pub fn get(&self, agent_id: &str) -> AgentResult<AgentSnapshot> {
        let state = self.state.lock().map_err(|_| AgentError::LockPoisoned)?;
        state.agents.get(agent_id).cloned().ok_or(AgentError::NotRegistered)
    }

    /// Marks agents offline after their heartbeat lease expires.
    /// Returns the number of agents transitioned to offline.
    pub fn expire_stale(&self, now_unix: i64) -> AgentResult<usize> {
        if now_unix <= 0 {
            return Err(AgentError::StaleTimestamp);
        }
        let mut state = self.state.lock().map_err(|_| AgentError::LockPoisoned)?;
        let mut expired = 0;
        for agent in state.agents.values_mut() {
            if now_unix.saturating_sub(agent.last_seen_unix) > self.heartbeat_ttl_seconds
                && agent.inventory.state != NodeState::Offline
            {
                agent.inventory.state = NodeState::Offline;
                expired += 1;
            }
        }
        Ok(expired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apolo_ax_core::ByteCount;

    struct TestVerifier;
    impl CredentialVerifier for TestVerifier {
        fn verify(&self, agent_id: &str, credential: &str) -> bool {
            agent_id == "node-1" && credential == "test-only-secret"
        }
    }

    fn registry() -> AgentRegistry {
        AgentRegistry::new(Arc::new(TestVerifier), 30).unwrap()
    }

    fn inventory() -> NodeInventory {
        NodeInventory {
            node_id: "node-1".into(),
            state: NodeState::Healthy,
            cpu_cores: 4,
            memory_total_bytes: ByteCount::from_bytes(4096),
            memory_used_bytes: ByteCount::from_bytes(512),
            storage_total_bytes: ByteCount::from_bytes(8192),
            storage_used_bytes: ByteCount::from_bytes(1024),
            storage_reserved_bytes: ByteCount::ZERO,
            network_mbps: 1000,
            accelerators: vec![],
            observed_at_unix: 1_800_000_000,
            hardware_verified: true,
        }
    }

    fn registration() -> AgentRegistration {
        AgentRegistration {
            agent_id: "node-1".into(),
            credential: "test-only-secret".into(),
            inventory: inventory(),
            observed_at_unix: 1_800_000_000,
        }
    }

    #[test]
    fn registration_does_not_trust_agent_hardware_claim() {
        let snapshot = registry().register(registration()).unwrap();
        assert!(!snapshot.inventory.hardware_verified);
        assert_eq!(snapshot.inventory.state, NodeState::Registering);
    }

    #[test]
    fn invalid_credential_is_rejected() {
        let mut request = registration();
        request.credential = "wrong".into();
        assert_eq!(registry().register(request), Err(AgentError::Unauthorized));
    }

    #[test]
    fn heartbeat_advances_sequence_and_keeps_hardware_unverified() {
        let registry = registry();
        registry.register(registration()).unwrap();
        let snapshot = registry.heartbeat(AgentHeartbeat {
            agent_id: "node-1".into(),
            sequence: 1,
            observed_at_unix: 1_800_000_001,
            inventory: inventory(),
        }).unwrap();
        assert_eq!(snapshot.last_sequence, 1);
        assert_eq!(snapshot.inventory.state, NodeState::Healthy);
        assert!(!snapshot.inventory.hardware_verified);
    }

    #[test]
    fn replayed_heartbeat_is_rejected() {
        let registry = registry();
        registry.register(registration()).unwrap();
        let request = AgentHeartbeat {
            agent_id: "node-1".into(),
            sequence: 1,
            observed_at_unix: 1_800_000_001,
            inventory: inventory(),
        };
        registry.heartbeat(request.clone()).unwrap();
        assert_eq!(registry.heartbeat(request), Err(AgentError::ReplayDetected));
    }

    #[test]
    fn stale_agent_is_marked_offline() {
        let registry = registry();
        registry.register(registration()).unwrap();
        assert_eq!(registry.expire_stale(1_800_000_030).unwrap(), 0);
        assert_eq!(registry.expire_stale(1_800_000_031).unwrap(), 1);
        assert_eq!(registry.get("node-1").unwrap().inventory.state, NodeState::Offline);
    }

    #[test]
    fn mismatched_inventory_identity_is_rejected() {
        let registry = registry();
        let mut request = registration();
        request.inventory.node_id = "another-node".into();
        assert_eq!(registry.register(request), Err(AgentError::InventoryIdentityMismatch));
    }
}
