use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

use bare_metal_evm_keccak::keccak256;
use bare_metal_evm_trie as trie;
use bare_metal_evm_types::{U256, U512};

use crate::account::Account;
use crate::journal::{Journal, JournalEntry};

use trie::{delete_trie_nodes, Database, Trie, EMPTY_ROOT_HASH};

// Error

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Trie(trie::Error),
    Decode,
    Journal,
    Arithmetic,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trie(e) => write!(f, "state trie: {e}"),
            Self::Decode => write!(f, "state decode error"),
            Self::Journal => write!(f, "journal error"),
            Self::Arithmetic => write!(f, "arithmetic overflow"),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::Trie(e) => Some(e),
            _ => None,
        }
    }
}

impl From<trie::Error> for Error {
    fn from(e: trie::Error) -> Self {
        Self::Trie(e)
    }
}

// WorldState

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
    /// The most recently committed state root.
    cached_root: Option<[u8; 32]>,
}

impl<D: fmt::Debug + Database> fmt::Debug for WorldState<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorldState")
            .field("db", &self.db)
            .field("account_cache", &self.account_cache)
            .field("storage_cache", &self.storage_cache)
            .field("cached_root", &self.cached_root)
            .finish_non_exhaustive()
    }
}

impl<D: Database> WorldState<D> {
    /// Create a new empty world state.
    #[must_use]
    pub const fn new(db: D) -> Self {
        Self {
            db,
            state_trie: Trie::new(),
            account_cache: BTreeMap::new(),
            storage_cache: BTreeMap::new(),
            code_cache: BTreeMap::new(),
            journal: Journal::new(),
            cached_root: Some(EMPTY_ROOT_HASH),
        }
    }

    /// Load world state from a persisted state root.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the trie nodes under `root` cannot be
    /// decoded (e.g. corrupt database or invalid root hash).
    pub fn from_root(db: D, root: &[u8; 32]) -> Result<Self, Error> {
        let state_trie = Trie::from_root(&db, root)?;
        Ok(Self {
            db,
            state_trie,
            account_cache: BTreeMap::new(),
            storage_cache: BTreeMap::new(),
            code_cache: BTreeMap::new(),
            journal: Journal::new(),
            cached_root: Some(*root),
        })
    }

    /// Read an account from cache or state trie.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the RLP-encoded account data in the trie
    /// is malformed, or if a trie node cannot be decoded.
    pub fn get_account(&self, address: &[u8; 20]) -> Result<Option<Account>, Error> {
        if let Some(opt) = self.account_cache.get(address) {
            return Ok(opt.clone());
        }
        let key = keccak256(address);
        let data = self.state_trie.get(&self.db, &key)?;
        match data {
            Some(bytes) => Ok(Some(Account::decode(&bytes).map_err(|()| Error::Decode)?)),
            None => Ok(None),
        }
    }

    /// Convenience: get the balance of an account. Returns zero for non-existent accounts.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the underlying account data is malformed.
    pub fn get_balance(&self, address: &[u8; 20]) -> Result<U256, Error> {
        Ok(self
            .get_account(address)?
            .map_or(U256::zero(), |a| a.balance))
    }

    /// Convenience: get the nonce of an account. Returns zero for non-existent accounts.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the underlying account data is malformed.
    pub fn get_nonce(&self, address: &[u8; 20]) -> Result<U256, Error> {
        Ok(self.get_account(address)?.map_or(U256::zero(), |a| a.nonce))
    }

    /// Convenience: get the code hash of an account. Returns [`crate::account::EMPTY_CODE_HASH`]
    /// for non-existent accounts.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the underlying account data is malformed.
    pub fn get_code_hash(&self, address: &[u8; 20]) -> Result<[u8; 32], Error> {
        Ok(self
            .get_account(address)?
            .map_or(crate::account::EMPTY_CODE_HASH, |a| a.code_hash))
    }

    /// Convenience: get the storage root of an account. Returns [`EMPTY_ROOT_HASH`]
    /// for non-existent accounts.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the underlying account data is malformed.
    pub fn get_storage_root(&self, address: &[u8; 20]) -> Result<[u8; 32], Error> {
        Ok(self
            .get_account(address)?
            .map_or(EMPTY_ROOT_HASH, |a| a.storage_root))
    }

    /// Insert or update an account.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the existing account (being overwritten)
    /// cannot be read from the trie due to malformed RLP data.
    pub fn set_account(&mut self, address: [u8; 20], account: Account) -> Result<(), Error> {
        let old = self.get_account(&address)?;
        self.journal
            .push(JournalEntry::AccountChange { address, old });
        self.account_cache.insert(address, Some(account));
        Ok(())
    }

    /// Remove an account from the state trie.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the existing account cannot be read from
    /// the trie.
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
        // Do NOT clear storage_cache entries — they are journaled and must survive
        // rollback. The get_storage guard below hides them while the account is deleted.
        Ok(())
    }

    /// Read a storage value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the account or storage trie data is
    /// malformed.
    pub fn get_storage(&self, address: &[u8; 20], slot: &U256) -> Result<U256, Error> {
        // If the account is deleted in the cache, storage is gone
        if self.account_cache.get(address) == Some(&None) {
            return Ok(U256::zero());
        }
        if let Some(val) = self.storage_cache.get(&(*address, *slot)) {
            return Ok(*val);
        }

        let storage_root = match self.get_account(address)? {
            Some(acc) if acc.storage_root != EMPTY_ROOT_HASH => acc.storage_root,
            _ => return Ok(U256::zero()),
        };

        let storage_trie = Trie::from_root(&self.db, &storage_root)?;
        let slot_hash = keccak256(&slot.to_bytes_be());
        let Some(raw) = storage_trie.get(&self.db, &slot_hash)? else {
            return Ok(U256::zero());
        };
        if raw.len() > 32 {
            return Err(Error::Decode);
        }
        let mut padded = [0u8; 32];
        padded[32 - raw.len()..].copy_from_slice(&raw);
        Ok(U256::from_bytes_be(padded))
    }

    /// Write a storage value. Zero values are stored (removed on commit).
    /// Auto-creates the account if it doesn't exist (matching Geth behavior).
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the storage trie cannot be read.
    pub fn set_storage(&mut self, address: [u8; 20], slot: U256, value: U256) -> Result<(), Error> {
        // Read the old value before auto-creating the account, so that we
        // capture the committed value even when the account was deleted from
        // the cache via a prior remove_account (see read_current_storage).
        let old = self.read_current_storage(&address, &slot)?;
        if old == value {
            return Ok(());
        }
        if self.get_account(&address)?.is_none() {
            // Account was deleted from cache; read the trie value for correct journaling
            let trie_old = self.read_current_account(&address)?;
            self.journal.push(JournalEntry::AccountChange {
                address,
                old: trie_old,
            });
            self.account_cache
                .insert(address, Some(Account::new_empty()));
        }
        self.journal
            .push(JournalEntry::StorageChange { address, slot, old });
        self.storage_cache.insert((address, slot), value);
        Ok(())
    }

    /// Read an account from the trie directly, bypassing the cache.
    fn read_current_account(&self, address: &[u8; 20]) -> Result<Option<Account>, Error> {
        let key = keccak256(address);
        match self.state_trie.get(&self.db, &key)? {
            Some(bytes) => Ok(Some(Account::decode(&bytes).map_err(|()| Error::Decode)?)),
            None => Ok(None),
        }
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
                Some(bytes) => Account::decode(&bytes).map_err(|()| Error::Decode)?,
                None => return Ok(U256::zero()),
            }
        };
        if account.storage_root == EMPTY_ROOT_HASH {
            return Ok(U256::zero());
        }
        let storage_trie = Trie::from_root(&self.db, &account.storage_root)?;
        let slot_hash = keccak256(&slot.to_bytes_be());
        let Some(raw) = storage_trie.get(&self.db, &slot_hash)? else {
            return Ok(U256::zero());
        };
        if raw.len() > 32 {
            return Err(Error::Decode);
        }
        let mut padded = [0u8; 32];
        padded[32 - raw.len()..].copy_from_slice(&raw);
        Ok(U256::from_bytes_be(padded))
    }

    /// Store contract code, computing the code hash automatically.
    ///
    /// This is the safe version — it computes `keccak256(&code)` internally,
    /// guaranteeing that the hash matches the code. Returns the computed hash.
    #[must_use]
    pub fn set_code(&mut self, address: [u8; 20], code: Vec<u8>) -> [u8; 32] {
        let code_hash = keccak256(&code);
        let old_present = self.code_cache.contains_key(&code_hash);
        self.journal.push(JournalEntry::CodeChange {
            address,
            hash: code_hash,
            old_present,
        });
        self.code_cache.insert(code_hash, code);
        code_hash
    }

    /// Store contract code with an externally provided code hash.
    ///
    /// # Deprecated
    /// Use [`set_code`](Self::set_code) instead, which computes the hash
    /// internally and guarantees correctness.
    #[deprecated(
        since = "0.2.0",
        note = "use set_code(address, code) which computes keccak256 internally"
    )]
    pub fn set_code_with_hash(&mut self, address: [u8; 20], code_hash: [u8; 32], code: Vec<u8>) {
        let old_present = self.code_cache.contains_key(&code_hash);
        self.journal.push(JournalEntry::CodeChange {
            address,
            hash: code_hash,
            old_present,
        });
        self.code_cache.insert(code_hash, code);
    }

    /// Retrieve contract code by hash.
    ///
    /// Checks the code cache first, then falls back to the database.
    /// This ensures code is accessible even after [`commit`](Self::commit)
    /// clears the cache or after loading from a persisted root via
    /// [`from_root`](Self::from_root).
    #[allow(clippy::result_unit_err)]
    pub fn get_code(&self, code_hash: &[u8; 32]) -> Option<Vec<u8>> {
        if let Some(code) = self.code_cache.get(code_hash) {
            return Some(code.clone());
        }
        self.db.get(code_hash).ok()?
    }

    /// Retrieve a reference to contract code by hash, without cloning.
    ///
    /// Only checks the in-memory code cache — does not fall back to the
    /// database. This is useful when the caller already has `code_hash`
    /// from a [`commit`](Self::commit) (which persists code to the DB)
    /// and just needs read-only access without allocation.
    #[must_use]
    pub fn get_code_ref(&self, code_hash: &[u8; 32]) -> Option<&[u8]> {
        self.code_cache.get(code_hash).map(|v| v.as_slice())
    }

    /// Consume the state and return the underlying database.
    #[must_use]
    pub fn into_db(self) -> D {
        self.db
    }

    /// Return the most recently committed state root.
    #[must_use]
    pub fn state_root(&self) -> [u8; 32] {
        self.cached_root.unwrap_or(EMPTY_ROOT_HASH)
    }

    /// Check whether an account exists in the state.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the trie data is malformed.
    pub fn account_exists(&self, address: &[u8; 20]) -> Result<bool, Error> {
        self.get_account(address).map(|opt| opt.is_some())
    }

    /// Add balance to an account, creating it if necessary.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the trie is corrupt, or
    /// [`Error::Arithmetic`] on overflow.
    pub fn add_balance(&mut self, address: [u8; 20], amount: U256) -> Result<(), Error> {
        let mut acc = self
            .get_account(&address)?
            .unwrap_or_else(Account::new_empty);
        acc.balance = acc.balance.checked_add(amount).ok_or(Error::Arithmetic)?;
        self.set_account(address, acc)
    }

    /// Subtract balance from an account, creating it if necessary.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the trie is corrupt, or
    /// [`Error::Arithmetic`] on underflow.
    pub fn sub_balance(&mut self, address: [u8; 20], amount: U256) -> Result<(), Error> {
        let mut acc = self
            .get_account(&address)?
            .unwrap_or_else(Account::new_empty);
        acc.balance = acc.balance.checked_sub(amount).ok_or(Error::Arithmetic)?;
        self.set_account(address, acc)
    }

    /// Increment the nonce of an account, creating it if necessary.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the trie is corrupt, or
    /// [`Error::Arithmetic`] on nonce overflow.
    pub fn increment_nonce(&mut self, address: [u8; 20]) -> Result<(), Error> {
        let mut acc = self
            .get_account(&address)?
            .unwrap_or_else(Account::new_empty);
        acc.nonce = acc
            .nonce
            .checked_add(U256::one())
            .ok_or(Error::Arithmetic)?;
        self.set_account(address, acc)
    }

    /// Compute the transaction fee as `gas_used * gas_price`, returning a
    /// 512-bit result so callers can detect overflow before reducing to
    /// the account's 256-bit balance.
    ///
    /// This is the canonical EVM fee calculation. The full product is
    /// returned as a [`U512`] because `gas_used * gas_price` can exceed
    /// the 256-bit range; callers are expected to [`U512::low_u256`] for the
    /// truncated value and [`U512::high_is_zero`] to detect overflow.
    #[must_use]
    pub fn compute_tx_fee(gas_used: U256, gas_price: U256) -> U512 {
        gas_used.mul_full(gas_price)
    }

    /// Save a checkpoint for rollback.
    ///
    /// Returns `false` if the maximum checkpoint depth has been reached.
    #[must_use]
    pub fn checkpoint(&mut self) -> bool {
        self.journal.checkpoint()
    }

    /// Roll back to the most recent checkpoint.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Journal`] if the journal is in an inconsistent state
    /// (should never happen in normal usage).
    pub fn rollback(&mut self) -> Result<(), Error> {
        let Some(&target) = self.journal.checkpoints.last() else {
            return Ok(());
        };

        while self.journal.len() > target {
            let entry = self.journal.pop().ok_or(Error::Journal)?;
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
                JournalEntry::CodeChange {
                    hash, old_present, ..
                } => {
                    if !old_present {
                        self.code_cache.remove(&hash);
                    }
                }
            }
        }
        self.journal.checkpoints.pop();
        Ok(())
    }

    /// Flush all pending changes to the trie and compute the state root.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the trie nodes cannot be decoded
    /// (e.g. corrupt database).
    pub fn commit(&mut self) -> Result<[u8; 32], Error> {
        self.commit_storage()?;
        self.commit_prune_deleted_storage()?;
        self.commit_accounts()?;
        self.commit_code()?;
        let root = self.state_trie.root_hash(&mut self.db)?;
        self.commit_clear_caches(root);
        Ok(root)
    }

    /// Flush storage changes for each account.
    fn commit_storage(&mut self) -> Result<(), Error> {
        let mut account_storage: BTreeMap<[u8; 20], Vec<U256>> = BTreeMap::new();
        for (addr, slot) in self.storage_cache.keys() {
            account_storage.entry(*addr).or_default().push(*slot);
        }

        for (addr, slots) in &mut account_storage {
            // Skip accounts being deleted or absent from cache
            if self.account_cache.get(addr).is_none_or(Option::is_none) {
                continue;
            }
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
                let val = self
                    .storage_cache
                    .get(&(*addr, *slot))
                    .copied()
                    .unwrap_or_default();
                let slot_hash = keccak256(&slot.to_bytes_be());
                if val.is_zero() {
                    storage_trie.remove(&mut self.db, &slot_hash)?;
                } else {
                    let raw = trim_leading_zeros_be(val);
                    storage_trie.insert(&mut self.db, &slot_hash, raw)?;
                }
            }

            let new_storage_root = storage_trie.root_hash(&mut self.db)?;

            if let Some(mut acc) = self.get_account(addr)? {
                acc.storage_root = new_storage_root;
                self.account_cache.insert(*addr, Some(acc));
            }
        }
        Ok(())
    }

    /// Prune old storage tries for deleted accounts.
    fn commit_prune_deleted_storage(&mut self) -> Result<(), Error> {
        let mut pruned_roots = alloc::collections::BTreeSet::new();
        for (addr, opt_acc) in &self.account_cache {
            if opt_acc.is_none() {
                let key = keccak256(addr);
                if let Some(data) = self.state_trie.get(&self.db, &key)? {
                    if let Ok(acc) = crate::account::Account::decode(&data) {
                        if acc.storage_root != EMPTY_ROOT_HASH
                            && pruned_roots.insert(acc.storage_root)
                        {
                            delete_trie_nodes(&mut self.db, &acc.storage_root)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Flush account deletions/insertions with EIP-158 empty-account pruning.
    fn commit_accounts(&mut self) -> Result<(), Error> {
        let empty_code_hash = crate::account::EMPTY_CODE_HASH;
        for (addr, opt_acc) in &self.account_cache {
            let key = keccak256(addr);
            if opt_acc.as_ref().is_none_or(|acc| {
                acc.nonce.is_zero()
                    && acc.balance.is_zero()
                    && acc.storage_root == EMPTY_ROOT_HASH
                    && acc.code_hash == empty_code_hash
            }) {
                self.state_trie.remove(&mut self.db, &key)?;
            } else if let Some(acc) = opt_acc {
                let encoded = acc.encode();
                self.state_trie.insert(&mut self.db, &key, encoded)?;
            }
        }
        Ok(())
    }

    /// Persist contract code cache to the database.
    fn commit_code(&mut self) -> Result<(), Error> {
        for (hash, code) in &self.code_cache {
            self.db
                .insert(*hash, code.clone())
                .map_err(|()| Error::Trie(trie::Error::Database))?;
        }
        Ok(())
    }

    /// Clear all caches and set the new state root.
    fn commit_clear_caches(&mut self, root: [u8; 32]) {
        self.account_cache.clear();
        self.storage_cache.clear();
        self.code_cache.clear();
        self.journal = Journal::new();
        self.cached_root = Some(root);
    }
}

// Helpers

/// Strip leading zero bytes from a U256 big-endian representation.
fn trim_leading_zeros_be(val: U256) -> Vec<u8> {
    let be = val.to_bytes_be();
    let start = be.iter().position(|&b| b != 0).unwrap_or(be.len());
    be[start..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use bare_metal_evm_keccak::keccak256;
    use bare_metal_evm_trie::MemoryDB;
    use bare_metal_evm_types::U256_MAX;

    // Compile-time Send + Sync verification
    #[test]
    fn world_state_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<WorldState<MemoryDB>>();
        assert_sync::<WorldState<MemoryDB>>();
    }

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
        let _ = state.checkpoint();
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
        let _ = state.checkpoint();
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
        let hash = state.set_code(addr(1), code.clone());
        assert_eq!(state.get_code(&hash), Some(code));
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
        let mut acc = Account::new_empty();
        acc.nonce = U256::from_u64(1);
        state.set_account(addr(1), acc).unwrap();
        let root1 = state.commit().unwrap();
        assert_ne!(root1, EMPTY_ROOT_HASH);

        let mut acc = Account::new_empty();
        acc.nonce = U256::from_u64(2);
        state.set_account(addr(1), acc).unwrap();
        let root2 = state.commit().unwrap();
        assert_ne!(root2, root1);

        let got = state.get_account(&addr(1)).unwrap().unwrap();
        assert_eq!(got.nonce, U256::from_u64(2));
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
    fn state_multiple_accounts_shared_storage_root() {
        let mut state = WorldState::new(make_db());
        state.set_account(addr(1), Account::new_empty()).unwrap();
        state.set_account(addr(2), Account::new_empty()).unwrap();
        let root = state.commit().unwrap();
        let db = state.into_db();
        let loaded = WorldState::from_root(db, &root).unwrap();
        assert_eq!(
            loaded.get_storage(&addr(1), &U256::from_u64(0)).unwrap(),
            U256::zero()
        );
        assert_eq!(
            loaded.get_storage(&addr(2), &U256::from_u64(0)).unwrap(),
            U256::zero()
        );
    }

    #[test]
    fn state_root_empty() {
        let state = WorldState::new(make_db());
        assert_eq!(state.state_root(), EMPTY_ROOT_HASH);
    }

    #[test]
    fn state_root_after_commit() {
        let mut state = WorldState::new(make_db());
        let mut acc = Account::new_empty();
        acc.nonce = U256::from_u64(1);
        state.set_account(addr(1), acc).unwrap();
        let root = state.commit().unwrap();
        assert_eq!(state.state_root(), root);
        assert_ne!(root, EMPTY_ROOT_HASH);
    }

    #[test]
    fn state_root_from_root() {
        let mut state = WorldState::new(make_db());
        let mut acc = Account::new_empty();
        acc.nonce = U256::from_u64(1);
        state.set_account(addr(1), acc).unwrap();
        let root = state.commit().unwrap();
        let db = state.into_db();
        let loaded = WorldState::from_root(db, &root).unwrap();
        assert_eq!(loaded.state_root(), root);
    }

    #[test]
    fn state_add_balance() {
        let mut state = WorldState::new(make_db());
        state.add_balance(addr(1), U256::from_u64(42)).unwrap();
        assert_eq!(
            state.get_account(&addr(1)).unwrap().unwrap().balance,
            U256::from_u64(42)
        );
    }

    #[test]
    fn state_sub_balance() {
        let mut state = WorldState::new(make_db());
        state.add_balance(addr(1), U256::from_u64(100)).unwrap();
        state.sub_balance(addr(1), U256::from_u64(30)).unwrap();
        assert_eq!(
            state.get_account(&addr(1)).unwrap().unwrap().balance,
            U256::from_u64(70)
        );
    }

    #[test]
    fn state_increment_nonce() {
        let mut state = WorldState::new(make_db());
        state.increment_nonce(addr(1)).unwrap();
        state.increment_nonce(addr(1)).unwrap();
        assert_eq!(
            state.get_account(&addr(1)).unwrap().unwrap().nonce,
            U256::from_u64(2)
        );
    }

    #[test]
    fn state_add_sub_balance_auto_creates_account() {
        let mut state = WorldState::new(make_db());
        state.add_balance(addr(1), U256::from_u64(50)).unwrap();
        state.sub_balance(addr(1), U256::from_u64(20)).unwrap();
        let acc = state.get_account(&addr(1)).unwrap().unwrap();
        assert_eq!(acc.balance, U256::from_u64(30));
        assert_eq!(acc.nonce, U256::zero());
    }

    #[test]
    fn state_set_storage_then_remove_account_rollback_restores_storage() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(1);

        state
            .set_storage(addr(1), slot, U256::from_u64(42))
            .unwrap();
        state.commit().unwrap();

        let _ = state.checkpoint();
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

    /// Regression: `remove_account` then `set_storage` then rollback should restore
    /// the committed storage value, not lose it.
    #[test]
    fn state_remove_account_then_set_storage_rollback_preserves_storage() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(1);

        // Commit account with storage
        state
            .set_storage(addr(1), slot, U256::from_u64(42))
            .unwrap();
        state.commit().unwrap();
        assert_eq!(
            state.get_storage(&addr(1), &slot).unwrap(),
            U256::from_u64(42)
        );

        let _ = state.checkpoint();

        // Remove the account
        state.remove_account(&addr(1)).unwrap();
        assert_eq!(state.get_account(&addr(1)).unwrap(), None);

        // Set storage again (auto-creates account)
        state
            .set_storage(addr(1), slot, U256::from_u64(99))
            .unwrap();

        // Rollback — should restore committed account + storage
        state.rollback().unwrap();

        let acc = state.get_account(&addr(1)).unwrap().unwrap();
        assert_eq!(acc.nonce, U256::zero());
        assert_eq!(
            state.get_storage(&addr(1), &slot).unwrap(),
            U256::from_u64(42)
        );
    }

    #[test]
    fn state_sub_balance_overflow_returns_error() {
        let mut state = WorldState::new(make_db());
        state.add_balance(addr(1), U256::zero()).unwrap();
        let result = state.sub_balance(addr(1), U256::one());
        assert_eq!(result, Err(Error::Arithmetic));
    }

    #[test]
    fn state_add_balance_overflow_returns_error() {
        let mut state = WorldState::new(make_db());
        state.add_balance(addr(1), U256_MAX).unwrap();
        let result = state.add_balance(addr(1), U256::one());
        assert_eq!(result, Err(Error::Arithmetic));
    }

    #[test]
    fn state_increment_nonce_overflow_returns_error() {
        let mut state = WorldState::new(make_db());
        let mut acc = Account::new_empty();
        acc.nonce = U256_MAX;
        state.set_account(addr(1), acc).unwrap();
        let result = state.increment_nonce(addr(1));
        assert_eq!(result, Err(Error::Arithmetic));
    }

    #[test]
    fn state_code_persists_after_commit_and_from_root() {
        let mut state = WorldState::new(make_db());
        let code = vec![0x60, 0x01];
        let hash = state.set_code(addr(1), code.clone());
        assert_eq!(state.get_code(&hash), Some(code.clone()));

        // Create an account referencing the code hash so it survives EIP-158 pruning
        let mut acc = Account::new_empty();
        acc.code_hash = hash;
        acc.nonce = U256::from_u64(1);
        state.set_account(addr(1), acc).unwrap();

        let root = state.commit().unwrap();
        assert_ne!(root, EMPTY_ROOT_HASH);

        // After commit, code should be accessible via DB fallback
        assert_eq!(state.get_code(&hash), Some(code.clone()));

        let db = state.into_db();
        let loaded = WorldState::from_root(db, &root).unwrap();
        assert_eq!(loaded.get_code(&hash), Some(code));
    }

    #[test]
    fn state_eip158_empty_account_pruned_on_commit() {
        let mut state = WorldState::new(make_db());
        state.set_account(addr(1), Account::new_empty()).unwrap();
        let root = state.commit().unwrap();
        assert_eq!(root, EMPTY_ROOT_HASH);
        assert_eq!(state.get_account(&addr(1)).unwrap(), None);
    }

    #[test]
    fn state_eip158_nonempty_account_survives_commit() {
        let mut state = WorldState::new(make_db());
        let mut acc = Account::new_empty();
        acc.nonce = U256::from_u64(1);
        state.set_account(addr(1), acc).unwrap();
        let root = state.commit().unwrap();
        assert_ne!(root, EMPTY_ROOT_HASH);
        assert!(state.get_account(&addr(1)).unwrap().is_some());
    }

    #[test]
    fn state_get_storage_returns_zero_for_deleted_account() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(1);
        state
            .set_storage(addr(1), slot, U256::from_u64(42))
            .unwrap();
        state.remove_account(&addr(1)).unwrap();
        assert_eq!(state.get_storage(&addr(1), &slot).unwrap(), U256::zero());
    }

    #[test]
    fn state_set_storage_on_pruned_account() {
        let mut state = WorldState::new(make_db());
        let slot = U256::from_u64(1);
        let val = U256::from_u64(42);

        // Create and commit empty account (will be pruned by EIP-158)
        state.set_account(addr(1), Account::new_empty()).unwrap();
        state.commit().unwrap();
        assert_eq!(state.get_account(&addr(1)).unwrap(), None);

        // Now set storage on the pruned account (auto-creates it)
        state.set_storage(addr(1), slot, val).unwrap();
        let acc = state.get_account(&addr(1)).unwrap().unwrap();
        assert_eq!(acc.nonce, U256::zero());
        assert_eq!(state.get_storage(&addr(1), &slot).unwrap(), val);
    }

    #[test]
    fn state_set_code_rollback_removes_code() {
        let mut state = WorldState::new(make_db());
        let code = vec![0x60, 0x01];
        let hash = keccak256(&code);

        let _ = state.checkpoint();
        assert!(state.get_code(&hash).is_none());
        let _ = state.set_code(addr(1), code.clone());
        assert_eq!(state.get_code(&hash), Some(code.clone()));

        state.rollback().unwrap();
        assert!(state.get_code(&hash).is_none());
    }

    #[test]
    fn state_set_code_rollback_with_preexisting_code() {
        let mut state = WorldState::new(make_db());
        let code1 = vec![0x60, 0x01];
        let code2 = vec![0x60, 0x02];

        // Insert code1, commit (persists to DB)
        let hash1 = state.set_code(addr(1), code1.clone());
        assert_eq!(state.get_code(&hash1), Some(code1.clone()));

        let _ = state.checkpoint();
        // Replace code1 with code2
        let hash2 = state.set_code(addr(1), code2.clone());
        assert_eq!(state.get_code(&hash2), Some(code2.clone()));

        state.rollback().unwrap();
        // After rollback, code1 should still be accessible (it was inserted before checkpoint)
        assert_eq!(state.get_code(&hash1), Some(code1));
        // code2 should be gone (it was inserted inside the checkpoint)
        assert!(state.get_code(&hash2).is_none());
    }

    #[test]
    fn state_set_code_journaled_across_checkpoint() {
        let mut state = WorldState::new(make_db());
        let code_a = vec![0x60, 0xaa];
        let code_b = vec![0x60, 0xbb];

        let _ = state.checkpoint();
        let hash_a = state.set_code(addr(1), code_a.clone());
        assert_eq!(state.get_code(&hash_a), Some(code_a.clone()));

        let _ = state.checkpoint();
        let hash_b = state.set_code(addr(1), code_b.clone());
        assert_eq!(state.get_code(&hash_b), Some(code_b.clone()));

        // Rollback inner checkpoint — only code_b should be removed
        state.rollback().unwrap();
        assert_eq!(state.get_code(&hash_a), Some(code_a));
        assert!(state.get_code(&hash_b).is_none());

        // Rollback outer checkpoint — code_a should be removed
        state.rollback().unwrap();
        assert!(state.get_code(&hash_a).is_none());
    }

    #[test]
    fn state_get_code_ref_zero_copy() {
        let mut state = WorldState::new(make_db());
        let code = vec![0x60, 0xaa, 0x60, 0xbb];
        let hash = keccak256(&code);

        // Before insert — no code in cache
        assert!(state.get_code_ref(&hash).is_none());

        let _ = state.set_code(addr(1), code.clone());

        // get_code_ref returns the same bytes without cloning
        let ref_result = state.get_code_ref(&hash);
        assert_eq!(ref_result, Some(code.as_slice()));

        // After commit, cache is cleared — get_code_ref returns None
        state.commit().unwrap();
        // get_code should still work (DB fallback)
        assert_eq!(state.get_code(&hash), Some(code));
        // get_code_ref returns None (cache empty)
        assert!(state.get_code_ref(&hash).is_none());
    }

    #[test]
    fn state_stress_many_accounts() {
        let mut state = WorldState::new(make_db());
        let mut accounts = Vec::new();

        // Insert 1000 accounts
        for i in 0..1000u64 {
            let addr = {
                let mut a = [0u8; 20];
                a[0..8].copy_from_slice(&i.to_le_bytes());
                a
            };
            state.add_balance(addr, U256::from_u64(i)).unwrap();
            state.increment_nonce(addr).unwrap();
            accounts.push(addr);
        }

        // Verify all accounts exist
        for (i, addr) in accounts.iter().enumerate() {
            assert!(state.account_exists(addr).unwrap());
            assert_eq!(
                state.get_account(addr).unwrap().unwrap().balance,
                U256::from_u64(i as u64)
            );
            assert_eq!(
                state.get_account(addr).unwrap().unwrap().nonce,
                U256::from_u64(1)
            );
        }

        // Commit and rebuild from root
        let root = state.commit().unwrap();
        let state2 = WorldState::from_root(state.into_db(), &root).unwrap();

        // Verify after reload
        for (i, addr) in accounts.iter().enumerate() {
            assert!(state2.account_exists(addr).unwrap());
            assert_eq!(
                state2.get_account(addr).unwrap().unwrap().balance,
                U256::from_u64(i as u64)
            );
        }
    }
}
