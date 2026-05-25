use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::account::Account;
use crate::db::Database;
use crate::journal::{Journal, JournalEntry};
use crate::keccak::keccak256;
use crate::trie::{self, delete_trie_nodes, Trie, EMPTY_ROOT_HASH};
use crate::U256;

// ============================================================
// Error
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Trie(trie::Error),
    Decode,
}

impl From<trie::Error> for Error {
    fn from(e: trie::Error) -> Self {
        Self::Trie(e)
    }
}

// ============================================================
// WorldState
// ============================================================

/// In-memory world state with deferred trie writes and journal-based rollback.
///
/// Reads hit the trie. Writes go to caches. On [`commit`](Self::commit),
/// all caches are flushed to the trie and persisted to the database.
#[derive(Clone)]
pub struct WorldState<D: Database> {
    db: D,
    state_trie: Trie,
    /// Pending account changes. `None` means the account was deleted.
    account_cache: BTreeMap<[u8; 20], Option<Account>>,
    /// Pending storage changes.
    storage_cache: BTreeMap<([u8; 20], U256), U256>,
    /// Contract code, keyed by code hash.
    code_cache: BTreeMap<[u8; 32], Vec<u8>>,
    /// Change journal for rollback.
    journal: Journal,
}

impl<D: Database> WorldState<D> {
    /// Create a new empty world state.
    #[must_use]
    pub fn new(db: D) -> Self {
        Self {
            db,
            state_trie: Trie::new(),
            account_cache: BTreeMap::new(),
            storage_cache: BTreeMap::new(),
            code_cache: BTreeMap::new(),
            journal: Journal::new(),
        }
    }

    /// Load world state from a persisted state root.
    pub fn from_root(db: D, root: &[u8; 32]) -> Result<Self, Error> {
        let state_trie = Trie::from_root(&db, root)?;
        Ok(Self {
            db,
            state_trie,
            account_cache: BTreeMap::new(),
            storage_cache: BTreeMap::new(),
            code_cache: BTreeMap::new(),
            journal: Journal::new(),
        })
    }

    /// Read an account from cache or state trie.
    pub fn get_account(&self, address: &[u8; 20]) -> Result<Option<Account>, Error> {
        if let Some(opt) = self.account_cache.get(address) {
            return Ok(opt.clone());
        }
        let key = keccak256(address);
        let data = self.state_trie.get(&self.db, &key)?;
        match data {
            Some(bytes) => Ok(Some(Account::decode(&bytes).map_err(|_| Error::Decode)?)),
            None => Ok(None),
        }
    }

    /// Insert or update an account.
    pub fn set_account(&mut self, address: [u8; 20], account: Account) -> Result<(), Error> {
        let old = self.get_account(&address)?;
        self.journal
            .push(JournalEntry::AccountChange { address, old });
        self.account_cache.insert(address, Some(account));
        Ok(())
    }

    /// Remove an account from the state trie.
    pub fn remove_account(&mut self, address: &[u8; 20]) -> Result<(), Error> {
        let old = self.get_account(address)?;
        if old.is_none() {
            return Ok(());
        }
        self.journal.push(JournalEntry::AccountChange {
            address: *address,
            old,
        });
        self.account_cache.insert(*address, None);
        self.storage_cache.retain(|(a, _), _| a != address);
        Ok(())
    }

    /// Read a storage value.
    pub fn get_storage(&self, address: &[u8; 20], slot: &U256) -> Result<U256, Error> {
        if let Some(val) = self.storage_cache.get(&(*address, *slot)) {
            return Ok(*val);
        }

        let storage_root = match self.get_account(address)? {
            Some(acc) if acc.storage_root != EMPTY_ROOT_HASH => acc.storage_root,
            _ => return Ok(U256::zero()),
        };

        let storage_trie = Trie::from_root(&self.db, &storage_root)?;
        let slot_hash = keccak256(&slot.to_bytes_be());
        let raw = match storage_trie.get(&self.db, &slot_hash)? {
            Some(data) => data,
            None => return Ok(U256::zero()),
        };
        let mut padded = [0u8; 32];
        padded[32 - raw.len()..].copy_from_slice(&raw);
        Ok(U256::from_bytes_be(padded))
    }

    /// Write a storage value. Zero values are stored (removed on commit).
    /// Auto-creates the account if it doesn't exist (matching Geth behavior).
    pub fn set_storage(&mut self, address: [u8; 20], slot: U256, value: U256) -> Result<(), Error> {
        // Read the old value before auto-creating the account, so that we
        // capture the committed value even when the account was deleted from
        // the cache via a prior remove_account (see read_current_storage).
        let old = self.read_current_storage(&address, &slot)?;
        if old == value {
            return Ok(());
        }
        if self.get_account(&address)?.is_none() {
            self.set_account(address, Account::new_empty())?;
        }
        self.journal
            .push(JournalEntry::StorageChange { address, slot, old });
        self.storage_cache.insert((address, slot), value);
        Ok(())
    }

    /// Read a storage value bypassing the account-cache `None` entry.
    /// Checks the storage cache first, then falls back to the committed trie.
    /// Needed by [`set_storage`] so that it can capture the current storage
    /// value before a deleted account is re-created in the cache.
    fn read_current_storage(&self, address: &[u8; 20], slot: &U256) -> Result<U256, Error> {
        // Check storage cache first (fast path)
        if let Some(val) = self.storage_cache.get(&(*address, *slot)) {
            return Ok(*val);
        }
        // Check the account trie directly, ignoring the cache's deleted flag
        let account = {
            let key = keccak256(address);
            match self.state_trie.get(&self.db, &key)? {
                Some(bytes) => {
                    Account::decode(&bytes).map_err(|_| Error::Decode)?
                }
                None => return Ok(U256::zero()),
            }
        };
        if account.storage_root == EMPTY_ROOT_HASH {
            return Ok(U256::zero());
        }
        let storage_trie = Trie::from_root(&self.db, &account.storage_root)?;
        let slot_hash = keccak256(&slot.to_bytes_be());
        let raw = match storage_trie.get(&self.db, &slot_hash)? {
            Some(data) => data,
            None => return Ok(U256::zero()),
        };
        let mut padded = [0u8; 32];
        padded[32 - raw.len()..].copy_from_slice(&raw);
        Ok(U256::from_bytes_be(padded))
    }

    /// Store contract code.
    pub fn set_code(&mut self, code_hash: [u8; 32], code: Vec<u8>) {
        self.code_cache.insert(code_hash, code);
    }

    /// Retrieve contract code by hash.
    #[must_use]
    pub fn get_code(&self, code_hash: &[u8; 32]) -> Option<&[u8]> {
        self.code_cache.get(code_hash).map(Vec::as_slice)
    }

    /// Consume the state and return the underlying database.
    #[must_use]
    pub fn into_db(self) -> D {
        self.db
    }

    /// Save a checkpoint for rollback.
    pub fn checkpoint(&mut self) {
        self.journal.checkpoint();
    }

    /// Roll back to the most recent checkpoint.
    pub fn rollback(&mut self) -> Result<(), Error> {
        let target = match self.journal.checkpoints.last() {
            Some(&t) => t,
            None => return Ok(()),
        };

        while self.journal.len() > target {
            let entry = self.journal.pop().unwrap();
            match entry {
                JournalEntry::AccountChange { address, old } => match old {
                    Some(acc) => {
                        self.account_cache.insert(address, Some(acc));
                    }
                    None => {
                        self.account_cache.remove(&address);
                    }
                },
                JournalEntry::StorageChange { address, slot, old } => {
                    self.storage_cache.insert((address, slot), old);
                }
            }
        }
        self.journal.checkpoints.pop();
        Ok(())
    }

    /// Flush all pending changes to the trie and compute the state root.
    pub fn commit(&mut self) -> Result<[u8; 32], Error> {
        // Group storage changes by account
        let mut account_storage: BTreeMap<[u8; 20], Vec<U256>> = BTreeMap::new();
        for (addr, slot) in self.storage_cache.keys() {
            account_storage.entry(*addr).or_default().push(*slot);
        }

        // Flush storage changes
        for (addr, slots) in account_storage.iter_mut() {
            slots.sort();
            slots.dedup();

            let storage_root = {
                let account = self.get_account(addr)?;
                match account {
                    Some(acc) if acc.storage_root != EMPTY_ROOT_HASH => acc.storage_root,
                    _ => EMPTY_ROOT_HASH,
                }
            };

            let mut storage_trie = if storage_root == EMPTY_ROOT_HASH {
                Trie::new()
            } else {
                Trie::from_root(&self.db, &storage_root)?
            };

            for slot in slots {
                let val = self.storage_cache[&(*addr, *slot)];
                let slot_hash = keccak256(&slot.to_bytes_be());
                if val.is_zero() {
                    storage_trie.remove(&mut self.db, &slot_hash)?;
                } else {
                    let raw = trim_leading_zeros_be(val);
                    storage_trie.insert(&mut self.db, &slot_hash, raw)?;
                }
            }

            let new_storage_root = storage_trie.root_hash(&mut self.db)?;

            // Update account in cache (written to trie in the account loop below)
            if let Some(mut acc) = self.get_account(addr)? {
                acc.storage_root = new_storage_root;
                self.account_cache.insert(*addr, Some(acc));
            }
        }

        // Prune old storage tries for deleted accounts
        for (addr, opt_acc) in &self.account_cache {
            if opt_acc.is_none() {
                let key = keccak256(addr);
                if let Some(data) = self.state_trie.get(&self.db, &key)? {
                    if let Ok(acc) = crate::account::Account::decode(&data) {
                        if acc.storage_root != EMPTY_ROOT_HASH {
                            delete_trie_nodes(&mut self.db, &acc.storage_root)?;
                        }
                    }
                }
            }
        }

        // Flush account deletions/insertions
        for (addr, opt_acc) in self.account_cache.iter() {
            let key = keccak256(addr);
            match opt_acc {
                Some(acc) => {
                    let encoded = acc.encode();
                    self.state_trie.insert(&mut self.db, &key, encoded)?;
                }
                None => {
                    self.state_trie.remove(&mut self.db, &key)?;
                }
            }
        }

        let root = self.state_trie.root_hash(&mut self.db)?;

        self.account_cache.clear();
        self.storage_cache.clear();
        self.journal = Journal::new();

        Ok(root)
    }
}

// ============================================================
// Helpers
// ============================================================

/// Strip leading zero bytes from a U256 big-endian representation.
fn trim_leading_zeros_be(val: U256) -> Vec<u8> {
    let be = val.to_bytes_be();
    let start = be.iter().position(|&b| b != 0).unwrap_or(be.len());
    be[start..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MemoryDB;
    use alloc::vec;

    fn make_db() -> MemoryDB {
        MemoryDB::new()
    }

    fn addr(n: u8) -> [u8; 20] {
        let mut a = [0u8; 20];
        a[19] = n;
        a
    }

    #[test]
    fn state_new_empty() {
        let state = WorldState::new(make_db());
        assert_eq!(state.get_account(&addr(1)).unwrap(), None);
    }

    #[test]
    fn state_set_get_account() {
        let mut state = WorldState::new(make_db());
        let acc = Account::new_empty();
        state.set_account(addr(1), acc.clone()).unwrap();
        assert_eq!(state.get_account(&addr(1)).unwrap(), Some(acc));
    }

    #[test]
    fn state_remove_account() {
        let mut state = WorldState::new(make_db());
        state.set_account(addr(1), Account::new_empty()).unwrap();
        assert!(state.get_account(&addr(1)).unwrap().is_some());
        state.remove_account(&addr(1)).unwrap();
        assert_eq!(state.get_account(&addr(1)).unwrap(), None);
    }

    #[test]
    fn state_storage_set_get() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(42);
        let val = U256::from_u64(99);
        state.set_storage(addr(1), slot, val).unwrap();
        assert_eq!(state.get_storage(&addr(1), &slot).unwrap(), val);
    }

    #[test]
    fn state_storage_zero_default() {
        let state = WorldState::new(make_db());
        let slot = U256::from_u64(42);
        assert_eq!(state.get_storage(&addr(1), &slot).unwrap(), U256::zero());
    }

    #[test]
    fn state_storage_noop() {
        let mut state = WorldState::new(make_db());
        state.set_account(addr(1), Account::new_empty()).unwrap();
        state.commit().unwrap();
        // Setting zero on an empty slot after the account exists should be a no-op
        let slot = U256::from_u64(42);
        state.set_storage(addr(1), slot, U256::zero()).unwrap();
        assert!(state.journal.is_empty());
    }

    #[test]
    fn state_checkpoint_rollback_account() {
        let mut state = WorldState::new(make_db());
        state.checkpoint();
        state.set_account(addr(1), Account::new_empty()).unwrap();
        assert!(state.get_account(&addr(1)).unwrap().is_some());
        state.rollback().unwrap();
        assert_eq!(state.get_account(&addr(1)).unwrap(), None);
    }

    #[test]
    fn state_checkpoint_rollback_storage() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(42);
        state.set_account(addr(1), Account::new_empty()).unwrap();
        state.checkpoint();
        state
            .set_storage(addr(1), slot, U256::from_u64(99))
            .unwrap();
        assert_eq!(
            state.get_storage(&addr(1), &slot).unwrap(),
            U256::from_u64(99)
        );
        state.rollback().unwrap();
        assert_eq!(state.get_storage(&addr(1), &slot).unwrap(), U256::zero());
    }

    #[test]
    fn state_commit_and_reload() {
        let mut state = WorldState::new(make_db());
        let a1 = addr(1);

        state.set_account(a1, Account::new_empty()).unwrap();
        state
            .set_storage(a1, U256::from_u64(1), U256::from_u64(100))
            .unwrap();

        let root = state.commit().unwrap();
        assert_ne!(root, EMPTY_ROOT_HASH);

        // Verify accounts are accessible after commit
        let acc = state.get_account(&a1).unwrap().unwrap();
        assert_eq!(acc.nonce, U256::zero());
        assert_eq!(acc.balance, U256::zero());
    }

    #[test]
    fn state_code_cache() {
        let mut state = WorldState::new(make_db());
        let code = vec![0x60, 0x01, 0x60, 0x02];
        let hash = keccak256(&code);
        state.set_code(hash, code.clone());
        assert_eq!(state.get_code(&hash), Some(code.as_slice()));
    }

    #[test]
    fn trim_zeros() {
        let val = U256::from_u64(0x0100);
        let trimmed = trim_leading_zeros_be(val);
        assert_eq!(trimmed, &[0x01, 0x00]); // preserves trailing zero

        let zero = U256::zero();
        assert!(trim_leading_zeros_be(zero).is_empty());
    }

    #[test]
    fn state_two_phase_commit() {
        let mut state = WorldState::new(make_db());
        state.set_account(addr(1), Account::new_empty()).unwrap();
        let root1 = state.commit().unwrap();
        assert_ne!(root1, EMPTY_ROOT_HASH);

        let mut acc = Account::new_empty();
        acc.nonce = U256::from_u64(1);
        state.set_account(addr(1), acc.clone()).unwrap();
        let root2 = state.commit().unwrap();
        assert_ne!(root2, root1);

        let got = state.get_account(&addr(1)).unwrap().unwrap();
        assert_eq!(got.nonce, U256::from_u64(1));
    }

    #[test]
    fn state_storage_auto_creates_account() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(1);
        state
            .set_storage(addr(1), slot, U256::from_u64(42))
            .unwrap();
        let acc = state.get_account(&addr(1)).unwrap().unwrap();
        assert_eq!(acc.nonce, U256::zero());
        assert_eq!(
            state.get_storage(&addr(1), &slot).unwrap(),
            U256::from_u64(42)
        );
        let root = state.commit().unwrap();
        assert_ne!(root, EMPTY_ROOT_HASH);
        let acc2 = state.get_account(&addr(1)).unwrap().unwrap();
        assert_eq!(acc2.nonce, U256::zero());
        assert_eq!(
            state.get_storage(&addr(1), &slot).unwrap(),
            U256::from_u64(42)
        );
    }

    #[test]
    fn state_account_delete_before_storage() {
        let mut state = WorldState::new(make_db());
        state.set_account(addr(1), Account::new_empty()).unwrap();
        state
            .set_storage(addr(1), U256::from_u64(1), U256::from_u64(100))
            .unwrap();
        state.remove_account(&addr(1)).unwrap();
        let _root = state.commit().unwrap();
        assert_eq!(state.get_account(&addr(1)).unwrap(), None);
    }

    #[test]
    fn state_storage_zero_removes_on_commit() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(1);

        state
            .set_storage(addr(1), slot, U256::from_u64(42))
            .unwrap();
        state.set_storage(addr(1), slot, U256::zero()).unwrap();
        assert_eq!(state.get_storage(&addr(1), &slot).unwrap(), U256::zero());

        let _root = state.commit().unwrap();
        // After commit, zero-valued storage is removed and defaults to zero
        assert_eq!(state.get_storage(&addr(1), &slot).unwrap(), U256::zero());
    }

    #[test]
    fn state_set_storage_then_remove_account_rollback_restores_storage() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(1);

        state
            .set_storage(addr(1), slot, U256::from_u64(42))
            .unwrap();
        state.commit().unwrap();

        state.checkpoint();
        state
            .set_storage(addr(1), slot, U256::from_u64(99))
            .unwrap();
        state.remove_account(&addr(1)).unwrap();
        state.rollback().unwrap();

        let acc = state.get_account(&addr(1)).unwrap().unwrap();
        assert_eq!(acc.nonce, U256::zero());
        assert_eq!(
            state.get_storage(&addr(1), &slot).unwrap(),
            U256::from_u64(42)
        );
    }

    /// Regression: remove_account then set_storage then rollback should restore
    /// the committed storage value, not lose it.
    #[test]
    fn state_remove_account_then_set_storage_rollback_preserves_storage() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(1);

        // Commit account with storage
        state.set_storage(addr(1), slot, U256::from_u64(42)).unwrap();
        state.commit().unwrap();
        assert_eq!(state.get_storage(&addr(1), &slot).unwrap(), U256::from_u64(42));

        state.checkpoint();

        // Remove the account
        state.remove_account(&addr(1)).unwrap();
        assert_eq!(state.get_account(&addr(1)).unwrap(), None);

        // Set storage again (auto-creates account)
        state.set_storage(addr(1), slot, U256::from_u64(99)).unwrap();

        // Rollback — should restore committed account + storage
        state.rollback().unwrap();

        let acc = state.get_account(&addr(1)).unwrap().unwrap();
        assert_eq!(acc.nonce, U256::zero());
        assert_eq!(
            state.get_storage(&addr(1), &slot).unwrap(),
            U256::from_u64(42)
        );
    }
}
