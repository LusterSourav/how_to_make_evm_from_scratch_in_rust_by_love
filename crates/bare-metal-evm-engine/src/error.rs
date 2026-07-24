use bare_metal_evm_types::U256;
use core::fmt;

//kept the error set minimal for now, add more as the engine grows
//TODO: maybe add a Revert variant when the CALL layer lands
//InvalidCallTarget(U256) — reserved, commented out until we implement call semantics
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    OutOfGas,
    StackOverflow,
    StackUnderflow,
    InvalidOpcode(u8),
    InvalidJump(U256),
    //InvalidCallTarget — reserved for when we implement CALL/CALLCODE/DELEGATECALL
    //CallTooDeep — hmm, this is actually a stack depth limit, belongs in CallState
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
