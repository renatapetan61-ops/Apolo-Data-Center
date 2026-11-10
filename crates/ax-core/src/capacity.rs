use crate::{AxError, AxResult};
use serde::{Deserialize, Serialize};

/// Unsigned 128-bit byte count supports exabyte-scale logical quotas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ByteCount(u128);

impl ByteCount {
    pub const ZERO: Self = Self(0);
    pub const fn from_bytes(bytes: u128) -> Self { Self(bytes) }
    pub const fn as_bytes(self) -> u128 { self.0 }

    pub fn checked_add(self, other: Self) -> AxResult<Self> {
        self.0.checked_add(other.0).map(Self).ok_or(AxError::CapacityOverflow)
    }

    pub fn checked_sub(self, other: Self) -> AxResult<Self> {
        self.0.checked_sub(other.0)
            .map(Self)
            .ok_or(AxError::InvalidCapacity("used capacity exceeds total capacity"))
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
        if !self.hardware_verified {
            return Err(AxError::HardwareUnverified);
        }
        self.physical_total_bytes.checked_sub(
            self.physical_used_bytes.checked_add(self.physical_reserved_bytes)?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(verified: bool, total: u128, used: u128, reserved: u128) -> CapacitySnapshot {
        CapacitySnapshot {
            logical_quota_bytes: ByteCount::from_bytes(10_000),
            physical_total_bytes: ByteCount::from_bytes(total),
            physical_used_bytes: ByteCount::from_bytes(used),
            physical_reserved_bytes: ByteCount::from_bytes(reserved),
            hardware_verified: verified,
        }
    }

    #[test]
    fn logical_quota_does_not_inflate_physical_capacity() {
        assert_eq!(
            snapshot(true, 1_000, 200, 100).physical_available_bytes().unwrap().as_bytes(),
            700
        );
    }

    #[test]
    fn rejects_overcommitted_physical_inventory() {
        assert!(snapshot(true, 100, 80, 30).physical_available_bytes().is_err());
    }

    #[test]
    fn rejects_unverified_physical_inventory() {
        assert_eq!(
            snapshot(false, 1_000, 100, 100).physical_available_bytes(),
            Err(AxError::HardwareUnverified)
        );
    }

    #[test]
    fn supports_exabyte_scale_byte_counts() {
        let one_exabyte = ByteCount::from_bytes(1_000_000_000_000_000_000);
        assert_eq!(one_exabyte.as_bytes(), 1_000_000_000_000_000_000);
    }
}
