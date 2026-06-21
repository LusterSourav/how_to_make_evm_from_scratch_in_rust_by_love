use crate::constants::COPY_GAS;
use crate::error::GasError;
use crate::memory::word_count;

/// Compute the per-word gas for copy operations:
/// CALLDATACOPY, CODECOPY, EXTCODECOPY, RETURNDATACOPY, and MCOPY (EIP-5656).
///
/// Cost = 3 * num_words(data_len)
pub fn copy_gas(data_len: usize) -> Result<u64, GasError> {
    let words = word_count(data_len);
    COPY_GAS.checked_mul(words as u64).ok_or(GasError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn prop_copy_gas_matches_expected() {
        proptest::proptest!(proptest::test_runner::Config::default(),
            |(len in 0..=2048usize)|
        {
            let words = if len == 0 { 0 } else { (len - 1) / 32 + 1 };
            let expected = COPY_GAS * words as u64;
            prop_assert_eq!(copy_gas(len).unwrap(), expected);
        });
    }

    #[test]
    fn copy_zero() {
        assert_eq!(copy_gas(0).unwrap(), 0);
    }

    #[test]
    fn copy_one_byte() {
        assert_eq!(copy_gas(1).unwrap(), 3);
    }

    #[test]
    fn copy_exact_word() {
        assert_eq!(copy_gas(32).unwrap(), 3);
    }

    #[test]
    fn copy_33_bytes() {
        assert_eq!(copy_gas(33).unwrap(), 6);
    }

    #[test]
    fn copy_64_bytes() {
        assert_eq!(copy_gas(64).unwrap(), 6);
    }

    #[test]
    fn copy_large() {
        assert_eq!(copy_gas(1024).unwrap(), 32 * 3);
    }

    #[test]
    fn copy_overflow_safe_max() {
        // Maximum possible copy size on 64-bit won't overflow u64
        let result = copy_gas(usize::MAX);
        assert!(result.is_ok());
    }
}
