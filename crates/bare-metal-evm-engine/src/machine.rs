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

///run bytecode, returns leftover gas on halt
//TODO: check gas limit before entering the loop (zero-limit edge case)
//TODO: jumptable dispatch if match overhead becomes a problem
//used a recursive descent parser for the first draft, the loop is simpler to trace
pub fn execute(code: &[u8], gas: GasMeter) -> Result<u64, Error> {
    let mut st = MachineState {
        stack: Stack::new(),
        memory: Memory::new(),
        pc: 0,
        gas,
        code: code.to_vec(),
    };

    loop {
        if st.pc >= st.code.len() {
            //ran out of code = implict halt
            return Ok(st.gas.remaining());
        }

        let op = st.code[st.pc];

        //consider a lookup table here if this match gets slow
        match op {
            opcodes::STOP => return Ok(st.gas.remaining()),

            opcodes::ADD => opcodes::op_add(&mut st.stack, &mut st.gas)?,
            opcodes::MUL => opcodes::op_mul(&mut st.stack, &mut st.gas)?,
            opcodes::SUB => opcodes::op_sub(&mut st.stack, &mut st.gas)?,
            opcodes::DIV => opcodes::op_div(&mut st.stack, &mut st.gas)?,
            opcodes::SDIV => opcodes::op_sdiv(&mut st.stack, &mut st.gas)?,
            opcodes::MOD => opcodes::op_mod(&mut st.stack, &mut st.gas)?,
            opcodes::SMOD => opcodes::op_smod(&mut st.stack, &mut st.gas)?,
            opcodes::ADDMOD => opcodes::op_addmod(&mut st.stack, &mut st.gas)?,
            opcodes::MULMOD => opcodes::op_mulmod(&mut st.stack, &mut st.gas)?,
            opcodes::EXP => opcodes::op_exp(&mut st.stack, &mut st.gas)?,

            opcodes::LT => opcodes::op_lt(&mut st.stack, &mut st.gas)?,
            opcodes::GT => opcodes::op_gt(&mut st.stack, &mut st.gas)?,
            opcodes::SLT => opcodes::op_slt(&mut st.stack, &mut st.gas)?,
            opcodes::SGT => opcodes::op_sgt(&mut st.stack, &mut st.gas)?,
            opcodes::EQ => opcodes::op_eq(&mut st.stack, &mut st.gas)?,
            opcodes::ISZERO => opcodes::op_iszero(&mut st.stack, &mut st.gas)?,
            opcodes::AND => opcodes::op_and(&mut st.stack, &mut st.gas)?,
            opcodes::OR => opcodes::op_or(&mut st.stack, &mut st.gas)?,
            opcodes::XOR => opcodes::op_xor(&mut st.stack, &mut st.gas)?,
            opcodes::NOT => opcodes::op_not(&mut st.stack, &mut st.gas)?,
            opcodes::BYTE => opcodes::op_byte(&mut st.stack, &mut st.gas)?,
            opcodes::SHL => opcodes::op_shl(&mut st.stack, &mut st.gas)?,
            opcodes::SHR => opcodes::op_shr(&mut st.stack, &mut st.gas)?,
            opcodes::SAR => opcodes::op_sar(&mut st.stack, &mut st.gas)?,

            opcodes::POP => opcodes::op_pop(&mut st.stack, &mut st.gas)?,

            op @ opcodes::PUSH1..=opcodes::PUSH32 => {
                let n = (op - opcodes::PUSH1 + 1) as usize;
                st.pc = opcodes::op_push(&mut st.stack, &mut st.gas, &st.code, st.pc, n)?;
                continue; //pc already past immediate
            }

            op @ opcodes::DUP1..=opcodes::DUP16 => {
                let n = (op - opcodes::DUP1) as usize;
                opcodes::op_dup(&mut st.stack, &mut st.gas, n)?;
            }

            op @ opcodes::SWAP1..=opcodes::SWAP16 => {
                let n = (op - opcodes::SWAP1) as usize;
                opcodes::op_swap(&mut st.stack, &mut st.gas, n)?;
            }

            _ => return Err(Error::InvalidOpcode(op)),
        }

        st.pc += 1;
    }
}
