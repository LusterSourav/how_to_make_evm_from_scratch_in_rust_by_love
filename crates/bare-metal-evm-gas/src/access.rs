use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use bare_metal_evm_types::U256;

use crate::constants::{COLD_ACCOUNT_ACCESS_COST, COLD_SLOAD_COST, WARM_STORAGE_READ_COST};

/// A single entry in the EIP-2930 access list: an address and its
/// storage keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessListItem {
    /// The 20-byte address to pre-warm.
    pub address: [u8; 20],
    /// Storage keys (32-byte slot identifiers) to pre-warm for this address.
    pub storage_keys: Vec<[u8; 32]>,
}

/// Tracks warm and cold addresses and storage slots per EIP-2929.
/// Each new address costs 2600, each new slot 2100. Repeated touches
/// cost 100.
#[derive(Debug, Clone)]
pub struct AccessSet {
    addresses: BTreeSet<[u8; 20]>,
    storage_slots: BTreeMap<[u8; 20], BTreeSet<U256>>,
}

impl AccessSet {
    pub fn new() -> Self {
        Self {
            addresses: BTreeSet::new(),
            storage_slots: BTreeMap::new(),
        }
    }

    /// Warm up addresses and storage keys from the transaction access
    /// list (EIP-2930).
    pub fn initialize(&mut self, access_list: &[AccessListItem]) {
        for item in access_list {
            self.addresses.insert(item.address);
            let slots = self.storage_slots.entry(item.address).or_default();
            for key in &item.storage_keys {
                slots.insert(U256::from_bytes_be(*key));
            }
        }
    }

    /// Warm up defaults: origin, recipient, and the 9 precompile
    /// addresses (0x01 through 0x09). This ensures first-touches to
    /// these addresses only cost 100 instead of 2600.
    pub fn warm_defaults(&mut self, origin: &[u8; 20], to: Option<&[u8; 20]>) {
        self.addresses.insert(*origin);
        if let Some(to_addr) = to {
            self.addresses.insert(*to_addr);
        }
        for i in 1..=9u8 {
            let mut addr = [0u8; 20];
            addr[19] = i;
            self.addresses.insert(addr);
        }
    }

    /// Touch an address. Returns the gas cost: 2600 on first access,
    /// 100 if already warm.
    pub fn touch_address(&mut self, address: &[u8; 20]) -> u64 {
        if self.addresses.insert(*address) {
            COLD_ACCOUNT_ACCESS_COST
        } else {
            WARM_STORAGE_READ_COST
        }
    }

    /// Touch a storage slot. Returns the gas cost: 2100 on first
    /// access, 100 if already warm.
    pub fn touch_storage_slot(&mut self, address: &[u8; 20], slot: U256) -> u64 {
        let slots = self.storage_slots.entry(*address).or_default();
        if slots.insert(slot) {
            COLD_SLOAD_COST
        } else {
            WARM_STORAGE_READ_COST
        }
    }

    #[must_use]
    #[cfg(test)]
    pub fn is_warm_address(&self, address: &[u8; 20]) -> bool {
        self.addresses.contains(address)
    }

    #[must_use]
    #[cfg(test)]
    pub fn is_warm_storage_slot(&self, address: &[u8; 20], slot: U256) -> bool {
        self.storage_slots
            .get(address)
            .map(|slots| slots.contains(&slot))
            .unwrap_or(false)
    }
}

impl Default for AccessSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn test_addr() -> [u8; 20] {
        [0xAA; 20]
    }

    fn test_slot() -> U256 {
        U256::from_u64(42)
    }

    #[test]
    fn access_set_fresh_is_cold() {
        let set = AccessSet::new();
        assert!(!set.is_warm_address(&test_addr()));
        assert!(!set.is_warm_storage_slot(&test_addr(), test_slot()));
    }

    #[test]
    fn access_set_touch_address_cold_first() {
        let mut set = AccessSet::new();
        assert_eq!(set.touch_address(&test_addr()), COLD_ACCOUNT_ACCESS_COST);
        assert!(set.is_warm_address(&test_addr()));
    }

    #[test]
    fn access_set_touch_address_warm_second() {
        let mut set = AccessSet::new();
        set.touch_address(&test_addr());
        assert_eq!(set.touch_address(&test_addr()), WARM_STORAGE_READ_COST);
    }

    #[test]
    fn access_set_touch_storage_slot_cold_first() {
        let mut set = AccessSet::new();
        assert_eq!(
            set.touch_storage_slot(&test_addr(), test_slot()),
            COLD_SLOAD_COST
        );
        assert!(set.is_warm_storage_slot(&test_addr(), test_slot()));
    }

    #[test]
    fn access_set_touch_storage_slot_warm_second() {
        let mut set = AccessSet::new();
        set.touch_storage_slot(&test_addr(), test_slot());
        assert_eq!(
            set.touch_storage_slot(&test_addr(), test_slot()),
            WARM_STORAGE_READ_COST
        );
    }

    #[test]
    fn access_set_initialize_warms_address_and_slots() {
        let key = [0xFFu8; 32];
        let items = vec![AccessListItem {
            address: test_addr(),
            storage_keys: vec![key],
        }];
        let mut set = AccessSet::new();
        set.initialize(&items);
        assert!(set.is_warm_address(&test_addr()));
        assert!(set.is_warm_storage_slot(&test_addr(), U256::from_bytes_be(key)));
    }

    #[test]
    fn access_set_diff_addresses_independent() {
        let mut set = AccessSet::new();
        let a = [1u8; 20];
        let b = [2u8; 20];
        set.touch_address(&a);
        assert!(set.is_warm_address(&a));
        assert!(!set.is_warm_address(&b));
    }

    #[test]
    fn access_set_diff_slots_independent() {
        let mut set = AccessSet::new();
        let s1 = U256::from_u64(1);
        let s2 = U256::from_u64(2);
        set.touch_storage_slot(&test_addr(), s1);
        assert!(set.is_warm_storage_slot(&test_addr(), s1));
        assert!(!set.is_warm_storage_slot(&test_addr(), s2));
    }

    #[test]
    fn access_set_precompile_addresses() {
        let mut set = AccessSet::new();
        let mut precompile = [0u8; 20];
        precompile[19] = 1;
        assert_eq!(set.touch_address(&precompile), COLD_ACCOUNT_ACCESS_COST);
        assert_eq!(set.touch_address(&precompile), WARM_STORAGE_READ_COST);
    }

    #[test]
    fn access_set_warm_defaults_warms_origin() {
        let origin = [0xBB; 20];
        let mut set = AccessSet::new();
        set.warm_defaults(&origin, None);
        assert!(set.is_warm_address(&origin));
        assert_eq!(set.touch_address(&origin), WARM_STORAGE_READ_COST);
    }

    #[test]
    fn access_set_warm_defaults_warms_to() {
        let origin = [0xBB; 20];
        let to = [0xCC; 20];
        let mut set = AccessSet::new();
        set.warm_defaults(&origin, Some(&to));
        assert!(set.is_warm_address(&to));
        assert_eq!(set.touch_address(&to), WARM_STORAGE_READ_COST);
    }

    #[test]
    fn access_set_warm_defaults_warms_precompiles() {
        let origin = [0xBB; 20];
        let mut set = AccessSet::new();
        set.warm_defaults(&origin, None);
        for i in 1..=9u8 {
            let mut addr = [0u8; 20];
            addr[19] = i;
            assert!(set.is_warm_address(&addr), "precompile {i} should be warm");
        }
    }

    #[test]
    fn access_set_initialize_then_touch_cost() {
        let key = [0xFFu8; 32];
        let items = alloc::vec![AccessListItem {
            address: test_addr(),
            storage_keys: alloc::vec![key],
        }];
        let mut set = AccessSet::new();
        set.initialize(&items);
        assert_eq!(set.touch_address(&test_addr()), WARM_STORAGE_READ_COST);
        assert_eq!(
            set.touch_storage_slot(&test_addr(), U256::from_bytes_be(key)),
            WARM_STORAGE_READ_COST
        );
    }

    #[test]
    fn access_set_no_precompile_zero() {
        let set = AccessSet::new();
        let addr_zero = [0u8; 20];
        // 0x00 is NOT a precompile and must NOT be auto-warmed
        assert!(!set.is_warm_address(&addr_zero));
    }
}
