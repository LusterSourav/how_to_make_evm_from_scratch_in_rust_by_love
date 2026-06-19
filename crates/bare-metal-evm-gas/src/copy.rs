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
}
