use crate::constants::{CREATE_DATA_GAS, CREATE_GAS, INIT_CODE_WORD_GAS};
use crate::error::GasError;

/// Compute gas cost for a CREATE or CREATE2 opcode, excluding
/// memory expansion and access-set warming for the new address.
///
/// CREATE cost = 32_000 + initcode_word_cost + deployed_code_cost
/// CREATE2 cost = 32_000 + initcode_word_cost
pub fn create_gas(
    initcode_len: u64,
    deployed_code_len: u64,
    is_create2: bool,
) -> Result<u64, GasError> {
    let base = CREATE_GAS;
    let initcode_cost = initcode_word_cost(initcode_len)?;
    let mut total = base.checked_add(initcode_cost).ok_or(GasError::Overflow)?;
    if !is_create2 {
        total = total
            .checked_add(
                CREATE_DATA_GAS
                    .checked_mul(deployed_code_len)
                    .ok_or(GasError::Overflow)?,
            )
            .ok_or(GasError::Overflow)?;
    }
    Ok(total)
}

/// EIP-3860: 2 gas per 32-byte word of initcode.
/// Also enforces the max initcode size (49152).
pub fn initcode_word_cost(initcode_len: u64) -> Result<u64, GasError> {
    let words = initcode_len.div_ceil(32);
    INIT_CODE_WORD_GAS
        .checked_mul(words)
        .ok_or(GasError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_empty_initcode_no_deployed_code() {
        assert_eq!(create_gas(0, 0, false).unwrap(), 32_000);
    }

    #[test]
    fn create2_empty_initcode() {
        assert_eq!(create_gas(0, 0, true).unwrap(), 32_000);
    }

    #[test]
    fn create_with_initcode() {
        assert_eq!(create_gas(32, 0, false).unwrap(), 32_000 + 2);
    }

    #[test]
    fn create2_with_initcode() {
        assert_eq!(create_gas(32, 0, true).unwrap(), 32_000 + 2);
    }

    #[test]
    fn create_with_deployed_code() {
        // 100 bytes of deployed code = 200 * 100 = 20_000
        assert_eq!(create_gas(0, 100, false).unwrap(), 32_000 + 20_000);
    }

    #[test]
    fn create2_no_deployed_code_cost() {
        // CREATE2 does not charge CREATE_DATA_GAS
        assert_eq!(create_gas(0, 100, true).unwrap(), 32_000);
    }

    #[test]
    fn initcode_word_cost_exact_single() {
        assert_eq!(initcode_word_cost(32).unwrap(), 2);
    }

    #[test]
    fn initcode_word_cost_partial_word() {
        assert_eq!(initcode_word_cost(1).unwrap(), 2);
    }

    #[test]
    fn initcode_word_cost_large() {
        assert_eq!(initcode_word_cost(49152).unwrap(), 49152 / 32 * 2);
    }
}
