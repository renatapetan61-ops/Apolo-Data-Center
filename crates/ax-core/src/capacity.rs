use serde::{Deserialize, Serialize};
use crate::{AxError, AxResult};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ByteCount(u64);
impl ByteCount {
    pub const ZERO: Self = Self(0);
    pub const fn from_bytes(bytes: u64) -> Self { Self(bytes) }
    pub const fn as_bytes(self) -> u64 { self.0 }
    pub fn checked_add(self, other: Self) -> AxResult<Self> {
        self.0.checked_add(other.0).map(Self).ok_or(AxError::CapacityOverflow)
    }
    pub fn checked_sub(self, other: Self) -> AxResult<Self> {
        self.0.checked_sub(other.0).map(Self).ok_or(AxError::InvalidCapacity("used capacity exceeds total capacity"))
    }
}

/// Logical quota is not evidence of physical storage capacity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacitySnapshot {
    pub logical_quota_bytes: ByteCount,
    pub physical_total_bytes: ByteCount,
    pub physical_used_bytes: ByteCount,
    pub physical_reserved_bytes: ByteCount,
    pub hardware_verified: bool,
}
impl CapacitySnapshot {
    pub fn physical_available_bytes(&self) -> AxResult<ByteCount> {
        self.physical_total_bytes.checked_sub(self.physical_used_bytes.checked_add(self.physical_reserved_bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logical_quota_does_not_inflate_physical_capacity() {
        let s = CapacitySnapshot { logical_quota_bytes: ByteCount::from_bytes(10_000), physical_total_bytes: ByteCount::from_bytes(1_000), physical_used_bytes: ByteCount::from_bytes(200), physical_reserved_bytes: ByteCount::from_bytes(100), hardware_verified: true };
        assert_eq!(s.physical_available_bytes().unwrap().as_bytes(), 700);
    }
    #[test]
    fn rejects_overcommitted_physical_inventory() {
        let s = CapacitySnapshot { logical_quota_bytes: ByteCount::from_bytes(1_000), physical_total_bytes: ByteCount::from_bytes(100), physical_used_bytes: ByteCount::from_bytes(80), physical_reserved_bytes: ByteCount::from_bytes(30), hardware_verified: true };
        assert!(s.physical_available_bytes().is_err());
    }
}
