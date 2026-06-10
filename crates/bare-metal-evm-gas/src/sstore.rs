use alloc::collections::BTreeMap;
use bare_metal_evm_types::U256;

use crate::constants::{
    SSTORE_CLEARS_SCHEDULE, SSTORE_RESET_GAS_EIP2929, SSTORE_SET_GAS, WARM_STORAGE_READ_COST,
};
use crate::error::GasError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SlotEntry {
    original: U256,
    current: U256,
}

#[derive(Debug, Clone, Default)]
pub struct SstoreTracker {
    slots: BTreeMap<([u8; 20], U256), SlotEntry>,
}

impl SstoreTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Charge gas for an SSTORE. Returns `(gas_cost, refund_delta)`.
    ///
    /// Implements the EIP-2200 truth table with EIP-2929 surcharges.
    /// Caller must check the sentry, apply the cold surcharge, and
    /// deduct `gas_cost` / add `refund_delta` themselves.
    pub fn charge_sstore(
        &mut self,
        address: &[u8; 20],
        slot: U256,
        new_value: U256,
    ) -> Result<(u64, i64), GasError> {
        let key = (*address, slot);
        let entry = self.slots.entry(key).or_insert_with(|| SlotEntry {
            original: U256::zero(),
            current: U256::zero(),
        });

        let original = entry.original;
        let current = entry.current;

        let (gas, refund) = compute_sstore_cost(original, current, new_value);

        entry.current = new_value;
        Ok((gas, refund))
    }

    /// Mark that the slot has been accessed. On first access, the given
    /// value becomes both original and current.
    #[allow(dead_code)]
    pub fn initialize_slot(&mut self, address: &[u8; 20], slot: U256, initial_value: U256) {
        let key = (*address, slot);
        self.slots.entry(key).or_insert(SlotEntry {
            original: initial_value,
            current: initial_value,
        });
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn get_slot_entry(&self, address: &[u8; 20], slot: U256) -> Option<(U256, U256)> {
        self.slots
            .get(&(*address, slot))
            .map(|e| (e.original, e.current))
    }
}

/// Compute SSTORE gas cost and refund delta per EIP-2200.
///
/// Truth table (post EIP-2929/3529):
///
/// | Original | Current | New | Cost  | Refund | Case               |
/// |----------|---------|-----|-------|--------|--------------------|
/// | 0        | 0       | 0   | 100   | 0      | Noop               |
/// | 0        | 0       | x   | 20000 | 0      | Clean set          |
/// | 0        | x       | 0   | 100   | +4800  | Dirty restore → 0  |
/// | 0        | x       | y   | 100   | 0      | Dirty modify       |
/// | x        | 0       | 0   | 100   | 0      | Noop               |
/// | x        | 0       | x   | 100   | 0      | Dirty restore → x  |
/// | x        | 0       | y   | 2900  | 0      | Clean modify       |
/// | x        | x       | 0   | 2900  | +4800  | Clean clear        |
/// | x        | x       | y   | 2900  | 0      | Clean modify       |
/// | x        | y       | 0   | 100   | +4800  | Dirty clear        |
/// | x        | y       | x   | 100   | 0      | Dirty restore → x  |
/// | x        | y       | z   | 100   | 0      | Dirty modify       |
fn compute_sstore_cost(original: U256, current: U256, new_value: U256) -> (u64, i64) {
    // Noop: current == new
    if current == new_value {
        return (WARM_STORAGE_READ_COST, 0);
    }

    // Clean case: original == current (slot hasn't been modified yet)
    if original == current {
        if original.is_zero() {
            // Clean set: 0 -> 0 -> x
            return (SSTORE_SET_GAS, 0);
        }
        if new_value.is_zero() {
            // Clean clear: x -> x -> 0
            return (SSTORE_RESET_GAS_EIP2929, SSTORE_CLEARS_SCHEDULE as i64);
        }
        // Clean modify: x -> x -> y
        return (SSTORE_RESET_GAS_EIP2929, 0);
    }

    // Dirty case: original != current (slot has been modified by this execution)
    let mut refund: i64 = 0;

    // If original is non-zero and we're clearing to zero, get a refund
    if !original.is_zero() && new_value.is_zero() {
        refund += SSTORE_CLEARS_SCHEDULE as i64;
    }

    // If original is non-zero and current was cleared (now zero), undo the clear
    if !original.is_zero() && current.is_zero() && !new_value.is_zero() {
        refund -= SSTORE_CLEARS_SCHEDULE as i64;
    }

    // If restoring to original value, get a refund
    if original == new_value {
        refund += SSTORE_CLEARS_SCHEDULE as i64;
    }

    (WARM_STORAGE_READ_COST, refund)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDR: [u8; 20] = [0x01; 20];
    const SLOT: U256 = U256::zero();

    fn v(val: u64) -> U256 {
        U256::from_u64(val)
    }

    // --- Noop scenarios ---

    #[test]
    fn sstore_noop_zero_zero_zero() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(0));
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(0)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        assert_eq!(refund, 0);
    }

    #[test]
    fn sstore_noop_nonzero_x_zero_zero() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(5));
        // First: x -> 0 (clear), cost=2900, refund=+4800
        t.charge_sstore(&ADDR, SLOT, v(0)).unwrap();
        // Second: 0 -> 0 (noop), cost=100, refund=0
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(0)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        assert_eq!(refund, 0);
    }

    #[test]
    fn sstore_noop_nonzero_x_x_x() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(5));
        // x -> x (noop)
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(5)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        assert_eq!(refund, 0);
    }

    // --- Clean scenarios ---

    #[test]
    fn sstore_clean_set_zero_zero_x() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(0));
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(5)).unwrap();
        assert_eq!(cost, SSTORE_SET_GAS);
        assert_eq!(refund, 0);
    }

    #[test]
    fn sstore_clean_clear_x_x_zero() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(5));
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(0)).unwrap();
        assert_eq!(cost, SSTORE_RESET_GAS_EIP2929);
        assert_eq!(refund, SSTORE_CLEARS_SCHEDULE as i64);
    }

    #[test]
    fn sstore_clean_modify_x_x_y() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(5));
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(10)).unwrap();
        assert_eq!(cost, SSTORE_RESET_GAS_EIP2929);
        assert_eq!(refund, 0);
    }

    // --- Dirty scenarios ---

    #[test]
    fn sstore_dirty_restore_to_zero_zero_x_zero() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(0));
        // 0 -> x (set)
        t.charge_sstore(&ADDR, SLOT, v(5)).unwrap();
        // x -> 0 (restore to zero)
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(0)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        assert_eq!(refund, SSTORE_CLEARS_SCHEDULE as i64);
    }

    #[test]
    fn sstore_dirty_modify_zero_x_y() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(0));
        // 0 -> x
        t.charge_sstore(&ADDR, SLOT, v(5)).unwrap();
        // x -> y
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(10)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        assert_eq!(refund, 0);
    }

    #[test]
    fn sstore_dirty_restore_to_original_x_zero_x() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(5));
        // x -> 0 (clear)
        t.charge_sstore(&ADDR, SLOT, v(0)).unwrap();
        // 0 -> x (restore)
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(5)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        // Refund: undo clear (-4800) + restore (+4800) = 0
        assert_eq!(refund, 0);
    }

    #[test]
    fn sstore_dirty_clear_x_y_zero() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(5));
        // x -> y
        t.charge_sstore(&ADDR, SLOT, v(10)).unwrap();
        // y -> 0 (dirty clear)
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(0)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        assert_eq!(refund, SSTORE_CLEARS_SCHEDULE as i64);
    }

    #[test]
    fn sstore_dirty_restore_to_original_x_y_x() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(5));
        // x -> y
        t.charge_sstore(&ADDR, SLOT, v(10)).unwrap();
        // y -> x (restore)
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(5)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        assert_eq!(refund, SSTORE_CLEARS_SCHEDULE as i64);
    }

    #[test]
    fn sstore_dirty_modify_x_y_z() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(5));
        // x -> y
        t.charge_sstore(&ADDR, SLOT, v(10)).unwrap();
        // y -> z
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(15)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        assert_eq!(refund, 0);
    }

    #[test]
    fn sstore_dirty_noop_x_y_y() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(5));
        // x -> y
        t.charge_sstore(&ADDR, SLOT, v(10)).unwrap();
        // y -> y (noop)
        let (cost, refund) = t.charge_sstore(&ADDR, SLOT, v(10)).unwrap();
        assert_eq!(cost, WARM_STORAGE_READ_COST);
        assert_eq!(refund, 0);
    }

    // --- Lookup helpers ---

    #[test]
    fn sstore_get_slot_entry_uninitialized() {
        let t = SstoreTracker::new();
        assert!(t.get_slot_entry(&ADDR, SLOT).is_none());
    }

    #[test]
    fn sstore_get_slot_entry_after_init() {
        let mut t = SstoreTracker::new();
        t.initialize_slot(&ADDR, SLOT, v(42));
        let (orig, curr) = t.get_slot_entry(&ADDR, SLOT).unwrap();
        assert_eq!(orig, v(42));
        assert_eq!(curr, v(42));
    }
}
