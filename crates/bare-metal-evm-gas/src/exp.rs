use crate::constants::{EXP_BYTE_GAS, EXP_GAS};
use crate::error::GasError;

/// Compute gas cost for an EXP opcode.
///
/// Cost = 10 (static) + 50 * byte_size_of_exponent
/// where byte_size_of_exponent is the number of bytes needed
/// to represent the exponent (i.e., (bit_len + 7) / 8).
pub fn exp_gas(exponent: &[u8; 32]) -> Result<u64, GasError> {
    let byte_size = exponent
        .iter()
        .position(|&b| b != 0)
        .map(|pos| 32 - pos)
        .unwrap_or(0);
    let gas = EXP_GAS
        .checked_add(
            EXP_BYTE_GAS
                .checked_mul(byte_size as u64)
                .ok_or(GasError::Overflow)?,
        )
        .ok_or(GasError::Overflow)?;
    Ok(gas)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_zero_exponent() {
        assert_eq!(exp_gas(&[0u8; 32]).unwrap(), 10);
    }

    #[test]
    fn exp_one_byte_exponent() {
        let mut exp = [0u8; 32];
        exp[31] = 0x01;
        assert_eq!(exp_gas(&exp).unwrap(), 10 + 50);
    }

    #[test]
    fn exp_two_byte_exponent() {
        let mut exp = [0u8; 32];
        exp[30] = 0x01;
        assert_eq!(exp_gas(&exp).unwrap(), 10 + 2 * 50);
    }

    #[test]
    fn exp_full_32_byte_exponent() {
        let exp = [0xFFu8; 32];
        assert_eq!(exp_gas(&exp).unwrap(), 10 + 32 * 50);
    }
}
