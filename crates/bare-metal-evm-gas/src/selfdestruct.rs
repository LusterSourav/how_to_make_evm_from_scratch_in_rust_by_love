use crate::constants::{CALL_VALUE_TRANSFER_GAS, SELFDESTRUCT_GAS_EIP150};
use crate::error::GasError;

/// Compute the SELFDESTRUCT base gas cost (does NOT include
/// beneficiary address access — use `GasMeter::charge_selfdestruct`
/// which charges both atomically).
///
/// Base cost = 5_000 + 9_000 (if value > 0)
pub fn selfdestruct_gas(has_value: bool) -> Result<u64, GasError> {
    let mut cost = SELFDESTRUCT_GAS_EIP150;
    if has_value {
        cost = cost
            .checked_add(CALL_VALUE_TRANSFER_GAS)
            .ok_or(GasError::Overflow)?;
    }
    Ok(cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selfdestruct_no_value() {
        assert_eq!(selfdestruct_gas(false).unwrap(), 5_000);
    }

    #[test]
    fn selfdestruct_with_value() {
        assert_eq!(selfdestruct_gas(true).unwrap(), 5_000 + 9_000);
    }
}
