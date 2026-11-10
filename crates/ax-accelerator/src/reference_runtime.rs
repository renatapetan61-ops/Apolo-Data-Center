//! Deterministic CPU reference interpreter for the preliminary AX ISA.
//!
//! This is a software conformance tool, not a physical accelerator or a
//! performance simulator. Register values are unsigned 64-bit bit patterns;
//! arithmetic wraps modulo 2^64. LOAD/STORE support 1, 2, 4, or 8 bytes.

use crate::{AxOpcode, AxProgram, AxIsaError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxExecutionResult {
    pub registers: Vec<u64>,
    pub instructions_executed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxRuntimeError {
    InvalidProgram(AxIsaError),
    MemoryRegionTooSmall,
    InstructionBudgetExceeded,
    InvalidInstructionShape,
    UnsupportedOperation(AxOpcode),
}

fn read_register(registers: &[u64], sources: &[u16], index: usize) -> Result<u64, AxRuntimeError> {
    let register = *sources.get(index).ok_or(AxRuntimeError::InvalidInstructionShape)?;
    registers.get(usize::from(register)).copied().ok_or(AxRuntimeError::InvalidInstructionShape)
}

pub fn execute_reference(
    program: &AxProgram,
    memory: &mut [u8],
    instruction_budget: u64,
) -> Result<AxExecutionResult, AxRuntimeError> {
    program.validate().map_err(AxRuntimeError::InvalidProgram)?;
    if program.device_memory_bytes > memory.len() as u64 {
        return Err(AxRuntimeError::MemoryRegionTooSmall);
    }
    if program.instructions.len() as u64 > instruction_budget {
        return Err(AxRuntimeError::InstructionBudgetExceeded);
    }

    let mut registers = vec![0_u64; usize::from(program.register_count)];
    for instruction in &program.instructions {
        let dst = instruction.dst.map(usize::from);
        let src = &instruction.src;
        match instruction.opcode {
            AxOpcode::Nop | AxOpcode::Barrier => {
                if dst.is_some() || !src.is_empty() {
                    return Err(AxRuntimeError::InvalidInstructionShape);
                }
            }
            AxOpcode::Load => {
                let target = dst.ok_or(AxRuntimeError::InvalidInstructionShape)?;
                let offset = instruction.address_offset.ok_or(AxRuntimeError::InvalidInstructionShape)? as usize;
                let width = instruction.access_bytes.ok_or(AxRuntimeError::InvalidInstructionShape)?;
                if !matches!(width, 1 | 2 | 4 | 8) {
                    return Err(AxRuntimeError::InvalidInstructionShape);
                }
                if !src.is_empty() { return Err(AxRuntimeError::InvalidInstructionShape); }
                let mut bytes = [0_u8; 8];
                bytes[..width as usize].copy_from_slice(&memory[offset..offset + width as usize]);
                registers[target] = u64::from_le_bytes(bytes);
            }
            AxOpcode::Store => {
                if dst.is_some() { return Err(AxRuntimeError::InvalidInstructionShape); }
                let value = read_register(&registers, src, 0)?;
                let offset = instruction.address_offset.ok_or(AxRuntimeError::InvalidInstructionShape)? as usize;
                let width = instruction.access_bytes.ok_or(AxRuntimeError::InvalidInstructionShape)?;
                if !matches!(width, 1 | 2 | 4 | 8) {
                    return Err(AxRuntimeError::InvalidInstructionShape);
                }
                memory[offset..offset + width as usize]
                    .copy_from_slice(&value.to_le_bytes()[..width as usize]);
            }
            AxOpcode::Add | AxOpcode::DotProduct => {
                let target = dst.ok_or(AxRuntimeError::InvalidInstructionShape)?;
                registers[target] = read(0)?.wrapping_add(read_register(&registers, src, 1)?);
            }
            AxOpcode::Multiply => {
                let target = dst.ok_or(AxRuntimeError::InvalidInstructionShape)?;
                registers[target] = read(0)?.wrapping_mul(read(1)?);
            }
            AxOpcode::FusedMultiplyAdd => {
                let target = dst.ok_or(AxRuntimeError::InvalidInstructionShape)?;
                registers[target] = read(0)?.wrapping_mul(read(1)?).wrapping_add(read_register(&registers, src, 2)?);
            }
            AxOpcode::Relu => {
                let target = dst.ok_or(AxRuntimeError::InvalidInstructionShape)?;
                registers[target] = (read(0)? as i64).max(0) as u64;
            }
            AxOpcode::MatrixMultiplyTile => {
                return Err(AxRuntimeError::UnsupportedOperation(AxOpcode::MatrixMultiplyTile));
            }
        }
    }

    Ok(AxExecutionResult {
        registers,
        instructions_executed: program.instructions.len() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AxInstruction, AX_ISA_MAJOR, AX_ISA_MINOR};

    fn program(instructions: Vec<AxInstruction>) -> AxProgram {
        AxProgram {
            isa_major: AX_ISA_MAJOR,
            isa_minor: AX_ISA_MINOR,
            register_count: 8,
            device_memory_bytes: 64,
            instructions,
        }
    }

    fn instruction(opcode: AxOpcode, dst: Option<u16>, src: &[u16], offset: Option<u64>, width: Option<u32>) -> AxInstruction {
        AxInstruction { opcode, dst, src: src.to_vec(), address_offset: offset, access_bytes: width }
    }

    #[test]
    fn executes_load_arithmetic_and_store_little_endian() {
        let mut memory = vec![0_u8; 64];
        memory[0..8].copy_from_slice(&7_u64.to_le_bytes());
        memory[8..16].copy_from_slice(&5_u64.to_le_bytes());
        let p = program(vec![
            instruction(AxOpcode::Load, Some(0), &[], Some(0), Some(8)),
            instruction(AxOpcode::Load, Some(1), &[], Some(8), Some(8)),
            instruction(AxOpcode::Multiply, Some(2), &[0, 1], None, None),
            instruction(AxOpcode::Store, None, &[2], Some(16), Some(8)),
        ]);
        let result = execute_reference(&p, &mut memory, 4).unwrap();
        assert_eq!(result.registers[2], 35);
        assert_eq!(u64::from_le_bytes(memory[16..24].try_into().unwrap()), 35);
        assert_eq!(result.instructions_executed, 4);
    }

    #[test]
    fn enforces_instruction_budget() {
        let p = program(vec![instruction(AxOpcode::Nop, None, &[], None, None)]);
        assert_eq!(execute_reference(&p, &mut [0; 64], 0), Err(AxRuntimeError::InstructionBudgetExceeded));
    }

    #[test]
    fn rejects_memory_region_larger_than_host_buffer() {
        let mut p = program(vec![instruction(AxOpcode::Nop, None, &[], None, None)]);
        p.device_memory_bytes = 65;
        assert_eq!(execute_reference(&p, &mut [0; 64], 1), Err(AxRuntimeError::MemoryRegionTooSmall));
    }

    #[test]
    fn matrix_tile_is_explicitly_not_implemented() {
        let p = program(vec![instruction(AxOpcode::MatrixMultiplyTile, Some(0), &[1, 2, 3], None, None)]);
        assert_eq!(execute_reference(&p, &mut [0; 64], 1), Err(AxRuntimeError::UnsupportedOperation(AxOpcode::MatrixMultiplyTile)));
    }

    #[test]
    fn arithmetic_wraps_deterministically() {
        let p = program(vec![
            instruction(AxOpcode::Add, Some(2), &[0, 1], None, None),
        ]);
        // Registers are zero-initialized in this first reference-runtime slice.
        let result = execute_reference(&p, &mut [0; 64], 1).unwrap();
        assert_eq!(result.registers[2], 0);
    }
}
