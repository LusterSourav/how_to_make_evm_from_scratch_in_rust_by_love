use alloc::vec::Vec;

use bare_metal_evm_types::U256;

/// An atomic change to the world state that can be rolled back.
#[derive(Clone, Debug, PartialEq)]
pub enum JournalEntry {
    /// An account was created or modified.
    AccountChange {
        address: [u8; 20],
        old: Option<crate::account::Account>,
    },
    /// A storage slot was changed.
    StorageChange {
        address: [u8; 20],
        slot: U256,
        old: U256,
    },
}

/// An append-only journal with checkpoint-based rollback support.
#[derive(Clone, Debug)]
pub struct Journal {
    pub(crate) entries: Vec<JournalEntry>,
    pub(crate) checkpoints: Vec<usize>,
}

impl Journal {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            checkpoints: Vec::new(),
        }
    }

    /// Append an entry to the journal.
    pub fn push(&mut self, entry: JournalEntry) {
        self.entries.push(entry);
    }

    /// Remove and return the most recent entry.
    pub fn pop(&mut self) -> Option<JournalEntry> {
        self.entries.pop()
    }

    /// Save the current entry count as a checkpoint.
    pub fn checkpoint(&mut self) {
        self.checkpoints.push(self.entries.len());
    }

    /// Roll back all entries since the most recent checkpoint.
    /// Returns `false` if no checkpoint exists.
    pub fn rollback(&mut self) -> bool {
        let target = match self.checkpoints.pop() {
            Some(t) => t,
            None => return false,
        };
        self.entries.truncate(target);
        true
    }

    /// Discard the most recent checkpoint without rolling back.
    /// Returns `false` if no checkpoint exists.
    pub fn commit_checkpoint(&mut self) -> bool {
        self.checkpoints.pop().is_some()
    }

    /// Number of entries in the journal.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the journal is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

}

impl Default for Journal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_empty() {
        let j = Journal::new();
        assert!(j.is_empty());
    }

    #[test]
    fn journal_push() {
        let mut j = Journal::new();
        j.push(JournalEntry::AccountChange {
            address: [0u8; 20],
            old: None,
        });
        assert_eq!(j.len(), 1);
    }

    #[test]
    fn journal_checkpoint_rollback() {
        let mut j = Journal::new();
        j.push(JournalEntry::AccountChange {
            address: [1u8; 20],
            old: None,
        });
        j.checkpoint();
        j.push(JournalEntry::AccountChange {
            address: [2u8; 20],
            old: None,
        });
        assert_eq!(j.len(), 2);
        assert!(j.rollback());
        assert_eq!(j.len(), 1);
    }

    #[test]
    fn journal_nested_checkpoints() {
        let mut j = Journal::new();
        j.checkpoint();
        j.push(JournalEntry::AccountChange {
            address: [1u8; 20],
            old: None,
        });
        j.checkpoint();
        j.push(JournalEntry::AccountChange {
            address: [2u8; 20],
            old: None,
        });
        assert_eq!(j.len(), 2);
        assert!(j.rollback());
        assert_eq!(j.len(), 1);
        assert!(j.rollback());
        assert_eq!(j.len(), 0);
    }

    #[test]
    fn journal_rollback_without_checkpoint() {
        let mut j = Journal::new();
        assert!(!j.rollback());
    }

    #[test]
    fn journal_commit_checkpoint() {
        let mut j = Journal::new();
        j.checkpoint();
        j.push(JournalEntry::AccountChange {
            address: [1u8; 20],
            old: None,
        });
        assert!(j.commit_checkpoint());
        // entries remain
        assert_eq!(j.len(), 1);
        // no checkpoint to rollback to
        assert!(!j.rollback());
    }

    #[test]
    fn journal_pop_empties() {
        let mut j = Journal::new();
        j.push(JournalEntry::AccountChange {
            address: [1u8; 20],
            old: None,
        });
        j.push(JournalEntry::AccountChange {
            address: [2u8; 20],
            old: None,
        });
        assert!(j.pop().is_some());
        assert!(j.pop().is_some());
        assert!(j.is_empty());
    }
}
