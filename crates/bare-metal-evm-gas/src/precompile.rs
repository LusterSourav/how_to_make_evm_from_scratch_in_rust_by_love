use crate::constants::{
    BLOB_POINT_EVAL_GAS, BLS12381_G1ADD_GAS, BLS12381_G1MUL_GAS, BLS12381_G2ADD_GAS,
    BLS12381_G2MUL_GAS, BLS12381_MAP_G1_GAS, BLS12381_MAP_G2_GAS, BLS12381_PAIRING_BASE_GAS,
    BLS12381_PAIRING_PER_PAIR_GAS, BN256ADD_GAS_ISTANBUL, BN256PAIRING_BASE_GAS_ISTANBUL,
    BN256PAIRING_PER_POINT_GAS_ISTANBUL, BN256SCALARMUL_GAS_ISTANBUL, ECRECOVER_GAS,
    IDENTITY_BASE_GAS, IDENTITY_PER_WORD_GAS, P256VERIFY_GAS, RIPEMD160_BASE_GAS,
    RIPEMD160_PER_WORD_GAS, SHA256_BASE_GAS, SHA256_PER_WORD_GAS,
};
use crate::error::GasError;
use crate::memory::word_count;

/// Compute gas cost for a precompile call by address.
///
/// Returns `None` for unknown precompile addresses (the EVM treats
/// them as regular CALLs).
pub fn precompile_gas(precompile_address: u8, input_len: usize) -> Option<Result<u64, GasError>> {
    match precompile_address {
        0x01 => Some(Ok(ECRECOVER_GAS)),
        0x02 => Some(sha256_gas(input_len)),
        0x03 => Some(ripemd160_gas(input_len)),
        0x04 => Some(identity_gas(input_len)),
        0x05 => unimplemented!("modexp gas is input-dependent and requires the full header"),
        0x06 => Some(Ok(BN256ADD_GAS_ISTANBUL)),
        0x07 => Some(Ok(BN256SCALARMUL_GAS_ISTANBUL)),
        // Bn256Pairing: num_pairs = input_len / 192
        0x08 => {
            let num_pairs = input_len / 192;
            Some(bn256_pairing_gas(num_pairs as u64))
        }
        0x09 => Some(Ok(0)), // blake2f — requires round count
        0x0a => Some(Ok(BLS12381_G1ADD_GAS)),
        0x0b => Some(Ok(BLS12381_G1MUL_GAS)),
        0x0c => Some(Ok(BLS12381_G2ADD_GAS)),
        0x0d => Some(Ok(BLS12381_G2MUL_GAS)),
        // Bls12381Pairing: num_pairs = input_len / 288
        0x0e => {
            let num_pairs = input_len / 288;
            Some(bls12381_pairing_gas(num_pairs as u64))
        }
        0x0f => Some(Ok(BLS12381_MAP_G1_GAS)),
        0x10 => Some(Ok(BLS12381_MAP_G2_GAS)),
        0x11 => Some(Ok(P256VERIFY_GAS)),
        0x12 => Some(Ok(BLOB_POINT_EVAL_GAS)),
        _ => None,
    }
}

/// SHA256 gas: 60 + 12 * num_words(input)
fn sha256_gas(input_len: usize) -> Result<u64, GasError> {
    let words = word_count(input_len);
    SHA256_BASE_GAS
        .checked_add(
            SHA256_PER_WORD_GAS
                .checked_mul(words as u64)
                .ok_or(GasError::Overflow)?,
        )
        .ok_or(GasError::Overflow)
}

/// RIPEMD160 gas: 600 + 120 * num_words(input)
fn ripemd160_gas(input_len: usize) -> Result<u64, GasError> {
    let words = word_count(input_len);
    RIPEMD160_BASE_GAS
        .checked_add(
            RIPEMD160_PER_WORD_GAS
                .checked_mul(words as u64)
                .ok_or(GasError::Overflow)?,
        )
        .ok_or(GasError::Overflow)
}

/// IDENTITY gas: 15 + 3 * num_words(input)
fn identity_gas(input_len: usize) -> Result<u64, GasError> {
    let words = word_count(input_len);
    IDENTITY_BASE_GAS
        .checked_add(
            IDENTITY_PER_WORD_GAS
                .checked_mul(words as u64)
                .ok_or(GasError::Overflow)?,
        )
        .ok_or(GasError::Overflow)
}

/// Compute gas for Bn256Pairing (precompile 08).
/// Istanbul: 45_000 + 34_000 * num_pairs
pub fn bn256_pairing_gas(num_pairs: u64) -> Result<u64, GasError> {
    BN256PAIRING_BASE_GAS_ISTANBUL
        .checked_add(
            BN256PAIRING_PER_POINT_GAS_ISTANBUL
                .checked_mul(num_pairs)
                .ok_or(GasError::Overflow)?,
        )
        .ok_or(GasError::Overflow)
}

/// Compute gas for Bls12381Pairing (precompile 14).
/// 37_700 + 32_600 * num_pairs
pub fn bls12381_pairing_gas(num_pairs: u64) -> Result<u64, GasError> {
    BLS12381_PAIRING_BASE_GAS
        .checked_add(
            BLS12381_PAIRING_PER_PAIR_GAS
                .checked_mul(num_pairs)
                .ok_or(GasError::Overflow)?,
        )
        .ok_or(GasError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecrecover_gas() {
        assert_eq!(precompile_gas(0x01, 0).unwrap().unwrap(), 3_000);
    }

    #[test]
    fn sha256_empty() {
        assert_eq!(precompile_gas(0x02, 0).unwrap().unwrap(), 60);
    }

    #[test]
    fn sha256_one_word() {
        assert_eq!(precompile_gas(0x02, 32).unwrap().unwrap(), 60 + 12);
    }

    #[test]
    fn ripemd160_empty() {
        assert_eq!(precompile_gas(0x03, 0).unwrap().unwrap(), 600);
    }

    #[test]
    fn ripemd160_one_word() {
        assert_eq!(precompile_gas(0x03, 32).unwrap().unwrap(), 600 + 120);
    }

    #[test]
    fn identity_empty() {
        assert_eq!(precompile_gas(0x04, 0).unwrap().unwrap(), 15);
    }

    #[test]
    fn identity_one_word() {
        assert_eq!(precompile_gas(0x04, 32).unwrap().unwrap(), 15 + 3);
    }

    #[test]
    fn bn256add() {
        assert_eq!(precompile_gas(0x06, 0).unwrap().unwrap(), 150);
    }

    #[test]
    fn bn256scalarmul() {
        assert_eq!(precompile_gas(0x07, 0).unwrap().unwrap(), 6_000);
    }

    #[test]
    fn bls_g1add() {
        assert_eq!(precompile_gas(0x0a, 0).unwrap().unwrap(), 375);
    }

    #[test]
    fn bls_g1mul() {
        assert_eq!(precompile_gas(0x0b, 0).unwrap().unwrap(), 12_000);
    }

    #[test]
    fn bls_g2add() {
        assert_eq!(precompile_gas(0x0c, 0).unwrap().unwrap(), 600);
    }

    #[test]
    fn bls_g2mul() {
        assert_eq!(precompile_gas(0x0d, 0).unwrap().unwrap(), 22_500);
    }

    #[test]
    fn bls_map_g1() {
        assert_eq!(precompile_gas(0x0f, 0).unwrap().unwrap(), 5_500);
    }

    #[test]
    fn bls_map_g2() {
        assert_eq!(precompile_gas(0x10, 0).unwrap().unwrap(), 23_800);
    }

    #[test]
    fn p256verify() {
        assert_eq!(precompile_gas(0x11, 0).unwrap().unwrap(), 6_900);
    }

    #[test]
    fn blob_point_eval() {
        assert_eq!(precompile_gas(0x12, 0).unwrap().unwrap(), 50_000);
    }

    #[test]
    fn bn256_pairing_gas_basic() {
        assert_eq!(bn256_pairing_gas(1).unwrap(), 45_000 + 34_000);
    }

    #[test]
    fn bn256_pairing_gas_2_pairs() {
        assert_eq!(bn256_pairing_gas(2).unwrap(), 45_000 + 2 * 34_000);
    }

    #[test]
    fn bls12381_pairing_gas_basic() {
        assert_eq!(bls12381_pairing_gas(1).unwrap(), 37_700 + 32_600);
    }

    #[test]
    fn bls12381_pairing_gas_2_pairs() {
        assert_eq!(bls12381_pairing_gas(2).unwrap(), 37_700 + 2 * 32_600);
    }

    #[test]
    fn unknown_precompile() {
        assert!(precompile_gas(0x13, 0).is_none());
    }
}
