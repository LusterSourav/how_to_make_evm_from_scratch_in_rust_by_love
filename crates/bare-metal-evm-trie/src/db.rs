use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Database abstraction for trie node storage.
#[allow(clippy::result_unit_err)]
pub trait Database: Send + Sync {
    fn get(&self, key: &[u8; 32]) -> Result<Option<Vec<u8>>, ()>;
    fn insert(&mut self, key: [u8; 32], value: Vec<u8>) -> Result<(), ()>;
    fn remove(&mut self, _key: &[u8; 32]) {}
}

/// In-memory trie node database backed by a BTreeMap.
#[derive(Clone, Debug)]
pub struct MemoryDB {
    store: BTreeMap<[u8; 32], Vec<u8>>,
}

impl MemoryDB {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: BTreeMap::new(),
        }
    }
}

impl Default for MemoryDB {
    fn default() -> Self {
        Self::new()
    }
}

impl Database for MemoryDB {
    fn get(&self, key: &[u8; 32]) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.store.get(key).cloned())
    }

    fn insert(&mut self, key: [u8; 32], value: Vec<u8>) -> Result<(), ()> {
        self.store.insert(key, value);
        Ok(())
    }

    fn remove(&mut self, key: &[u8; 32]) {
        self.store.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn memory_db_roundtrip() {
        let mut db = MemoryDB::new();
        let key = [0xabu8; 32];
        let value = vec![0x01, 0x02, 0x03];
        db.insert(key, value.clone()).unwrap();
        assert_eq!(db.get(&key).unwrap(), Some(value));
    }

    #[test]
    fn memory_db_missing_key() {
        let db = MemoryDB::new();
        let key = [0xffu8; 32];
        assert_eq!(db.get(&key).unwrap(), None);
    }

    #[test]
    fn memory_db_overwrite() {
        let mut db = MemoryDB::new();
        let key = [0xabu8; 32];
        db.insert(key, vec![0x01]).unwrap();
        db.insert(key, vec![0x02]).unwrap();
        assert_eq!(db.get(&key).unwrap(), Some(vec![0x02]));
    }

    #[test]
    fn memory_db_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<MemoryDB>();
        assert_sync::<MemoryDB>();
    }
}
