use alloc::vec::Vec;
use bare_metal_evm_gas::GasMeter;

use crate::error::Error;
use crate::memory::Memory;
use crate::opcodes;
use crate::stack::Stack;

pub struct MachineState {
    pub stack: Stack,
    pub memory: Memory,
    pub pc: usize,
    pub gas: GasMeter,
    pub code: Vec<u8>,
}

/// Execute bytecode with the given gas meter. Returns remaining gas on normal halt.
pub fn execute(code: &[u8], gas: GasMeter) -> Result<u64, Error> {
    let mut state = MachineState {
        stack: Stack::new(),
        memory: Memory::new(),
        pc: 0,
        gas,
        code: code.to_vec(),
    };

    loop {
        if state.pc >= state.code.len() {
            return Ok(state.gas.remaining());
        }

        let opcode = state.code[state.pc];

        match opcode {
            opcodes::STOP => return Ok(state.gas.remaining()),

            // ── Arithmetic ──
            opcodes::ADD => opcodes::op_add(&mut state.stack, &mut state.gas)?,
            opcodes::MUL => opcodes::op_mul(&mut state.stack, &mut state.gas)?,
            opcodes::SUB => opcodes::op_sub(&mut state.stack, &mut state.gas)?,
            opcodes::DIV => opcodes::op_div(&mut state.stack, &mut state.gas)?,
            opcodes::SDIV => opcodes::op_sdiv(&mut state.stack, &mut state.gas)?,
            opcodes::MOD => opcodes::op_mod(&mut state.stack, &mut state.gas)?,
            opcodes::SMOD => opcodes::op_smod(&mut state.stack, &mut state.gas)?,
            opcodes::ADDMOD => opcodes::op_addmod(&mut state.stack, &mut state.gas)?,
            opcodes::MULMOD => opcodes::op_mulmod(&mut state.stack, &mut state.gas)?,
            opcodes::EXP => opcodes::op_exp(&mut state.stack, &mut state.gas)?,

            // ── Comparison ──
            opcodes::LT => opcodes::op_lt(&mut state.stack, &mut state.gas)?,
            opcodes::GT => opcodes::op_gt(&mut state.stack, &mut state.gas)?,
            opcodes::SLT => opcodes::op_slt(&mut state.stack, &mut state.gas)?,
            opcodes::SGT => opcodes::op_sgt(&mut state.stack, &mut state.gas)?,
            opcodes::EQ => opcodes::op_eq(&mut state.stack, &mut state.gas)?,
            opcodes::ISZERO => opcodes::op_iszero(&mut state.stack, &mut state.gas)?,

            // ── Bitwise ──
            opcodes::AND => opcodes::op_and(&mut state.stack, &mut state.gas)?,
            opcodes::OR => opcodes::op_or(&mut state.stack, &mut state.gas)?,
            opcodes::XOR => opcodes::op_xor(&mut state.stack, &mut state.gas)?,
            opcodes::NOT => opcodes::op_not(&mut state.stack, &mut state.gas)?,
            opcodes::BYTE => opcodes::op_byte(&mut state.stack, &mut state.gas)?,
            opcodes::SHL => opcodes::op_shl(&mut state.stack, &mut state.gas)?,
            opcodes::SHR => opcodes::op_shr(&mut state.stack, &mut state.gas)?,
            opcodes::SAR => opcodes::op_sar(&mut state.stack, &mut state.gas)?,

            // ── Stack ──
            opcodes::POP => opcodes::op_pop(&mut state.stack, &mut state.gas)?,

            op @ opcodes::PUSH1..=opcodes::PUSH32 => {
                let n = (op - opcodes::PUSH1 + 1) as usize;
                state.pc =
                    opcodes::op_push(&mut state.stack, &mut state.gas, &state.code, state.pc, n)?;
                continue; // pc already advanced past immediate data
            }

            op @ opcodes::DUP1..=opcodes::DUP16 => {
                let n = (op - opcodes::DUP1) as usize;
                opcodes::op_dup(&mut state.stack, &mut state.gas, n)?;
            }

            op @ opcodes::SWAP1..=opcodes::SWAP16 => {
                let n = (op - opcodes::SWAP1) as usize;
                opcodes::op_swap(&mut state.stack, &mut state.gas, n)?;
            }

            _ => return Err(Error::InvalidOpcode(opcode)),
        }

        state.pc += 1;
    }
}
