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

    ///32-byte words, rounded up. used for memory gas calculations.
    //div_ceil is the right tool here, avoids manual rounding
    pub fn words(&self) -> usize {
        self.data.len().div_ceil(32)
    }

    //read/write/resize land later with MLOAD/MSTORE in phase 4.2
    //tried storing size as a separate field, worked but added complexity for no gain
    //also tried a page-based layout (64 byte pages) but it was premature
}
