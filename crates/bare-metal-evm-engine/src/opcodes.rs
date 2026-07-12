use bare_metal_evm_gas::GasMeter;
use bare_metal_evm_types::{U256, U256_MAX};

use crate::error::Error;
use crate::stack::Stack;

// ── Opcode constants ────────────────────────────────────────────────

pub const STOP: u8 = 0x00;
pub const ADD: u8 = 0x01;
pub const MUL: u8 = 0x02;
pub const SUB: u8 = 0x03;
pub const DIV: u8 = 0x04;
pub const SDIV: u8 = 0x05;
pub const MOD: u8 = 0x06;
pub const SMOD: u8 = 0x07;
pub const ADDMOD: u8 = 0x08;
pub const MULMOD: u8 = 0x09;
pub const EXP: u8 = 0x0a;
pub const LT: u8 = 0x10;
pub const GT: u8 = 0x11;
pub const SLT: u8 = 0x12;
pub const SGT: u8 = 0x13;
pub const EQ: u8 = 0x14;
pub const ISZERO: u8 = 0x15;
pub const AND: u8 = 0x16;
pub const OR: u8 = 0x17;
pub const XOR: u8 = 0x18;
pub const NOT: u8 = 0x19;
pub const BYTE: u8 = 0x1a;
pub const SHL: u8 = 0x1b;
pub const SHR: u8 = 0x1c;
pub const SAR: u8 = 0x1d;
pub const POP: u8 = 0x50;
pub const PUSH1: u8 = 0x60;
pub const PUSH32: u8 = 0x7f;
pub const DUP1: u8 = 0x80;
pub const DUP16: u8 = 0x8f;
pub const SWAP1: u8 = 0x90;
pub const SWAP16: u8 = 0x9f;

// ── Arithmetic handlers ─────────────────────────────────────────────

pub(crate) fn op_add(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a.wrapping_add(b))
}

pub(crate) fn op_mul(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(5).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a.wrapping_mul(b))
}

pub(crate) fn op_sub(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a.wrapping_sub(b))
}

pub(crate) fn op_div(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(5).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    let q = if b.is_zero() { U256::zero() } else { a.div_rem(b).0 };
    stack.push(q)
}

pub(crate) fn op_sdiv(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(5).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a.sdiv(b))
}

pub(crate) fn op_mod(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(5).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    let r = if b.is_zero() { U256::zero() } else { a.div_rem(b).1 };
    stack.push(r)
}

pub(crate) fn op_smod(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(5).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a.smod(b))
}

// ponytail: addmod uses the identity (a+b) mod n = ((a mod n) + (b mod n)) mod n
// to keep intermediates within U256. Overflow case handled via wrapping arithmetic.
pub(crate) fn op_addmod(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(8).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    let n = stack.pop()?;
    stack.push(addmod(a, b, n))
}

// ponytail: mulmod uses Russian peasant method for modular multiplication
// since U512 lacks arithmetic ops. O(256) iterations, each O(1).
pub(crate) fn op_mulmod(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(8).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    let n = stack.pop()?;
    stack.push(mulmod(a, b, n))
}

pub(crate) fn op_exp(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    let base = stack.pop()?;
    let exponent = stack.pop()?;
    let cost = bare_metal_evm_gas::exp::exp_gas(&exponent.to_bytes_be()).map_err(|_| Error::OutOfGas)?;
    gas.charge(cost).map_err(|_| Error::OutOfGas)?;
    stack.push(base.exp(exponent))
}

// ── Comparison handlers ─────────────────────────────────────────────

pub(crate) fn op_lt(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(if a < b { U256::one() } else { U256::zero() })
}

pub(crate) fn op_gt(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(if a > b { U256::one() } else { U256::zero() })
}

pub(crate) fn op_slt(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(if signed_lt(a, b) { U256::one() } else { U256::zero() })
}

pub(crate) fn op_sgt(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(if signed_gt(a, b) { U256::one() } else { U256::zero() })
}

pub(crate) fn op_eq(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(if a == b { U256::one() } else { U256::zero() })
}

pub(crate) fn op_iszero(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let a = stack.pop()?;
    stack.push(if a.is_zero() { U256::one() } else { U256::zero() })
}

// ── Bitwise handlers ────────────────────────────────────────────────

pub(crate) fn op_and(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a & b)
}

pub(crate) fn op_or(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a | b)
}

pub(crate) fn op_xor(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a ^ b)
}

pub(crate) fn op_not(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let a = stack.pop()?;
    stack.push(!a)
}

pub(crate) fn op_byte(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let offset = stack.pop()?;
    let word = stack.pop()?;
    let byte = if offset >= U256::from_u64(32) {
        U256::zero()
    } else {
        let i = offset.low_u64() as usize;
        let bytes = word.to_bytes_be();
        U256::from_u64(bytes[i] as u64)
    };
    stack.push(byte)
}

pub(crate) fn op_shl(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let shift = stack.pop()?;
    let value = stack.pop()?;
    let result = if shift >= U256::from_u64(256) {
        U256::zero()
    } else {
        value.wrapping_shl(shift.low_u64() as u32)
    };
    stack.push(result)
}

pub(crate) fn op_shr(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let shift = stack.pop()?;
    let value = stack.pop()?;
    let result = if shift >= U256::from_u64(256) {
        U256::zero()
    } else {
        value.wrapping_shr(shift.low_u64() as u32)
    };
    stack.push(result)
}

// ponytail: SAR (arithmetic shift right) preserves sign bit
pub(crate) fn op_sar(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let shift = stack.pop()?;
    let value = stack.pop()?;
    let result = if shift >= U256::from_u64(256) {
        if value.is_negative() { U256_MAX } else { U256::zero() }
    } else {
        let s = shift.low_u64() as u32;
        let mut result = value.wrapping_shr(s);
        // Fill high bits with sign bit for negative values
        if value.is_negative() {
            let mask = U256::wrapping_sub(U256::zero(), U256::one()) << (256 - s);
            result |= mask;
        }
        result
    };
    stack.push(result)
}

// ── Stack op handlers ───────────────────────────────────────────────

pub(crate) fn op_pop(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(2).map_err(|_| Error::OutOfGas)?;
    stack.pop()?;
    Ok(())
}

pub(crate) fn op_push(
    stack: &mut Stack,
    gas: &mut GasMeter,
    code: &[u8],
    pc: usize,
    n: usize,
) -> Result<usize, Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let start = pc + 1;
    let end = start + n;
    if end > code.len() {
        return Err(Error::InvalidOpcode(PUSH1));
    }
    let mut bytes = [0u8; 32];
    bytes[32 - n..].copy_from_slice(&code[start..end]);
    stack.push(U256::from_bytes_be(bytes))?;
    Ok(end) // new pc after skipping the immediate data
}

pub(crate) fn op_dup(stack: &mut Stack, gas: &mut GasMeter, n: usize) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    stack.dup(n)
}

pub(crate) fn op_swap(stack: &mut Stack, gas: &mut GasMeter, n: usize) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    stack.swap(n)
}

// ── Helpers ─────────────────────────────────────────────────────────

fn signed_lt(a: U256, b: U256) -> bool {
    match (a.is_negative(), b.is_negative()) {
        (true, false) => true,
        (false, true) => false,
        _ => a < b,
    }
}

fn signed_gt(a: U256, b: U256) -> bool {
    match (a.is_negative(), b.is_negative()) {
        (false, true) => true,
        (true, false) => false,
        _ => a > b,
    }
}

fn addmod(a: U256, b: U256, n: U256) -> U256 {
    if n.is_zero() {
        return U256::zero();
    }
    let a_mod = a.div_rem(n).1;
    let b_mod = b.div_rem(n).1;
    let (sum, overflow) = a_mod.overflowing_add(b_mod);
    if overflow {
        let adjust = U256::zero().wrapping_sub(n);
        sum.wrapping_add(adjust)
    } else if sum >= n {
        sum - n
    } else {
        sum
    }
}

fn mulmod(a: U256, b: U256, n: U256) -> U256 {
    if n.is_zero() {
        return U256::zero();
    }
    let a_mod = a.div_rem(n).1;
    let b_mod = b.div_rem(n).1;
    // Russian peasant: O(256) iterations of addmod
    let mut result = U256::zero();
    let mut base = a_mod;
    let mut exp = b_mod;
    while !exp.is_zero() {
        if exp.low_u64() & 1 == 1 {
            result = addmod(result, base, n);
        }
        base = addmod(base, base, n);
        exp = exp.wrapping_shr(1);
    }
    result
}
