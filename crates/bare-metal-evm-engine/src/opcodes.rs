use bare_metal_evm_gas::GasMeter;
use bare_metal_evm_types::{U256, U256_MAX};

use crate::error::Error;
use crate::stack::Stack;

//opcode constants, in EVM numerical order
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

pub(crate) fn op_add(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(3).is_err() {
        return Err(Error::OutOfGas);
    }
    let b = stack.pop()?;
    let a = stack.pop()?;
    //evm add wraps, same as rust
    stack.push(a.wrapping_add(b))
}

pub(crate) fn op_mul(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(5).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a.wrapping_mul(b))
}

pub(crate) fn op_sub(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(3).is_err() {
        return Err(Error::OutOfGas);
    }
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a.wrapping_sub(b))
}

pub(crate) fn op_div(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(5).map_err(|_| Error::OutOfGas)?;
    let y = stack.pop()?;
    let x = stack.pop()?;
    //evm returns zero for div-by-zero, no trap
    let q = if y.is_zero() {
        U256::zero()
    } else {
        x.div_rem(y).0
    };
    stack.push(q)
}

pub(crate) fn op_sdiv(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(5).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    //TODO: int256.min / -1 overflow, spec says returns min
    stack.push(a.sdiv(b))
}

pub(crate) fn op_mod(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(5).is_err() {
        return Err(Error::OutOfGas);
    }
    let y = stack.pop()?;
    let x = stack.pop()?;
    let r = if y.is_zero() {
        U256::zero()
    } else {
        x.div_rem(y).1
    };
    stack.push(r)
}

pub(crate) fn op_smod(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(5).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a.smod(b))
}

//addmod uses the modulus identity to keep results in u256
//tried full-u512 path, was overkill for it
pub(crate) fn op_addmod(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(8).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    let n = stack.pop()?;
    stack.push(addmod(a, b, n))
}

//russian peasant, O(256) iters, stays in u256
//TODO: profile for large n, fine for now but eip-? may want faster
pub(crate) fn op_mulmod(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(8).is_err() {
        return Err(Error::OutOfGas);
    }
    let b = stack.pop()?;
    let a = stack.pop()?;
    let n = stack.pop()?;
    stack.push(mulmod(a, b, n))
}

pub(crate) fn op_exp(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    let base = stack.pop()?;
    let exponent = stack.pop()?;
    //gas depends on exponent byte size, delegated to the gas crate
    //gas-charge-before-work is the evm pattern (yes its weird)
    let cost =
        bare_metal_evm_gas::exp::exp_gas(&exponent.to_bytes_be()).map_err(|_| Error::OutOfGas)?;
    gas.charge(cost).map_err(|_| Error::OutOfGas)?;
    stack.push(base.exp(exponent))
}

pub(crate) fn op_lt(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let y = stack.pop()?;
    let x = stack.pop()?;
    stack.push(if x < y { U256::one() } else { U256::zero() })
}

pub(crate) fn op_gt(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(3).is_err() {
        return Err(Error::OutOfGas);
    }
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(if a > b { U256::one() } else { U256::zero() })
}

pub(crate) fn op_slt(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(if signed_lt(a, b) {
        U256::one()
    } else {
        U256::zero()
    })
}

pub(crate) fn op_sgt(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(if signed_gt(a, b) {
        U256::one()
    } else {
        U256::zero()
    })
}

pub(crate) fn op_eq(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(3).is_err() {
        return Err(Error::OutOfGas);
    }
    let y = stack.pop()?;
    let x = stack.pop()?;
    stack.push(if x == y { U256::one() } else { U256::zero() })
}

pub(crate) fn op_iszero(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let a = stack.pop()?;
    stack.push(if a.is_zero() {
        U256::one()
    } else {
        U256::zero()
    })
}

pub(crate) fn op_and(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a & b)
}

pub(crate) fn op_or(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(3).is_err() {
        return Err(Error::OutOfGas);
    }
    let y = stack.pop()?;
    let x = stack.pop()?;
    stack.push(x | y)
}

pub(crate) fn op_xor(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let b = stack.pop()?;
    let a = stack.pop()?;
    stack.push(a ^ b)
}

pub(crate) fn op_not(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(3).is_err() {
        return Err(Error::OutOfGas);
    }
    let a = stack.pop()?;
    stack.push(!a)
}

pub(crate) fn op_byte(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let i = stack.pop()?;
    let v = stack.pop()?;
    //left-indexed from most significant byte, zero if past 32
    //obscure: byte(0) of 0x1234... returns 0x12 (big-endian index)
    let byte = if i >= U256::from_u64(32) {
        U256::zero()
    } else {
        let idx = i.low_u64() as usize;
        let bytes = v.to_bytes_be();
        U256::from_u64(bytes[idx] as u64)
    };
    stack.push(byte)
}

pub(crate) fn op_shl(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let shift = stack.pop()?;
    let v = stack.pop()?;
    //shift >= 256 gives zero in evm (shift amounts > 255 are illegal)
    let res = if shift >= U256::from_u64(256) {
        U256::zero()
    } else {
        v.wrapping_shl(shift.low_u64() as u32)
    };
    stack.push(res)
}

pub(crate) fn op_shr(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(3).is_err() {
        return Err(Error::OutOfGas);
    }
    let shift = stack.pop()?;
    let v = stack.pop()?;
    let res = if shift >= U256::from_u64(256) {
        U256::zero()
    } else {
        v.wrapping_shr(shift.low_u64() as u32)
    };
    stack.push(res)
}

//SAR sign-extends: shifts right, fills high bits with sign bit
//analagous to asm arithmetic shift right
pub(crate) fn op_sar(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    let shift = stack.pop()?;
    let v = stack.pop()?;
    let res = if shift >= U256::from_u64(256) {
        //all bits gone, result is all-ones (negative) or zero (positive)
        if v.is_negative() {
            U256_MAX
        } else {
            U256::zero()
        }
    } else {
        let s = shift.low_u64() as u32;
        let mut tmp = v.wrapping_shr(s);
        if v.is_negative() {
            let mask = U256::wrapping_sub(U256::zero(), U256::one()) << (256 - s);
            tmp |= mask;
        }
        tmp
    };
    stack.push(res)
}

pub(crate) fn op_pop(stack: &mut Stack, gas: &mut GasMeter) -> Result<(), Error> {
    if gas.charge(2).is_err() {
        return Err(Error::OutOfGas);
    }
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
    Ok(end) //pc advances past the immediate bytes
            //HACK: returning PUSH1 error for any short push is misleading but works for halt
}

pub(crate) fn op_dup(stack: &mut Stack, gas: &mut GasMeter, n: usize) -> Result<(), Error> {
    gas.charge(3).map_err(|_| Error::OutOfGas)?;
    stack.dup(n)
}

pub(crate) fn op_swap(stack: &mut Stack, gas: &mut GasMeter, n: usize) -> Result<(), Error> {
    if gas.charge(3).is_err() {
        return Err(Error::OutOfGas);
    }
    stack.swap(n)
}

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
        //wrapping add of complement is same as modular subtraction
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
    //russian peasant, O(256), each step is one addmod
    //tried double-and-add first, russian peasant is simpler
    let mut res = U256::zero();
    let mut base = a_mod;
    let mut exp = b_mod;
    while !exp.is_zero() {
        if exp.low_u64() & 1 == 1 {
            res = addmod(res, base, n);
        }
        base = addmod(base, base, n);
        exp = exp.wrapping_shr(1);
    }
    res
}
