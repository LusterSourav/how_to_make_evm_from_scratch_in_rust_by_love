use crate::access::AccessListItem;
use crate::constants::{
    TX_ACCESS_LIST_ADDRESS_GAS, TX_ACCESS_LIST_STORAGE_KEY_GAS, TX_CREATE_GAS,
    TX_DATA_NON_ZERO_GAS, TX_DATA_ZERO_GAS, TX_GAS,
};
use crate::error::GasError;

/// Compute intrinsic gas for a transaction: base cost, calldata, and
/// access list items.
pub fn intrinsic_gas(
    calldata: &[u8],
    access_list: &[AccessListItem],
    is_create: bool,
) -> Result<u64, GasError> {
    let mut gas = if is_create { TX_CREATE_GAS } else { TX_GAS };

    for &byte in calldata {
        gas = gas
            .checked_add(if byte == 0 {
                TX_DATA_ZERO_GAS
            } else {
                TX_DATA_NON_ZERO_GAS
            })
            .ok_or(GasError::Overflow)?;
    }

    for item in access_list {
        gas = gas
            .checked_add(TX_ACCESS_LIST_ADDRESS_GAS)
            .ok_or(GasError::Overflow)?;
        gas = gas
            .checked_add(
                TX_ACCESS_LIST_STORAGE_KEY_GAS
                    .checked_mul(item.storage_keys.len() as u64)
                    .ok_or(GasError::Overflow)?,
            )
            .ok_or(GasError::Overflow)?;
    }

    Ok(gas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn intrinsic_simple_tx() {
        assert_eq!(intrinsic_gas(b"", &[], false).unwrap(), 21_000);
    }

    #[test]
    fn intrinsic_create() {
        assert_eq!(intrinsic_gas(b"", &[], true).unwrap(), 53_000);
    }

    #[test]
    fn intrinsic_zero_bytes() {
        let calldata = [0u8; 10];
        assert_eq!(
            intrinsic_gas(&calldata, &[], false).unwrap(),
            21_000 + 10 * 4
        );
    }

    #[test]
    fn intrinsic_nonzero_bytes() {
        let calldata = [0xFFu8; 10];
        assert_eq!(
            intrinsic_gas(&calldata, &[], false).unwrap(),
            21_000 + 10 * 16
        );
    }

    #[test]
    fn intrinsic_mixed_bytes() {
        let calldata = [0u8, 0xFF, 0u8, 0xFF];
        assert_eq!(
            intrinsic_gas(&calldata, &[], false).unwrap(),
            21_000 + 2 * 4 + 2 * 16
        );
    }

    #[test]
    fn intrinsic_with_access_list() {
        let items = vec![AccessListItem {
            address: [1u8; 20],
            storage_keys: vec![[2u8; 32], [3u8; 32]],
        }];
        assert_eq!(
            intrinsic_gas(b"", &items, false).unwrap(),
            21_000 + 2400 + 2 * 1900
        );
    }

    #[test]
    fn intrinsic_create_with_calldata() {
        let calldata = [0xABu8; 5];
        assert_eq!(
            intrinsic_gas(&calldata, &[], true).unwrap(),
            53_000 + 5 * 16
        );
    }

    #[test]
    fn intrinsic_no_overflow_on_large_calldata() {
        let calldata = vec![0xFFu8; 100_000];
        let result = intrinsic_gas(&calldata, &[], false);
        assert!(result.is_ok());
        assert!(result.unwrap() > 21_000);
    }

    #[test]
    fn intrinsic_multiple_access_list_items() {
        let items = vec![
            AccessListItem {
                address: [1u8; 20],
                storage_keys: vec![[2u8; 32]],
            },
            AccessListItem {
                address: [3u8; 20],
                storage_keys: vec![[4u8; 32], [5u8; 32]],
            },
        ];
        assert_eq!(
            intrinsic_gas(b"", &items, false).unwrap(),
            21_000 + 2 * 2400 + 3 * 1900
        );
    }
}
