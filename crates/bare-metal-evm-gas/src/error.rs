use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GasError {
    OutOfGas,
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
