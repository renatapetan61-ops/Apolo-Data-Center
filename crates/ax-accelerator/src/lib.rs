//! APOLO AX accelerator architecture contracts.\n\npub mod reference_runtime;\npub mod bytecode;
//!
//! This crate defines an original, versioned command model and backend boundary.
//! It does not emulate physical performance or claim to drive hardware yet.

use apolo_ax_core::ResourceId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const AX_ISA_MAJOR: u16 = 0;
pub const AX_ISA_MINOR: u16 = 1;
pub const AX_MAX_REGISTERS: u16 = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxOpcode {
    Nop,
    Load,
    Store,
    Add,
    Multiply,
    FusedMultiplyAdd,
    DotProduct,
    Relu,
    MatrixMultiplyTile,
    Barrier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxInstruction {
    pub opcode: AxOpcode,
    pub dst: Option<u16>,
    pub src: Vec<u16>,
    /// Byte offset into the job's explicitly registered device-memory region.
    pub address_offset: Option<u64>,
    /// Number of bytes accessed by Load/Store.
    pub access_bytes: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxProgram {
    pub isa_major: u16,
    pub isa_minor: u16,
    pub register_count: u16,
    pub device_memory_bytes: u64,
    pub instructions: Vec<AxInstruction>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AxIsaError {
    #[error("unsupported AX ISA version")]
    UnsupportedIsa,
    #[error("register count is outside the supported range")]
    InvalidRegisterCount,
    #[error("program contains no instructions")]
    EmptyProgram,
    #[error("instruction references a register outside the declared file")]
    RegisterOutOfRange,
    #[error("memory instruction is missing its address or access size")]
    MissingMemoryOperand,
    #[error("memory access is outside the declared device memory region")]
    MemoryOutOfBounds,
    #[error("non-memory instruction contains memory operands")]
    UnexpectedMemoryOperand,
    #[error("opcode has an invalid register operand count")]
    InvalidOperandCount,
    #[error("device does not advertise the required ISA version")]
    DeviceIsaMismatch,
    #[error("device is not verified and healthy")]
    DeviceNotTrusted,
    #[error("backend operation failed: {0}")]
    Backend(String),
}

pub type AxIsaResult<T> = Result<T, AxIsaError>;

impl AxProgram {
    pub fn validate(&self) -> AxIsaResult<()> {
        if self.isa_major != AX_ISA_MAJOR || self.isa_minor > AX_ISA_MINOR {
            return Err(AxIsaError::UnsupportedIsa);
        }
        if self.register_count == 0 || self.register_count > AX_MAX_REGISTERS {
            return Err(AxIsaError::InvalidRegisterCount);
        }
        if self.instructions.is_empty() {
            return Err(AxIsaError::EmptyProgram);
        }

        for instruction in &self.instructions {
            if let Some(dst) = instruction.dst {
                if dst >= self.register_count {
                    return Err(AxIsaError::RegisterOutOfRange);
                }
            }
            for register in &instruction.src {
                if *register >= self.register_count {
                    return Err(AxIsaError::RegisterOutOfRange);
                }
            }

            let (expected_sources, destination_required) = match instruction.opcode {
                AxOpcode::Nop | AxOpcode::Barrier => (0, false),
                AxOpcode::Load => (0, true),
                AxOpcode::Store => (1, false),
                AxOpcode::Relu => (1, true),
                AxOpcode::Add | AxOpcode::Multiply | AxOpcode::DotProduct => (2, true),
                AxOpcode::FusedMultiplyAdd | AxOpcode::MatrixMultiplyTile => (3, true),
            };
            if instruction.src.len() != expected_sources
                || instruction.dst.is_some() != destination_required
            {
                return Err(AxIsaError::InvalidOperandCount);
            }

            if matches!(instruction.opcode, AxOpcode::Load | AxOpcode::Store) {
                let offset = instruction.address_offset.ok_or(AxIsaError::MissingMemoryOperand)?;
                let bytes = instruction.access_bytes.ok_or(AxIsaError::MissingMemoryOperand)?;
                if bytes == 0 {
                    return Err(AxIsaError::MissingMemoryOperand);
                }
                let end = offset.checked_add(u64::from(bytes)).ok_or(AxIsaError::MemoryOutOfBounds)?;
                if end > self.device_memory_bytes {
                    return Err(AxIsaError::MemoryOutOfBounds);
                }
            } else if instruction.address_offset.is_some() || instruction.access_bytes.is_some() {
                return Err(AxIsaError::UnexpectedMemoryOperand);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxDeviceDescriptor {
    pub resource_id: ResourceId,
    pub node_id: String,
    pub architecture: String,
    pub isa_major: u16,
    pub isa_minor: u16,
    pub register_count: u16,
    pub memory_bytes: u64,
    pub hardware_verified: bool,
    pub healthy: bool,
}

impl AxDeviceDescriptor {
    pub fn validate_program(&self, program: &AxProgram) -> AxIsaResult<()> {
        if !self.hardware_verified || !self.healthy {
            return Err(AxIsaError::DeviceNotTrusted);
        }
        if self.isa_major != program.isa_major || self.isa_minor < program.isa_minor {
            return Err(AxIsaError::DeviceIsaMismatch);
        }
        if program.register_count > self.register_count || program.device_memory_bytes > self.memory_bytes {
            return Err(AxIsaError::InvalidRegisterCount);
        }
        program.validate()
    }
}

/// Hardware/driver boundary. A future FPGA/ASIC driver implements this trait.
/// The core never reports simulated execution as physical accelerator work.
pub trait AcceleratorBackend: Send + Sync {
    fn descriptor(&self) -> &AxDeviceDescriptor;
    fn submit(&self, program: &AxProgram) -> AxIsaResult<AxSubmission>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxSubmission {
    pub submission_id: String,
    pub accepted: bool,
    pub hardware_timestamp_ns: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device() -> AxDeviceDescriptor {
        AxDeviceDescriptor {
            resource_id: ResourceId::new(),
            node_id: "ax-node-test".into(),
            architecture: "APOLO-AX-Proto".into(),
            isa_major: AX_ISA_MAJOR,
            isa_minor: AX_ISA_MINOR,
            register_count: 64,
            memory_bytes: 4096,
            hardware_verified: true,
            healthy: true,
        }
    }

    fn program() -> AxProgram {
        AxProgram {
            isa_major: AX_ISA_MAJOR,
            isa_minor: AX_ISA_MINOR,
            register_count: 16,
            device_memory_bytes: 1024,
            instructions: vec![
                AxInstruction { opcode: AxOpcode::Load, dst: Some(0), src: vec![], address_offset: Some(0), access_bytes: Some(16) },
                AxInstruction { opcode: AxOpcode::Add, dst: Some(2), src: vec![0, 1], address_offset: None, access_bytes: None },
                AxInstruction { opcode: AxOpcode::Store, dst: None, src: vec![2], address_offset: Some(32), access_bytes: Some(16) },
            ],
        }
    }

    #[test]
    fn valid_program_passes_isa_validation() { assert!(program().validate().is_ok()); }

    #[test]
    fn out_of_bounds_memory_is_rejected() {
        let mut p = program();
        p.instructions[0].address_offset = Some(1020);
        assert_eq!(p.validate(), Err(AxIsaError::MemoryOutOfBounds));
    }

    #[test]
    fn invalid_register_is_rejected() {
        let mut p = program();
        p.instructions[1].src[0] = 16;
        assert_eq!(p.validate(), Err(AxIsaError::RegisterOutOfRange));
    }

    #[test]
    fn unsupported_isa_is_rejected() {
        let mut p = program();
        p.isa_major = 99;
        assert_eq!(p.validate(), Err(AxIsaError::UnsupportedIsa));
    }

    #[test]
    fn unverified_device_is_never_accepted() {
        let mut d = device();
        d.hardware_verified = false;
        assert_eq!(d.validate_program(&program()), Err(AxIsaError::DeviceNotTrusted));
    }

    #[test]
    fn device_must_support_program_memory_and_registers() {
        let mut d = device();
        d.register_count = 8;
        assert_eq!(d.validate_program(&program()), Err(AxIsaError::InvalidRegisterCount));
    }
}
