//! Experimental AX bytecode container (little-endian, v0 format).
//! The wire format is not ABI-frozen and must not be used as a stable external contract.

use crate::{AxInstruction, AxOpcode, AxProgram, AX_ISA_MAJOR, AX_ISA_MINOR};

const MAGIC: &[u8; 4] = b"AXBC";
const MAX_BYTECODE_BYTES: usize = 64 * 1024 * 1024;
const MAX_INSTRUCTIONS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxBytecodeError {
    TooLarge,
    Truncated,
    BadMagic,
    UnsupportedOpcode(u8),
    InvalidEncoding,
    InvalidProgram,
}

fn opcode_to_byte(op: AxOpcode) -> u8 {
    match op {
        AxOpcode::Nop => 0, AxOpcode::Load => 1, AxOpcode::Store => 2,
        AxOpcode::Add => 3, AxOpcode::Multiply => 4,
        AxOpcode::FusedMultiplyAdd => 5, AxOpcode::DotProduct => 6,
        AxOpcode::Relu => 7, AxOpcode::MatrixMultiplyTile => 8,
        AxOpcode::Barrier => 9,
    }
}

fn byte_to_opcode(value: u8) -> Result<AxOpcode, AxBytecodeError> {
    match value {
        0 => Ok(AxOpcode::Nop), 1 => Ok(AxOpcode::Load), 2 => Ok(AxOpcode::Store),
        3 => Ok(AxOpcode::Add), 4 => Ok(AxOpcode::Multiply),
        5 => Ok(AxOpcode::FusedMultiplyAdd), 6 => Ok(AxOpcode::DotProduct),
        7 => Ok(AxOpcode::Relu), 8 => Ok(AxOpcode::MatrixMultiplyTile),
        9 => Ok(AxOpcode::Barrier), other => Err(AxBytecodeError::UnsupportedOpcode(other)),
    }
}

fn take<const N: usize>(bytes: &[u8], cursor: &mut usize) -> Result<[u8; N], AxBytecodeError> {
    let end = cursor.checked_add(N).ok_or(AxBytecodeError::Truncated)?;
    let slice = bytes.get(*cursor..end).ok_or(AxBytecodeError::Truncated)?;
    *cursor = end;
    slice.try_into().map_err(|_| AxBytecodeError::Truncated)
}

pub fn encode(program: &AxProgram) -> Result<Vec<u8>, AxBytecodeError> {
    program.validate().map_err(|_| AxBytecodeError::InvalidProgram)?;
    if program.instructions.len() > MAX_INSTRUCTIONS { return Err(AxBytecodeError::TooLarge); }
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&program.isa_major.to_le_bytes());
    out.extend_from_slice(&program.isa_minor.to_le_bytes());
    out.extend_from_slice(&program.register_count.to_le_bytes());
    out.extend_from_slice(&program.device_memory_bytes.to_le_bytes());
    out.extend_from_slice(&(program.instructions.len() as u32).to_le_bytes());
    for ins in &program.instructions {
        out.push(opcode_to_byte(ins.opcode));
        out.extend_from_slice(&ins.dst.unwrap_or(u16::MAX).to_le_bytes());
        out.push(u8::try_from(ins.src.len()).map_err(|_| AxBytecodeError::InvalidEncoding)?);
        for src in &ins.src { out.extend_from_slice(&src.to_le_bytes()); }
        out.extend_from_slice(&ins.address_offset.unwrap_or(u64::MAX).to_le_bytes());
        out.extend_from_slice(&ins.access_bytes.unwrap_or(0).to_le_bytes());
        if out.len() > MAX_BYTECODE_BYTES { return Err(AxBytecodeError::TooLarge); }
    }
    Ok(out)
}

pub fn decode(bytes: &[u8]) -> Result<AxProgram, AxBytecodeError> {
    if bytes.len() > MAX_BYTECODE_BYTES { return Err(AxBytecodeError::TooLarge); }
    if bytes.get(..4) != Some(MAGIC) { return Err(AxBytecodeError::BadMagic); }
    let mut cursor = 4;
    let major = u16::from_le_bytes(take(bytes, &mut cursor)?);
    let minor = u16::from_le_bytes(take(bytes, &mut cursor)?);
    let registers = u16::from_le_bytes(take(bytes, &mut cursor)?);
    let memory = u64::from_le_bytes(take(bytes, &mut cursor)?);
    let count = u32::from_le_bytes(take(bytes, &mut cursor)?) as usize;
    if count == 0 || count > MAX_INSTRUCTIONS { return Err(AxBytecodeError::TooLarge); }
    let mut instructions = Vec::with_capacity(count.min(4096));
    for _ in 0..count {
        let opcode = byte_to_opcode(take::<1>(bytes, &mut cursor)?[0])?;
        let raw_dst = u16::from_le_bytes(take(bytes, &mut cursor)?);
        let src_count = take::<1>(bytes, &mut cursor)?[0] as usize;
        if src_count > 3 { return Err(AxBytecodeError::InvalidEncoding); }
        let mut src = Vec::with_capacity(src_count);
        for _ in 0..src_count { src.push(u16::from_le_bytes(take(bytes, &mut cursor)?)); }
        let raw_offset = u64::from_le_bytes(take(bytes, &mut cursor)?);
        let raw_width = u32::from_le_bytes(take(bytes, &mut cursor)?);
        instructions.push(AxInstruction {
            opcode, dst: (raw_dst != u16::MAX).then_some(raw_dst), src,
            address_offset: (raw_offset != u64::MAX).then_some(raw_offset),
            access_bytes: (raw_width != 0).then_some(raw_width),
        });
    }
    if cursor != bytes.len() { return Err(AxBytecodeError::InvalidEncoding); }
    let program = AxProgram { isa_major: major, isa_minor: minor, register_count: registers, device_memory_bytes: memory, instructions };
    program.validate().map_err(|_| AxBytecodeError::InvalidProgram)?;
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AxProgram {
        AxProgram {
            isa_major: AX_ISA_MAJOR, isa_minor: AX_ISA_MINOR, register_count: 8,
            device_memory_bytes: 128,
            instructions: vec![
                AxInstruction { opcode: AxOpcode::Load, dst: Some(0), src: vec![], address_offset: Some(0), access_bytes: Some(8) },
                AxInstruction { opcode: AxOpcode::Add, dst: Some(2), src: vec![0, 1], address_offset: None, access_bytes: None },
                AxInstruction { opcode: AxOpcode::Store, dst: None, src: vec![2], address_offset: Some(16), access_bytes: Some(8) },
            ],
        }
    }

    #[test] fn round_trip_preserves_program() {
        let p = sample();
        assert_eq!(decode(&encode(&p).unwrap()).unwrap(), p);
    }

    #[test] fn rejects_truncated_program() {
        let mut bytes = encode(&sample()).unwrap();
        bytes.pop();
        assert_eq!(decode(&bytes), Err(AxBytecodeError::Truncated));
    }

    #[test] fn rejects_bad_magic() {
        assert_eq!(decode(b"NOPE"), Err(AxBytecodeError::BadMagic));
    }

    #[test] fn rejects_trailing_bytes() {
        let mut bytes = encode(&sample()).unwrap();
        bytes.push(0);
        assert_eq!(decode(&bytes), Err(AxBytecodeError::InvalidEncoding));
    }

    #[test] fn rejects_unknown_opcode() {
        let mut bytes = encode(&sample()).unwrap();
        bytes[22] = 255;
        assert_eq!(decode(&bytes), Err(AxBytecodeError::UnsupportedOpcode(255)));
    }
}
