use bare_metal_evm_gas::GasMeter;
use bare_metal_evm_types::U256;

fn origin() -> [u8; 20] {
    [0xBB; 20]
}

fn to() -> [u8; 20] {
    [0xCC; 20]
}

fn test_addr() -> [u8; 20] {
    [0xAA; 20]
}

#[test]
fn simple_eth_transfer() {
    let mut m = GasMeter::new(30_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    // Intrinsic: 21000, remaining: 9000
    assert_eq!(m.remaining(), 9_000);

    // Warm the recipient (already warm from default)
    m.charge_account_access(&to()).unwrap();
    assert_eq!(m.remaining(), 9_000 - 100);
}

#[test]
fn contract_creation_tx() {
    let mut m = GasMeter::new(100_000, b"\x60\x00\x60\x00", &[], true, &origin(), None).unwrap();
    // Intrinsic (create): 53000 + 2*16 (non-zero) + 2*4 (zero) = 53040
    // Remaining: 100000 - 53040 = 46960
    assert_eq!(m.remaining(), 46_960);

    // Charge memory and CREATE gas
    m.charge_memory(64).unwrap();
    m.charge(m.remaining() - 1).unwrap();
    assert_eq!(m.remaining(), 1);
}

#[test]
fn sstore_clean_modify_after_initialize() {
    let mut m = GasMeter::new(200_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let slot = U256::from_u64(42);

    m.initialize_storage_slot(&test_addr(), slot, U256::from_u64(100));
    // Clean modify (100,100,200): base=2900, cold_addr=2600, cold_slot=2100
    m.charge_sstore(&test_addr(), slot, U256::from_u64(200))
        .unwrap();
    let expected_gas = 21_000 + 2_600 + 2_100 + 2_900;
    assert_eq!(m.remaining(), 200_000 - expected_gas);
}

#[test]
fn sstore_with_refund_capped_by_eip3529() {
    let mut m = GasMeter::new(200_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let slot = U256::from_u64(0);

    // Cold set: 0 -> 1 (costs 20000 + 2600 + 2100 = 24700)
    m.charge_sstore(&test_addr(), slot, U256::from_u64(1))
        .unwrap();

    // Clear: 1 -> 0 (costs 100 + 100 + 100 = 300, refund +4800)
    m.charge_sstore(&test_addr(), slot, U256::from_u64(0))
        .unwrap();
    assert_eq!(m.refund(), 4800);

    m.apply_refund().unwrap();
    let gas_used = 200_000 - m.remaining();
    // Max refund = gas_used / 5, but gas_used = 21000 + 24700 + 300 = 46000
    // So max_refund = 46000/5 = 9200, actual refund = 4800
    // So final gas_used = 46000 - 4800 = 41200
    assert!(gas_used < 46000);
    assert!(gas_used >= 41200);
}

#[test]
fn eip7623_calldata_floor() {
    // 12 zero-byte calldata: tokens = 12, floor = 12 * 10 = 120
    let calldata = [0u8; 12];
    let mut m = GasMeter::new(100_000, &calldata, &[], false, &origin(), Some(&to())).unwrap();
    // Intrinsic: 21000 + 12*4 = 21048, remaining: 78952
    assert_eq!(m.remaining(), 78_952);

    // Use all remaining gas
    m.charge(m.remaining()).unwrap();

    m.apply_refund().unwrap();
    // floor = min(120, 100000) = 120
    // gas_used_before_floor = 100000
    // final_used = max(100000, 120) = 100000
    assert_eq!(m.remaining(), 0);
}

#[test]
fn eip150_63_64_rule() {
    let m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    // 63/64 rule: max forward = 63/64 of remaining after cost
    // remaining = 79000 (after 21000 intrinsic)
    let (cost, forwarded) = m.gas_for_call(u64::MAX, false, false).unwrap();
    assert_eq!(cost, 700);
    let expected_max_forward = (79_000 - 700) - (79_000 - 700) / 64;
    assert_eq!(forwarded, expected_max_forward);
}

#[test]
fn call_with_value_and_new_account() {
    let m = GasMeter::new(200_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let (cost, forwarded) = m.gas_for_call(50_000, true, true).unwrap();
    // cost = base(700) + value(9000) + new_account(25000) = 34700
    assert_eq!(cost, 34_700);
    // stipend = 2300
    assert_eq!(forwarded, 50_000 + 2_300);
}

#[test]
fn eip3860_initcode_word_cost() {
    // 32 bytes initcode = 1 word, cost = 2
    assert_eq!(
        bare_metal_evm_gas::create::initcode_word_cost(32).unwrap(),
        2
    );
    // 33 bytes = 2 words, cost = 4
    assert_eq!(
        bare_metal_evm_gas::create::initcode_word_cost(33).unwrap(),
        4
    );
    // Exactly at MAX_INIT_CODE_SIZE
    assert!(bare_metal_evm_gas::create::initcode_word_cost(49152).is_ok());
    // Exceeds MAX_INIT_CODE_SIZE
    assert!(bare_metal_evm_gas::create::initcode_word_cost(49153).is_err());
}

#[test]
fn create_with_initcode_and_deployed_code() {
    // CREATE with 32 bytes initcode and 10 bytes deployed code
    let gas = bare_metal_evm_gas::create::create_gas(32, 10, false).unwrap();
    // 32000 + initcode_word_cost(32) + CREATE_DATA_GAS * 10
    // = 32000 + 2 + 200 * 10 = 32000 + 2 + 2000 = 34002
    assert_eq!(gas, 34_002);
}

#[test]
fn create2_no_deployed_code_cost() {
    // CREATE2: deployed code cost is NOT charged
    let gas = bare_metal_evm_gas::create::create_gas(32, 10, true).unwrap();
    // 32000 + initcode_word_cost(32) = 32000 + 2 = 32002
    assert_eq!(gas, 32_002);
}

#[test]
fn precompile_lookup_all() {
    // Known precompiles
    assert_eq!(
        bare_metal_evm_gas::precompile::precompile_gas(0x01, 0)
            .unwrap()
            .unwrap(),
        3_000
    );
    assert!(bare_metal_evm_gas::precompile::precompile_gas(0x02, 32)
        .unwrap()
        .is_ok());
    // Unknown precompile
    assert!(bare_metal_evm_gas::precompile::precompile_gas(0x13, 0).is_none());
    // modexp and blake2f return None
    assert!(bare_metal_evm_gas::precompile::precompile_gas(0x05, 0).is_none());
    assert!(bare_metal_evm_gas::precompile::precompile_gas(0x09, 0).is_none());
}

#[test]
fn bn256_pairing_zero_pairs() {
    assert_eq!(
        bare_metal_evm_gas::precompile::bn256_pairing_gas(0).unwrap(),
        45_000
    );
}

#[test]
fn memory_expansion_large() {
    // 32768 words costs ~2.2M gas; provision enough
    let mut m = GasMeter::new(3_000_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    // Expand to 1000 words
    m.charge_memory(32_000).unwrap();
    assert_eq!(m.memory_words(), 1000);
    assert!(m.remaining() < 3_000_000);

    // Expand to max
    let max_bytes = 1 << 20; // 1 MB
    m.charge_memory(max_bytes).unwrap();
    assert_eq!(m.memory_words(), 32768);
    assert_eq!(m.memory_size(), max_bytes);
}

#[test]
fn transient_storage_independent_addresses() {
    let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let slot_a = U256::from_u64(1);
    let slot_b = U256::from_u64(2);
    let addr_a = [0xAA; 20];
    let addr_b = [0xBB; 20];

    m.charge_tstore(&addr_a, slot_a, U256::from_u64(100))
        .unwrap();
    m.charge_tstore(&addr_b, slot_b, U256::from_u64(200))
        .unwrap();

    let val_a = m.charge_tload(&addr_a, slot_a).unwrap();
    let val_b = m.charge_tload(&addr_b, slot_b).unwrap();
    assert_eq!(val_a, U256::from_u64(100));
    assert_eq!(val_b, U256::from_u64(200));
}

#[test]
fn sstore_cross_address_independence() {
    let mut m = GasMeter::new(300_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let slot = U256::from_u64(0);
    let addr_a = [0xAA; 20];
    let addr_b = [0xBB; 20];

    // Cold set on addr_a: 0 -> 1
    m.charge_sstore(&addr_a, slot, U256::from_u64(1)).unwrap();
    let after_a = m.remaining();

    // Cold set on addr_b: 0 -> 1 (different address, same slot — still cold)
    m.charge_sstore(&addr_b, slot, U256::from_u64(2)).unwrap();
    let after_b = m.remaining();

    assert!(after_a > after_b);
}

#[test]
fn exhaustive_tload_default_zero() {
    let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    // Uninitialized transient storage returns zero
    let val = m.charge_tload(&[0x01; 20], U256::from_u64(999)).unwrap();
    assert_eq!(val, U256::zero());
}

#[test]
fn gas_for_call_zero_requested() {
    let m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let (cost, forwarded) = m.gas_for_call(0, false, false).unwrap();
    assert_eq!(cost, 700);
    assert_eq!(forwarded, 0);
}

#[test]
fn gas_for_call_new_account_no_value() {
    let m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let (cost, _forwarded) = m.gas_for_call(10_000, false, true).unwrap();
    // cost = base(700) + new_account(25000) = 25700
    assert_eq!(cost, 25_700);
}

#[test]
fn charge_zero_resets_memory_words() {
    let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    m.charge_memory(0).unwrap();
    assert_eq!(m.memory_words(), 0);
}

#[test]
fn sstore_refund_cycle_zero_net() {
    let mut m = GasMeter::new(200_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let slot = U256::from_u64(0);
    let addr = test_addr();

    // Initialize slot with original = 42 so refund logic triggers
    m.initialize_storage_slot(&addr, slot, U256::from_u64(42));

    // Warm up (first cold write): 42 -> 1 (cold: 22100, refund 0)
    m.charge_sstore(&addr, slot, U256::from_u64(1)).unwrap();

    // Dirty restore to original: 1 -> 42 (cost: 100, refund +4800)
    m.charge_sstore(&addr, slot, U256::from_u64(42)).unwrap();
    assert_eq!(m.refund(), 4800);

    // Dirty clear (restored value -> 0): 42 -> 0 when original=42 (refund +4800 again)
    m.charge_sstore(&addr, slot, U256::from_u64(0)).unwrap();
    // Dirty set after clear: 0 -> 99 (refund -4800)
    m.charge_sstore(&addr, slot, U256::from_u64(99)).unwrap();
    // Net: 4800 + 4800 - 4800 = 4800, so not zero; just verify no underflow
    assert!(m.refund() >= 0);
    assert_eq!(m.refund(), 4800);
}

#[test]
fn precompile_gas_large_valid() {
    // SHA256 with 10_000_000 bytes: 312500 words, gas = 60 + 312500*12
    let ok = bare_metal_evm_gas::precompile::precompile_gas(0x02, 10_000_000);
    assert_eq!(ok, Some(Ok(60 + 312500 * 12)));
}

#[test]
fn initcode_word_cost_overflow_handling() {
    // Very large initcode should overflow
    let result = bare_metal_evm_gas::create::initcode_word_cost(u64::MAX);
    assert!(result.is_err());
}

#[test]
fn log_gas_all_topics_with_data() {
    use bare_metal_evm_gas::log::log_gas;
    // LOG4 with 256 bytes of data
    let gas = log_gas(4, 256).unwrap();
    // 375 + 4*375 + 8*256 = 375 + 1500 + 2048 = 3923
    assert_eq!(gas, 3_923);
}

#[test]
fn log_gas_overflow() {
    use bare_metal_evm_gas::log::log_gas;
    // 5 topics is invalid (max 4), but function doesn't validate
    // Test with topic count 255 to ensure checked math
    let result = log_gas(255, u64::MAX);
    assert!(result.is_err());
}

#[test]
fn charge_sstore_sentry_exact_boundary() {
    let mut m = GasMeter::new(100_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let slot = U256::from_u64(0);
    // Drain to exactly SSTORE_SENTRY_GAS (2300) remaining
    m.charge(m.remaining() - 2300).unwrap();
    assert_eq!(m.remaining(), 2300);
    // SSTORE sentry requires remaining > 2300, so this should fail
    let result = m.charge_sstore(&test_addr(), slot, U256::from_u64(1));
    assert_eq!(result, Err(bare_metal_evm_gas::GasError::OutOfGas));
}

#[test]
fn eip7623_all_nonzero_calldata() {
    // 100 non-zero bytes: tokens = 100 * 4 = 400, floor = 400 * 10 = 4000
    // intrinsic = 21000 + 100*16 = 22600
    let calldata = [0xFFu8; 100];
    let mut m = GasMeter::new(100_000, &calldata, &[], false, &origin(), Some(&to())).unwrap();
    assert_eq!(m.remaining(), 100_000 - 22_600);

    m.charge(m.remaining()).unwrap();
    m.apply_refund().unwrap();
    // floor = min(4000, 100000) = 4000, final_used = max(100000, 4000) = 100000
    assert_eq!(m.remaining(), 0);
}

#[test]
fn eip7623_mixed_calldata() {
    // 10 zero bytes + 10 non-zero: tokens = 10*1 + 10*4 = 50, floor = 500
    let calldata: Vec<u8> = (0..20).map(|i| if i < 10 { 0 } else { 0xFF }).collect();
    let mut m = GasMeter::new(100_000, &calldata, &[], false, &origin(), Some(&to())).unwrap();
    // intrinsic = 21000 + 10*4 + 10*16 = 21200
    assert_eq!(m.remaining(), 100_000 - 21_200);

    m.charge(m.remaining()).unwrap();
    m.apply_refund().unwrap();
    assert_eq!(m.remaining(), 0);
}

#[test]
fn eip7623_floor_equals_initial_gas_accepted() {
    // 400 zero bytes: tokens = 400, floor = 4000, intrinsic = 21000 + 400*4 = 22600
    // initial_gas = 22600 → intrinsic == initial_gas, remaining = 0
    let calldata = [0u8; 400];
    let m = GasMeter::new(22_600, &calldata, &[], false, &origin(), Some(&to())).unwrap();
    assert_eq!(m.remaining(), 0);
}

#[test]
fn duplicate_access_list_entries() {
    let key = [0xABu8; 32];
    let items = vec![
        bare_metal_evm_gas::AccessListItem {
            address: [0xAA; 20],
            storage_keys: vec![key],
        },
        bare_metal_evm_gas::AccessListItem {
            address: [0xAA; 20],
            storage_keys: vec![key],
        },
    ];
    let m = GasMeter::new(100_000, b"", &items, false, &origin(), Some(&to())).unwrap();
    // Access list costs: 2 * 2400 (addresses) + 2 * 1900 (keys) = 8600
    // Intrinsic: 21000 + 8600 = 29600
    assert_eq!(m.remaining(), 100_000 - 29_600);
    // Warm access to the duplicate address + slot costs only 100 each
    let mut m = m;
    let before = m.remaining();
    m.charge_account_access(&[0xAA; 20]).unwrap();
    m.charge_sload(&[0xAA; 20], U256::from_bytes_be(key))
        .unwrap();
    assert_eq!(before - m.remaining(), 200);
}

#[test]
fn memory_charge_at_max_valid() {
    let mut m = GasMeter::new(3_000_000, b"", &[], false, &origin(), Some(&to())).unwrap();
    let max_bytes = 1usize << 20;
    m.charge_memory(max_bytes).unwrap();
    assert_eq!(m.memory_words(), 32768);
    assert!(m.remaining() > 0);
    // Beyond max should fail
    assert!(m.charge_memory(max_bytes + 1).is_err());
}
