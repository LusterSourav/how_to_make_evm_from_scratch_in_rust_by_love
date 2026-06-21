/// Basic WorldState operations: create, mutate, commit, reload.
///
/// Run with: cargo run --example basic_state
///
/// This example uses `std` for I/O, but the library itself is `#![no_std]`.
extern crate bare_metal_evm;

use bare_metal_evm::{Account, MemoryDB, WorldState, EMPTY_ROOT_HASH, U256};

fn main() -> Result<(), Box<dyn core::fmt::Debug>> {
    // --- Step 1: Create an empty state ---
    let db = MemoryDB::new();
    let mut state = WorldState::new(db);

    assert_eq!(state.state_root(), EMPTY_ROOT_HASH);
    println!("1. Empty state root: {:?}", hex::encode(state.state_root()));

    // --- Step 2: Create and mutate an account ---
    let addr = [0u8; 20]; // address 0x00...00
    let mut acc = Account::new_empty();
    acc.nonce = U256::from_u64(1);
    acc.balance = U256::from_u64(1000);
    state.set_account(addr, acc).unwrap();
    state.add_balance(addr, U256::from_u64(500)).unwrap();
    state.increment_nonce(addr).unwrap();

    let acc = state.get_account(&addr).unwrap().unwrap();
    println!("2. Balance: {}, Nonce: {}", acc.balance, acc.nonce);

    // --- Step 3: Set storage ---
    let slot = U256::from_u64(42);
    state.set_storage(addr, slot, U256::from_u64(99)).unwrap();
    let val = state.get_storage(&addr, &slot).unwrap();
    println!("3. Storage[42] = {val}");

    // --- Step 4: Set contract code ---
    let code = vec![0x60, 0x01, 0x60, 0x02, 0x01]; // PUSH1 1; PUSH1 2; ADD
    let code_hash = state.set_code(addr, code.clone());
    println!("4. Code hash: {:?}", hex::encode(code_hash));
    assert!(state.get_code(&code_hash).is_some());

    let mut acc = state.get_account(&addr).unwrap().unwrap();
    acc.code_hash = code_hash;
    state.set_account(addr, acc).unwrap();

    // --- Step 5: Checkpoint and rollback ---
    let _ = state.checkpoint();
    let old_balance = state.get_account(&addr).unwrap().unwrap().balance;
    state.add_balance(addr, U256::from_u64(9999)).unwrap();
    state.rollback().unwrap();
    let balance_after = state.get_account(&addr).unwrap().unwrap().balance;
    assert_eq!(balance_after, old_balance);
    println!("5. Rollback works: balance unchanged at {old_balance}");

    // --- Step 6: Commit to the trie ---
    let root = state.commit().unwrap();
    println!("6. Committed root: {:?}", hex::encode(root));
    assert_ne!(root, EMPTY_ROOT_HASH);

    // --- Step 7: Reload from root ---
    let db = state.into_db();
    let state2 = WorldState::from_root(db, &root).unwrap();
    let acc2 = state2.get_account(&addr).unwrap().unwrap();
    println!(
        "7. Reloaded: balance={}, nonce={}",
        acc2.balance, acc2.nonce
    );
    assert_eq!(acc2.balance, U256::from_u64(1500));
    assert_eq!(acc2.nonce, U256::from_u64(2));
    assert_eq!(
        state2.get_storage(&addr, &slot).unwrap(),
        U256::from_u64(99)
    );
    assert!(state2.get_code(&code_hash).is_some());

    println!("8. All checks passed!");
    Ok(())
}
