use alloc::collections::BTreeMap;
use bare_metal_evm_types::U256;

use crate::constants::{TLOAD_GAS, TSTORE_GAS};
use crate::error::GasError;

#[derive(Debug, Clone, Default)]
pub struct TransientStorage {
    store: BTreeMap<([u8; 20], U256), U256>,
}

impl TransientStorage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the gas cost for a TLOAD.
    #[must_use]
    #[allow(dead_code)]
    pub fn cost_tload() -> u64 {
        TLOAD_GAS
    }

    /// Returns the gas cost for a TSTORE.
    #[must_use]
    #[allow(dead_code)]
    pub fn cost_tstore() -> u64 {
        TSTORE_GAS
    }

    /// Charge for TLOAD and return the gas cost. Use `get()` for the
    /// stored value.
    pub fn load(&mut self, _address: &[u8; 20], _slot: U256) -> Result<u64, GasError> {
        Ok(TLOAD_GAS)
    }

    /// Charge for TSTORE and write the value.
    pub fn store(&mut self, address: &[u8; 20], slot: U256, value: U256) -> Result<u64, GasError> {
        self.store.insert((*address, slot), value);
        Ok(TSTORE_GAS)
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
        ts.store(&ADDR, SLOT, U256::from_u64(42)).unwrap();
        assert_eq!(ts.get(&ADDR, SLOT), U256::from_u64(42));
    }

    #[test]
    fn transient_overwrite() {
        let mut ts = TransientStorage::new();
        ts.store(&ADDR, SLOT, U256::from_u64(1)).unwrap();
        ts.store(&ADDR, SLOT, U256::from_u64(2)).unwrap();
        assert_eq!(ts.get(&ADDR, SLOT), U256::from_u64(2));
    }

    #[test]
    fn transient_reset_clears() {
        let mut ts = TransientStorage::new();
        ts.store(&ADDR, SLOT, U256::from_u64(99)).unwrap();
        ts.reset();
        assert_eq!(ts.get(&ADDR, SLOT), U256::zero());
    }

    #[test]
    fn transient_cost_tload() {
        assert_eq!(TransientStorage::cost_tload(), TLOAD_GAS);
    }

    #[test]
    fn transient_cost_tstore() {
        assert_eq!(TransientStorage::cost_tstore(), TSTORE_GAS);
    }
}
