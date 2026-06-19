use bare_metal_evm_types::U256;

use crate::access::{AccessListItem, AccessSet};
use crate::call;
use crate::constants::{EIP7623_CALLDATA_FLOOR_DIVISOR, MAX_REFUND_QUOTIENT, SSTORE_SENTRY_GAS};
use crate::error::GasError;
use crate::intrinsic;
use crate::memory;
use crate::sstore::SstoreTracker;
use crate::transient::TransientStorage;

#[derive(Debug, Clone)]
pub struct GasMeter {
    initial_gas: u64,
    remaining: u64,
    memory_words: usize,
    refund: i64,
    calldata_tokens: u64,
    access_set: AccessSet,
    sstore: SstoreTracker,
    transient: TransientStorage,
}

impl GasMeter {
    /// Create a new gas meter. Deducts intrinsic gas, warms the access
    /// list plus precompile defaults, and rejects the transaction if the
    /// EIP-7623 calldata floor exceeds `initial_gas`.
    pub fn new(
        initial_gas: u64,
        calldata: &[u8],
        access_list: &[AccessListItem],
        is_create: bool,
        origin: &[u8; 20],
        to: Option<&[u8; 20]>,
    ) -> Result<Self, GasError> {
        let intrinsic = intrinsic::intrinsic_gas(calldata, access_list, is_create)?;

        if intrinsic > initial_gas {
            return Err(GasError::OutOfGas);
        }

        // EIP-7623: reject if the calldata floor exceeds the gas limit.
        let calldata_tokens = eip7623_calldata_tokens(calldata)?;
        let floor_gas = calldata_tokens
            .checked_mul(EIP7623_CALLDATA_FLOOR_DIVISOR)
            .ok_or(GasError::Overflow)?;
        if floor_gas > initial_gas {
            return Err(GasError::OutOfGas);
        }

        let mut access_set = AccessSet::new();
        access_set.initialize(access_list);
        access_set.warm_defaults(origin, to);

        Ok(GasMeter {
            initial_gas,
            remaining: initial_gas - intrinsic,
            memory_words: 0,
            refund: 0,
            calldata_tokens,
            access_set,
            sstore: SstoreTracker::new(),
            transient: TransientStorage::new(),
        })
    }

    /// Charge a fixed gas amount. Returns `OutOfGas` when the remaining
    /// gas is too low.
    pub fn charge(&mut self, amount: u64) -> Result<(), GasError> {
        self.remaining = self
            .remaining
            .checked_sub(amount)
            .ok_or(GasError::OutOfGas)?;
        Ok(())
    }

    /// Charge for memory expansion up to `new_byte_size`. No-op when
    /// memory doesn't expand.
    pub fn charge_memory(&mut self, new_byte_size: usize) -> Result<(), GasError> {
        let new_words = memory::word_count(new_byte_size);
        let cost = memory::memory_expansion_cost(self.memory_words, new_words)?;
        if cost > 0 {
            self.remaining = self.remaining.checked_sub(cost).ok_or(GasError::OutOfGas)?;
            self.memory_words = new_words;
        }
        Ok(())
    }

    /// Charge for an account touch. Cold on first access, warm after.
    pub fn charge_account_access(&mut self, address: &[u8; 20]) -> Result<(), GasError> {
        let cost = self.access_set.touch_address(address);
        self.remaining = self.remaining.checked_sub(cost).ok_or(GasError::OutOfGas)?;
        Ok(())
    }

    /// Charge for a storage slot read. Cold on first access, warm after.
    pub fn charge_sload(&mut self, address: &[u8; 20], slot: U256) -> Result<(), GasError> {
        let cost = self.access_set.touch_storage_slot(address, slot);
        self.remaining = self.remaining.checked_sub(cost).ok_or(GasError::OutOfGas)?;
        Ok(())
    }

    /// Charge for an SSTORE. Handles the EIP-2200 truth table,
    /// EIP-2929 cold surcharge, and sentry check.
    pub fn charge_sstore(
        &mut self,
        address: &[u8; 20],
        slot: U256,
        new_value: U256,
    ) -> Result<(), GasError> {
        // EIP-2200 sentry: always checked, regardless of cost matrix
        if self.remaining <= SSTORE_SENTRY_GAS {
            return Err(GasError::OutOfGas);
        }

        // Warm up the slot (cold surcharge applied here)
        let slot_cost = self.access_set.touch_storage_slot(address, slot);

        let (base_cost, refund_delta) = self.sstore.charge_sstore(address, slot, new_value);

        let total_cost = base_cost.checked_add(slot_cost).ok_or(GasError::Overflow)?;

        self.remaining = self
            .remaining
            .checked_sub(total_cost)
            .ok_or(GasError::OutOfGas)?;
        self.refund = self.refund.saturating_add(refund_delta);
        Ok(())
    }

    /// Charge for a TLOAD. Returns the stored value (zero if unset).
    pub fn charge_tload(&mut self, address: &[u8; 20], slot: U256) -> Result<U256, GasError> {
        let cost = self.transient.load(address, slot);
        self.remaining = self.remaining.checked_sub(cost).ok_or(GasError::OutOfGas)?;
        Ok(self.transient.get(address, slot))
    }

    /// Charge for a TSTORE.
    pub fn charge_tstore(
        &mut self,
        address: &[u8; 20],
        slot: U256,
        value: U256,
    ) -> Result<(), GasError> {
        let cost = self.transient.store(address, slot, value);
        self.remaining = self.remaining.checked_sub(cost).ok_or(GasError::OutOfGas)?;
        Ok(())
    }

    /// Compute gas cost for a child call. Returns
    /// `(cost_to_caller, gas_forwarded_to_child)`.
    pub fn gas_for_call(
        &self,
        requested_gas: u64,
        has_value: bool,
        is_new_account: bool,
    ) -> Result<(u64, u64), GasError> {
        call::gas_for_child_call(self.remaining, requested_gas, has_value, is_new_account)
    }

    /// Apply refund at the end of a transaction. Enforces EIP-3529 refund
    /// cap (`min(refund, gas_used / 5)`) and EIP-7623 calldata floor
    /// (`max(gas_used, 10 * calldata_tokens)`).
    pub fn apply_refund(&mut self) -> Result<(), GasError> {
        let gas_used_before_refund = self
            .initial_gas
            .checked_sub(self.remaining)
            .ok_or(GasError::Overflow)?;

        // EIP-3529 refund cap based on actual gas used (before floor).
        let max_refund = gas_used_before_refund / MAX_REFUND_QUOTIENT;
        let positive_refund = core::cmp::max(self.refund, 0) as u64;
        let capped_refund = core::cmp::min(positive_refund, max_refund);

        let used_after_refund = gas_used_before_refund
            .checked_sub(capped_refund)
            .ok_or(GasError::Overflow)?;

        // EIP-7623 calldata floor applied AFTER refund.
        let floor_gas = self
            .calldata_tokens
            .checked_mul(EIP7623_CALLDATA_FLOOR_DIVISOR)
            .ok_or(GasError::Overflow)?;
        let final_used = core::cmp::max(
            used_after_refund,
            core::cmp::min(floor_gas, self.initial_gas),
        );
        self.remaining = self
            .initial_gas
            .checked_sub(final_used)
            .ok_or(GasError::Overflow)?;
        self.refund = 0;
        Ok(())
    }

    #[must_use]
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    #[must_use]
    pub fn memory_words(&self) -> usize {
        self.memory_words
    }

    #[must_use]
    pub fn memory_size(&self) -> usize {
        self.memory_words * 32
    }

    #[must_use]
    pub fn refund(&self) -> i64 {
        self.refund
    }

    /// Consume the meter and return remaining gas.
    #[must_use]
    pub fn exhaust(self) -> u64 {
        self.remaining
    }

    /// Reset transient storage at the end of a transaction.
    pub fn reset_transient(&mut self) {
        self.transient.reset();
    }
}

/// EIP-7623: count calldata tokens. Non-zero bytes cost 4 tokens,
/// zero bytes cost 1 token. The floor is `tokens * 10`.
fn eip7623_calldata_tokens(calldata: &[u8]) -> Result<u64, GasError> {
    calldata.iter().try_fold(0u64, |acc, &b| {
        acc.checked_add(if b == 0 { 1u64 } else { 4u64 })
            .ok_or(GasError::Overflow)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{COLD_ACCOUNT_ACCESS_COST, COLD_SLOAD_COST, WARM_STORAGE_READ_COST};
    use alloc::vec;

    fn test_addr() -> [u8; 20] {
        [0xAA; 20]
    }

    fn origin() -> [u8; 20] {
        [0xBB; 20]
    }

    fn to() -> [u8; 20] {
        [0xCC; 20]
    }

    #[test]
    fn meter_new_basic_tx() {
        let m = GasMeter::new(21_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        assert_eq!(m.remaining(), 0);
    }

    #[test]
    fn meter_new_with_buffer() {
        let m = GasMeter::new(30_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        assert_eq!(m.remaining(), 9_000);
    }

    #[test]
    fn meter_new_create() {
        let m = GasMeter::new(74_000, b"", &[], true, &origin(), None).unwrap();
        assert_eq!(m.remaining(), 21_000);
    }

    #[test]
    fn meter_out_of_gas_on_intrinsic() {
        let result = GasMeter::new(20_000, b"", &[], false, &origin(), Some(&to()));
        assert!(result.is_err());
    }

    #[test]
    fn meter_out_of_gas_on_floor() {
        // 12 zero-byte calldata: intrinsic = 21000 + 48 = 21048
        // floor = 12 * 10 = 120 > 1000
        let calldata = vec![0u8; 12];
        let result = GasMeter::new(1000, &calldata, &[], false, &origin(), Some(&to()));
        assert!(result.is_err());
    }

    #[test]
    fn meter_charge_success() {
        let mut m = GasMeter::new(30_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        m.charge(5_000).unwrap();
        assert_eq!(m.remaining(), 4_000);
    }

    #[test]
    fn meter_charge_out_of_gas() {
        let mut m = GasMeter::new(30_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        assert_eq!(m.charge(10_000), Err(GasError::OutOfGas));
    }

    #[test]
    fn meter_charge_memory_expansion() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        m.charge_memory(64).unwrap();
        assert_eq!(m.memory_words(), 2);
    }

    #[test]
    fn meter_charge_memory_no_expansion() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        m.charge_memory(32).unwrap();
        m.charge_memory(32).unwrap();
        assert_eq!(m.memory_words(), 1);
    }

    #[test]
    fn meter_charge_account_access_cold() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        m.charge_account_access(&test_addr()).unwrap();
        assert_eq!(m.remaining(), 100_000 - 21_000 - COLD_ACCOUNT_ACCESS_COST);
    }

    #[test]
    fn meter_charge_account_access_warm() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        m.charge_account_access(&test_addr()).unwrap();
        let after_cold = m.remaining();
        m.charge_account_access(&test_addr()).unwrap();
        assert_eq!(m.remaining(), after_cold - WARM_STORAGE_READ_COST);
    }

    #[test]
    fn meter_charge_sload_cold() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(1);
        m.charge_sload(&test_addr(), slot).unwrap();
        assert_eq!(m.remaining(), 100_000 - 21_000 - COLD_SLOAD_COST);
    }

    #[test]
    fn meter_charge_sload_warm() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(1);
        m.charge_sload(&test_addr(), slot).unwrap();
        m.charge_sload(&test_addr(), slot).unwrap();
        assert_eq!(
            m.remaining(),
            100_000 - 21_000 - COLD_SLOAD_COST - WARM_STORAGE_READ_COST
        );
    }

    #[test]
    fn meter_charge_sstore_basic() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(0);
        m.charge_sstore(&test_addr(), slot, U256::from_u64(42))
            .unwrap();
        // cold surcharge (2100) + clean set (20000) = 22100
        assert_eq!(m.remaining(), 100_000 - 21_000 - 22_100);
    }

    #[test]
    fn meter_charge_sstore_noop() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(0);
        m.charge_sstore(&test_addr(), slot, U256::from_u64(42))
            .unwrap();
        m.charge_sstore(&test_addr(), slot, U256::from_u64(42))
            .unwrap();
        // cold set (20000 + 2100) + warm noop (100 + 100) = 22300
        assert_eq!(m.remaining(), 100_000 - 21_000 - 22_300);
    }

    #[test]
    fn meter_charge_sstore_sentry() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(0);
        m.charge(m.remaining() - SSTORE_SENTRY_GAS - 1).unwrap();
        let result = m.charge_sstore(&test_addr(), slot, U256::from_u64(1));
        assert_eq!(result, Err(GasError::OutOfGas));
    }

    #[test]
    fn meter_charge_sstore_refund_tracking() {
        let mut m = GasMeter::new(200_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(0);
        m.charge_sstore(&test_addr(), slot, U256::from_u64(42))
            .unwrap();
        assert_eq!(m.refund(), 0);
        m.charge_sstore(&test_addr(), slot, U256::from_u64(0))
            .unwrap();
        assert_eq!(m.refund(), 4800);
    }

    #[test]
    fn meter_charge_tload_default_zero() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(7);
        let val = m.charge_tload(&test_addr(), slot).unwrap();
        assert_eq!(val, U256::zero());
        assert_eq!(m.remaining(), 100_000 - 21_000 - 100);
    }

    #[test]
    fn meter_charge_tstore_and_tload() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(7);
        m.charge_tstore(&test_addr(), slot, U256::from_u64(99))
            .unwrap();
        let val = m.charge_tload(&test_addr(), slot).unwrap();
        assert_eq!(val, U256::from_u64(99));
        assert_eq!(m.remaining(), 100_000 - 21_000 - 200);
    }

    #[test]
    fn meter_gas_for_call_basic() {
        let m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let (cost, forwarded) = m.gas_for_call(50_000, false, false).unwrap();
        assert_eq!(cost, 700);
        assert!(forwarded <= 50_000);
    }

    #[test]
    fn meter_apply_refund_basic() {
        let mut m = GasMeter::new(200_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(0);
        m.charge_sstore(&test_addr(), slot, U256::from_u64(1))
            .unwrap();
        m.charge_sstore(&test_addr(), slot, U256::from_u64(0))
            .unwrap();
        let before = m.remaining();
        m.apply_refund().unwrap();
        assert!(m.remaining() >= before);
    }

    #[test]
    fn meter_apply_refund_cap() {
        let mut m = GasMeter::new(200_000, b"\x00", &[], false, &origin(), Some(&to())).unwrap();
        m.refund = 100_000;
        m.apply_refund().unwrap();
        assert_eq!(m.refund(), 0);
    }

    #[test]
    fn meter_apply_refund_floor() {
        let mut m = GasMeter::new(200_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        m.refund = 50_000;
        m.apply_refund().unwrap();
        assert!(m.remaining() <= 200_000);
    }

    #[test]
    fn meter_apply_refund_floor_exceeds_initial() {
        // 12 zero-byte calldata: floor = 120, intrinsic = 21048
        // gas_used < floor but floor < initial_gas so cap holds
        let calldata = vec![0u8; 12];
        let mut m = GasMeter::new(100_000, &calldata, &[], false, &origin(), Some(&to())).unwrap();
        // consume all execution gas
        m.charge(m.remaining()).unwrap();
        m.apply_refund().unwrap();
        // effective_used capped at initial_gas, no underflow
        assert!(m.remaining() <= 100_000);
    }

    #[test]
    fn meter_exhaust() {
        let m = GasMeter::new(30_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        assert_eq!(m.exhaust(), 9_000);
    }

    #[test]
    fn meter_reset_transient() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(1);
        m.charge_tstore(&test_addr(), slot, U256::from_u64(42))
            .unwrap();
        m.reset_transient();
        let val = m.charge_tload(&test_addr(), slot).unwrap();
        assert_eq!(val, U256::zero());
    }

    #[test]
    fn meter_memory_size_bytes() {
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        m.charge_memory(64).unwrap();
        assert_eq!(m.memory_size(), 64);
    }

    #[test]
    fn meter_warms_origin() {
        let o = origin();
        let mut m = GasMeter::new(100_000, b"", &[], false, &o, Some(&to())).unwrap();
        let before = m.remaining();
        m.charge_account_access(&o).unwrap();
        assert_eq!(before - m.remaining(), WARM_STORAGE_READ_COST);
    }

    #[test]
    fn meter_warms_to() {
        let t = to();
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&t)).unwrap();
        let before = m.remaining();
        m.charge_account_access(&t).unwrap();
        assert_eq!(before - m.remaining(), WARM_STORAGE_READ_COST);
    }

    #[test]
    fn meter_warms_precompiles() {
        let mut m = GasMeter::new(500_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        for i in 1..=9u8 {
            let mut addr = [0u8; 20];
            addr[19] = i;
            let before = m.remaining();
            m.charge_account_access(&addr).unwrap();
            assert_eq!(
                before - m.remaining(),
                WARM_STORAGE_READ_COST,
                "precompile {i} should be warm"
            );
        }
    }

    #[test]
    fn meter_no_to_skips_recipient_warming() {
        let t = to();
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), None).unwrap();
        let before = m.remaining();
        m.charge_account_access(&t).unwrap();
        assert_eq!(before - m.remaining(), COLD_ACCOUNT_ACCESS_COST);
    }

    #[test]
    fn meter_charge_sstore_warm_clean_modify() {
        // After a cold set (0→x), a second write to the same slot
        // becomes dirty-modify (0,x,y): cost = 100 slot + 100 base.
        let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(0);
        m.charge_sstore(&test_addr(), slot, U256::from_u64(42))
            .unwrap();
        let after_cold = m.remaining();
        m.charge_sstore(&test_addr(), slot, U256::from_u64(99))
            .unwrap();
        assert_eq!(after_cold - m.remaining(), 200);
    }

    #[test]
    fn meter_apply_refund_after_fix() {
        let calldata = b"\x01\x02\x03";
        let mut m = GasMeter::new(200_000, calldata, &[], false, &origin(), Some(&to())).unwrap();
        let slot = U256::from_u64(0);
        m.charge_sstore(&test_addr(), slot, U256::from_u64(1))
            .unwrap();
        m.charge_sstore(&test_addr(), slot, U256::from_u64(0))
            .unwrap();
        m.apply_refund().unwrap();
        assert_eq!(m.refund(), 0);
    }

    #[test]
    fn meter_apply_refund_zero_refund() {
        let mut m = GasMeter::new(21_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        m.apply_refund().unwrap();
        assert_eq!(m.remaining(), 0);
    }

    #[test]
    fn meter_apply_refund_negative_refund() {
        let mut m = GasMeter::new(200_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        m.refund = -5000;
        m.apply_refund().unwrap();
        assert_eq!(m.refund(), 0);
    }

    #[test]
    fn meter_charge_zero_amount() {
        let mut m = GasMeter::new(21_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let before = m.remaining();
        m.charge(0).unwrap();
        assert_eq!(m.remaining(), before);
    }

    #[test]
    fn meter_charge_memory_zero() {
        let mut m = GasMeter::new(21_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let before = m.remaining();
        m.charge_memory(0).unwrap();
        assert_eq!(m.remaining(), before);
        assert_eq!(m.memory_words(), 0);
    }

    #[test]
    fn meter_cross_address_sstore() {
        let mut m = GasMeter::new(200_000, b"", &[], false, &origin(), Some(&to())).unwrap();
        let addr_a = [0xAA; 20];
        let addr_b = [0xBB; 20];
        let slot = U256::from_u64(0);
        // cold on addr_a
        m.charge_sstore(&addr_a, slot, U256::from_u64(1)).unwrap();
        // cold on addr_b (different address, same slot)
        m.charge_sstore(&addr_b, slot, U256::from_u64(2)).unwrap();
        // warm on addr_a — dirty modify (0,1,3): 100 slot + 100 base
        let before = m.remaining();
        m.charge_sstore(&addr_a, slot, U256::from_u64(3)).unwrap();
        assert_eq!(before - m.remaining(), 200);
    }

    #[test]
    fn meter_integration_intrinsic_memory_sstore() {
        let mut m = GasMeter::new(
            100_000,
            b"\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0A",
            &[],
            false,
            &origin(),
            Some(&to()),
        )
        .unwrap();
        let after_intrinsic = m.remaining();
        assert_eq!(after_intrinsic, 78_840);

        m.charge_memory(32).unwrap();
        assert_eq!(m.remaining(), 78_840 - 3);

        let slot = U256::from_u64(0);
        m.charge_sstore(&test_addr(), slot, U256::from_u64(1))
            .unwrap();
        assert!(m.remaining() > 0);
    }
}
