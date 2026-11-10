use crate::{AxError, AxResult, AcceleratorKind, ByteCount};
use serde::{Deserialize, Serialize};

/// Commercial tier labels are logical product classes, not hardware claims.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileCode {
    Ax1,
    Ax8,
    Ax128,
    Ax10000,
    Custom(String),
}

impl ProfileCode {
    pub const fn scale_units(&self) -> Option<u32> {
        match self {
            Self::Ax1 => Some(1),
            Self::Ax8 => Some(8),
            Self::Ax128 => Some(128),
            Self::Ax10000 => Some(10_000),
            Self::Custom(_) => None,
        }
    }

    pub fn validate(&self) -> AxResult<()> {
        if let Self::Custom(name) = self {
            if name.trim().is_empty() {
                return Err(AxError::InvalidIdentifier("custom profile name cannot be empty"));
            }
        }
        Ok(())
    }
}

/// Explicit resource demand. A tier alone never implies a physical device count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRequest {
    pub cpu_cores: u32,
    pub memory_bytes: ByteCount,
    pub accelerator_count: u32,
    pub accelerator_kind: Option<AcceleratorKind>,
    pub storage_bytes: ByteCount,
    pub network_mbps: u64,
}

impl ResourceRequest {
    pub fn validate(&self) -> AxResult<()> {
        if self.cpu_cores == 0 {
            return Err(AxError::InvalidResource("cpu_cores must be positive"));
        }
        if self.memory_bytes == ByteCount::ZERO {
            return Err(AxError::InvalidResource("memory_bytes must be positive"));
        }
        if self.accelerator_count > 0 && self.accelerator_kind.is_none() {
            return Err(AxError::InvalidResource(
                "accelerator_kind is required when accelerator_count is positive",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxProfile {
    pub code: ProfileCode,
    pub request: ResourceRequest,
}

impl AxProfile {
    pub fn new(code: ProfileCode, request: ResourceRequest) -> AxResult<Self> {
        code.validate()?;
        request.validate()?;
        Ok(Self { code, request })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(accelerator_count: u32, accelerator_kind: Option<AcceleratorKind>) -> ResourceRequest {
        ResourceRequest {
            cpu_cores: 4,
            memory_bytes: ByteCount::from_bytes(8 * 1024 * 1024 * 1024),
            accelerator_count,
            accelerator_kind,
            storage_bytes: ByteCount::from_bytes(100 * 1024 * 1024 * 1024),
            network_mbps: 1_000,
        }
    }

    #[test]
    fn tier_labels_have_only_declared_logical_scale() {
        assert_eq!(ProfileCode::Ax10000.scale_units(), Some(10_000));
        assert_eq!(ProfileCode::Custom("research".into()).scale_units(), None);
    }

    #[test]
    fn profile_requires_explicit_hardware_demand() {
        assert!(AxProfile::new(ProfileCode::Ax1, request(0, None)).is_ok());
        assert!(AxProfile::new(ProfileCode::Ax8, request(1, None)).is_err());
    }

    #[test]
    fn empty_custom_profile_name_is_rejected() {
        assert!(ProfileCode::Custom("  ".into()).validate().is_err());
    }
}
