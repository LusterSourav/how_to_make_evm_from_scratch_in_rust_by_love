use crate::constants::{CALL_GAS, CALL_NEW_ACCOUNT_GAS, CALL_STIPEND, CALL_VALUE_TRANSFER_GAS};
use crate::error::GasError;

#[must_use]
pub fn call_stipend(has_value: bool) -> u64 {
    if has_value {
        CALL_STIPEND
    } else {
        0
    }
}

#[must_use]
pub fn eip150_available_gas(remaining_gas: u64) -> u64 {
    remaining_gas - remaining_gas / 64
}

/// Returns `(cost_to_caller, gas_forwarded_to_child)`.
///
/// The caller pays `cost_to_caller`. The child starts with
/// `gas_forwarded_to_child`. The caller retains at least
/// `(remaining_gas - cost) / 64` (EIP-150 63/64 rule applied
/// after intrinsic call cost).
///
/// The stipend (2300) is added after the EIP-150 cap, so the
/// child can exceed the caller's retained gas. This matches
/// go-ethereum behavior.
pub fn gas_for_child_call(
    remaining_gas: u64,
    requested_gas: u64,
    has_value: bool,
    is_new_account: bool,
) -> Result<(u64, u64), GasError> {
    let mut cost = CALL_GAS;
    if has_value {
        cost = cost
            .checked_add(CALL_VALUE_TRANSFER_GAS)
            .ok_or(GasError::Overflow)?;
    }
    if is_new_account {
        cost = cost
            .checked_add(CALL_NEW_ACCOUNT_GAS)
            .ok_or(GasError::Overflow)?;
    }

    if cost > remaining_gas {
        return Err(GasError::OutOfGas);
    }

    let max_forward = eip150_available_gas(remaining_gas - cost);
    let forwarded = core::cmp::min(requested_gas, max_forward);
    let stipend = call_stipend(has_value);

    let total_forwarded = forwarded.checked_add(stipend).ok_or(GasError::Overflow)?;
    Ok((cost, total_forwarded))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stipend_with_value() {
        assert_eq!(call_stipend(true), 2300);
    }

    #[test]
    fn stipend_without_value() {
        assert_eq!(call_stipend(false), 0);
    }

    #[test]
    fn eip150_available_gas_full() {
        assert_eq!(eip150_available_gas(1000), 985);
    }

    #[test]
    fn eip150_available_gas_small() {
        assert_eq!(eip150_available_gas(64), 63);
    }

    #[test]
    fn eip150_available_gas_zero() {
        assert_eq!(eip150_available_gas(0), 0);
    }

    #[test]
    fn child_call_basic() {
        let (cost, forwarded) = gas_for_child_call(100_000, 50_000, false, false).unwrap();
        assert_eq!(cost, 700);
        assert_eq!(forwarded, 50_000);
    }

    #[test]
    fn child_call_with_value() {
        let (cost, _) = gas_for_child_call(100_000, 50_000, true, false).unwrap();
        assert_eq!(cost, 700 + 9000);
    }

    #[test]
    fn child_call_new_account() {
        let (cost, _) = gas_for_child_call(100_000, 50_000, false, true).unwrap();
        assert_eq!(cost, 700 + 25000);
    }

    #[test]
    fn child_call_value_and_new_account() {
        let (cost, _) = gas_for_child_call(100_000, 50_000, true, true).unwrap();
        assert_eq!(cost, 700 + 9000 + 25000);
    }

    #[test]
    fn child_call_insufficient_gas() {
        let result = gas_for_child_call(500, 50_000, true, true);
        assert_eq!(result, Err(GasError::OutOfGas));
    }

    #[test]
    fn child_call_eip150_capped() {
        let (cost, forwarded) = gas_for_child_call(100_000, 99_000, false, false).unwrap();
        assert_eq!(cost, 700);
        // 63/64 applies after deducting cost: eip150_available_gas(100_000 - 700)
        let max_forward = eip150_available_gas(100_000 - 700);
        assert_eq!(forwarded, max_forward);
    }

    #[test]
    fn child_call_stipend_added() {
        let (_, forwarded) = gas_for_child_call(100_000, 50_000, true, false).unwrap();
        assert!(forwarded >= 50_000 + 2300);
    }

    #[test]
    fn child_call_exact_cost() {
        let cost_needed = 700 + 9000 + 25000;
        let result = gas_for_child_call(cost_needed, 0, true, true);
        assert!(result.is_ok());
        let (cost, _) = result.unwrap();
        assert_eq!(cost, cost_needed);
    }

    #[test]
    fn call_gas_min_edge() {
        // remaining == cost, 0 forwarded but stipend still given
        let (cost, forwarded) = gas_for_child_call(700 + 9000, 0, true, false).unwrap();
        assert_eq!(cost, 700 + 9000);
        assert_eq!(forwarded, 2300);
    }

    #[test]
    fn eip150_available_gas_one() {
        assert_eq!(eip150_available_gas(1), 1);
    }

    #[test]
    fn child_call_max_remaining_with_stipend() {
        // With maximum remaining gas, EIP-150's 63/64 rule prevents
        // forwarded + stipend from overflowing u64.
        let result = gas_for_child_call(u64::MAX, u64::MAX, true, false);
        assert!(result.is_ok());
        let (cost, forwarded) = result.unwrap();
        assert_eq!(cost, 700 + 9_000);
        assert!(forwarded < u64::MAX);
        assert!(forwarded > u64::MAX / 2);
    }
}
