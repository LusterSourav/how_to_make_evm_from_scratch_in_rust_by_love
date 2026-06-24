use alloc::collections::BTreeMap;
use bare_metal_evm_types::U256;

use crate::constants::{TLOAD_GAS, TSTORE_GAS};

#[derive(Debug, Clone, Default)]
pub struct TransientStorage {
    store: BTreeMap<([u8; 20], U256), U256>,
}

impl TransientStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the gas cost for a TLOAD. Use `get()` to read the stored value.
    pub fn gas_cost_tload(&self) -> u64 {
        TLOAD_GAS
    }

    /// Charge for TSTORE and write the value.
    pub fn store(&mut self, address: &[u8; 20], slot: U256, value: U256) -> u64 {
        self.store.insert((*address, slot), value);
        TSTORE_GAS
    }

    /// Read the value at (address, slot). Returns zero if not set.
    #[must_use]
    pub fn get(&self, address: &[u8; 20], slot: U256) -> U256 {
        self.store
            .get(&(*address, slot))
            .copied()
            .unwrap_or(U256::zero())
    }

    /// Reset all transient storage. Called at the end of each transaction.
    pub fn reset(&mut self) {
        self.store.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDR: [u8; 20] = [0x01; 20];
    const SLOT: U256 = U256::zero();

    #[test]
    fn transient_get_default_zero() {
        let ts = TransientStorage::new();
        assert_eq!(ts.get(&ADDR, SLOT), U256::zero());
    }

    #[test]
    fn transient_store_and_get() {
        let mut ts = TransientStorage::new();
        ts.store(&ADDR, SLOT, U256::from_u64(42));
        assert_eq!(ts.get(&ADDR, SLOT), U256::from_u64(42));
    }

    #[test]
    fn transient_overwrite() {
        let mut ts = TransientStorage::new();
        ts.store(&ADDR, SLOT, U256::from_u64(1));
        ts.store(&ADDR, SLOT, U256::from_u64(2));
        assert_eq!(ts.get(&ADDR, SLOT), U256::from_u64(2));
    }

    #[test]
    fn transient_reset_clears() {
        let mut ts = TransientStorage::new();
        ts.store(&ADDR, SLOT, U256::from_u64(99));
        ts.reset();
        assert_eq!(ts.get(&ADDR, SLOT), U256::zero());
    }

    #[test]
    fn transient_cost_tload() {
        assert_eq!(TransientStorage::new().gas_cost_tload(), TLOAD_GAS);
    }

    #[test]
    fn transient_cost_tstore() {
        let mut ts = TransientStorage::new();
        let cost = ts.store(&[0x01; 20], U256::zero(), U256::from_u64(42));
        assert_eq!(cost, TSTORE_GAS);
    }

    #[test]
    fn transient_different_addresses() {
        let mut ts = TransientStorage::new();
        let addr_a = [0xAA; 20];
        let addr_b = [0xBB; 20];
        let slot = U256::zero();
        ts.store(&addr_a, slot, U256::from_u64(1));
        ts.store(&addr_b, slot, U256::from_u64(2));
        assert_eq!(ts.get(&addr_a, slot), U256::from_u64(1));
        assert_eq!(ts.get(&addr_b, slot), U256::from_u64(2));
    }
}
