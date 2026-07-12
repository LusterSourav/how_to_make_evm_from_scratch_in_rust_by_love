use bare_metal_evm_types::U256;
use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    OutOfGas,
    StackOverflow,
    StackUnderflow,
    InvalidOpcode(u8),
    InvalidJump(U256),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OutOfGas => write!(f, "out of gas"),
            Error::StackOverflow => write!(f, "stack overflow"),
            Error::StackUnderflow => write!(f, "stack underflow"),
            Error::InvalidOpcode(op) => write!(f, "invalid opcode: {op:#04x}"),
            Error::InvalidJump(dest) => write!(f, "invalid jump destination: {dest}"),
        }
    }
}

impl core::error::Error for Error {}
