use crate::constants::{CREATE_DATA_GAS, CREATE_GAS, INIT_CODE_WORD_GAS, MAX_INIT_CODE_SIZE};
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
/// Returns `OutOfGas` if initcode exceeds `MAX_INIT_CODE_SIZE` (49152).
pub fn initcode_word_cost(initcode_len: u64) -> Result<u64, GasError> {
    if initcode_len > MAX_INIT_CODE_SIZE {
        return Err(GasError::OutOfGas);
    }
    let words = initcode_len.div_ceil(32);
    INIT_CODE_WORD_GAS
        .checked_mul(words)
        .ok_or(GasError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn prop_initcode_word_cost_matches_expected() {
        proptest::proptest!(proptest::test_runner::Config::default(),
            |(len in 0..=2048u64)|
        {
            let words = len.div_ceil(32);
            let expected = INIT_CODE_WORD_GAS * words;
            prop_assert_eq!(initcode_word_cost(len).unwrap(), expected);
        });
    }

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

    #[test]
    fn initcode_word_cost_zero() {
        assert_eq!(initcode_word_cost(0).unwrap(), 0);
    }

    #[test]
    fn initcode_word_cost_partial_last_word() {
        // 31 bytes rounds up to 1 word
        assert_eq!(initcode_word_cost(31).unwrap(), 2);
        // 49151 bytes is just under the limit
        assert!(initcode_word_cost(49151).is_ok());
    }

    #[test]
    fn initcode_word_cost_exceeds_max() {
        // One byte over the limit
        assert_eq!(initcode_word_cost(49153), Err(GasError::OutOfGas));
    }

    #[test]
    fn create_gas_deployed_code_overflow() {
        // Very large deployed_code_len triggers checked_mul overflow
        assert_eq!(create_gas(0, u64::MAX, false), Err(GasError::Overflow));
    }
}
