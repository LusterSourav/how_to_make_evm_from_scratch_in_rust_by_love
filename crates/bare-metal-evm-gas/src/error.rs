use core::fmt;

/// Errors that can occur during gas metering operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GasError {
    /// The transaction or operation has insufficient remaining gas.
    OutOfGas,
    /// A gas calculation overflowed the maximum `u64` value.
    Overflow,
}

impl fmt::Display for GasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfGas => write!(f, "out of gas"),
            Self::Overflow => write!(f, "gas calculation overflow"),
        }
    }
}
