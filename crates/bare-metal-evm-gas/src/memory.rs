use crate::constants::{MEMORY_GAS, MEMORY_MAX_SIZE, QUAD_COEFF_DIV};
use crate::error::GasError;

/// Round up `byte_size` to the next 32-byte word.
#[must_use]
pub fn word_count(byte_size: usize) -> usize {
    if byte_size == 0 {
        return 0;
    }
    (byte_size - 1) / 32 + 1
}

/// Gas cost for `num_words` words of memory: `3*words + words^2/512`.
/// Returns `OutOfGas` when the word count exceeds the maximum memory size.
pub fn memory_cost(num_words: usize) -> Result<u64, GasError> {
    if num_words > MEMORY_MAX_SIZE / 32 {
        return Err(GasError::OutOfGas);
    }
    let w = num_words as u128;
    let linear = MEMORY_GAS as u128 * w;
    let quadratic = w * w / QUAD_COEFF_DIV as u128;
    (linear + quadratic)
        .try_into()
        .map_err(|_| GasError::Overflow)
}

/// Incremental gas cost when memory grows from `prev_num_words` to
/// `new_num_words`. Returns zero if memory doesn't expand.
/// Propagates `OutOfGas` from `memory_cost` when the word count
/// exceeds the maximum memory size.
pub fn memory_expansion_cost(prev_num_words: usize, new_num_words: usize) -> Result<u64, GasError> {
    if new_num_words <= prev_num_words {
        return Ok(0);
    }
    let prev = memory_cost(prev_num_words)?;
    let new = memory_cost(new_num_words)?;
    // new >= prev because memory_cost is monotonic
    Ok(new - prev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn word_count_zero() {
        assert_eq!(word_count(0), 0);
    }

    #[test]
    fn word_count_one_byte() {
        assert_eq!(word_count(1), 1);
    }

    #[test]
    fn word_count_exact_word() {
        assert_eq!(word_count(32), 1);
    }

    #[test]
    fn word_count_fractional_word() {
        assert_eq!(word_count(33), 2);
    }

    #[test]
    fn word_count_large() {
        assert_eq!(word_count(1024), 32);
    }

    #[test]
    fn memory_cost_zero_words() {
        assert_eq!(memory_cost(0).unwrap(), 0);
    }

    #[test]
    fn memory_cost_one_word() {
        assert_eq!(memory_cost(1).unwrap(), 3);
    }

    #[test]
    fn memory_cost_two_words() {
        assert_eq!(memory_cost(2).unwrap(), 6);
    }

    #[test]
    fn memory_cost_quadratic_kicks_in() {
        let _one_word = memory_cost(1).unwrap();
        let many_words = memory_cost(512).unwrap();
        assert!(many_words > 512 * 3);
    }

    #[test]
    fn memory_cost_overflow_at_max() {
        assert!(memory_cost(MEMORY_MAX_SIZE / 32 + 1).is_err());
    }

    #[test]
    fn memory_expansion_cost_no_expansion() {
        assert_eq!(memory_expansion_cost(5, 5).unwrap(), 0);
        assert_eq!(memory_expansion_cost(10, 5).unwrap(), 0);
    }

    #[test]
    fn memory_expansion_cost_delta() {
        let c10 = memory_cost(10).unwrap();
        let c20 = memory_cost(20).unwrap();
        assert_eq!(memory_expansion_cost(10, 20).unwrap(), c20 - c10);
    }

    #[test]
    fn memory_expansion_cost_zero_to_nonzero() {
        let c5 = memory_cost(5).unwrap();
        assert_eq!(memory_expansion_cost(0, 5).unwrap(), c5);
    }

    #[test]
    fn memory_cost_typical_tx() {
        let cost = memory_cost(128).unwrap();
        assert_eq!(cost, 416);
    }

    #[test]
    fn memory_cost_exact_max_valid() {
        let words = MEMORY_MAX_SIZE / 32;
        let result = memory_cost(words);
        assert!(result.is_ok());
    }

    #[test]
    fn prop_memory_cost_monotonic() {
        proptest::proptest!(proptest::test_runner::Config::default(),
            |(a in 0usize..=32768usize, b in 0usize..=32768usize)|
        {
            if a > b { return Ok(()); }
            let cost_a = memory_cost(a).unwrap();
            let cost_b = memory_cost(b).unwrap();
            prop_assert!(cost_a <= cost_b, "memory_cost should be monotonic: {a}→{cost_a} > {b}→{cost_b}");
        });
    }

    #[test]
    fn prop_memory_expansion_non_negative() {
        proptest::proptest!(proptest::test_runner::Config::default(),
            |(prev in 0usize..=32768usize, new in 0usize..=32768usize)|
        {
            if new <= prev {
                prop_assert_eq!(memory_expansion_cost(prev, new).unwrap(), 0);
            } else {
                let cost = memory_expansion_cost(prev, new).unwrap();
                prop_assert!(cost > 0);
            }
        });
    }
}
