use crate::constants::{KECCAK256_GAS, KECCAK256_WORD_GAS};
use crate::error::GasError;
use crate::memory::word_count;

/// Compute gas cost for a SHA3 (KECCAK256) opcode.
///
/// Cost = 30 (static) + 6 * num_words(input_data)
/// where num_words = ceil(input_size / 32)
pub fn sha3_gas(data_len: usize) -> Result<u64, GasError> {
    let words = word_count(data_len);
    let gas = KECCAK256_GAS
        .checked_add(
            KECCAK256_WORD_GAS
                .checked_mul(words as u64)
                .ok_or(GasError::Overflow)?,
        )
        .ok_or(GasError::Overflow)?;
    Ok(gas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn prop_sha3_gas_word_aligned() {
        proptest::proptest!(proptest::test_runner::Config::default(),
            |(len in 0..=1024usize)|
        {
            let words = if len == 0 { 0 } else { (len - 1) / 32 + 1 };
            let expected = KECCAK256_GAS + KECCAK256_WORD_GAS * words as u64;
            prop_assert_eq!(sha3_gas(len).unwrap(), expected);
        });
    }

    #[test]
    fn sha3_empty() {
        assert_eq!(sha3_gas(0).unwrap(), 30);
    }

    #[test]
    fn sha3_one_byte() {
        assert_eq!(sha3_gas(1).unwrap(), 30 + 6);
    }

    #[test]
    fn sha3_exact_word() {
        assert_eq!(sha3_gas(32).unwrap(), 30 + 6);
    }

    #[test]
    fn sha3_33_bytes() {
        assert_eq!(sha3_gas(33).unwrap(), 30 + 2 * 6);
    }

    #[test]
    fn sha3_large() {
        assert_eq!(sha3_gas(1024).unwrap(), 30 + 32 * 6);
    }

    #[test]
    fn sha3_overflow_safe_max() {
        // Maximum possible input size on 64-bit won't overflow u64
        let result = sha3_gas(usize::MAX);
        assert!(result.is_ok());
    }
}
