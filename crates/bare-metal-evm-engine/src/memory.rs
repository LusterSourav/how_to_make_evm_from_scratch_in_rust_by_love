use alloc::vec::Vec;

pub struct Memory {
    data: Vec<u8>,
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

impl Memory {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Number of 32-byte words (rounded up).
    pub fn words(&self) -> usize {
        self.data.len().div_ceil(32)
    }

    // ponytail: read/write/resize added when MLOAD/MSTORE opcodes land (Phase 4.2)
}
