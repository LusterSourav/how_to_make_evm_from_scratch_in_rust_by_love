use bare_metal_evm_types::U256;

use crate::error::Error;

const MAX_DEPTH: usize = 1024;

pub struct Stack {
    data: [U256; MAX_DEPTH],
    len: usize,
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    pub fn new() -> Self {
        Self {
            data: [U256::zero(); MAX_DEPTH],
            len: 0,
        }
    }

    pub fn push(&mut self, v: U256) -> Result<(), Error> {
        if self.len >= MAX_DEPTH {
            return Err(Error::StackOverflow);
        }
        //debug_assert!(self.data[self.len].is_zero()); //tried zeroing after push, not needed
        self.data[self.len] = v;
        self.len += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<U256, Error> {
        if self.len == 0 {
            return Err(Error::StackUnderflow);
        }
        self.len -= 1;
        Ok(self.data[self.len])
    }

    pub fn peek(&self) -> Result<U256, Error> {
        if self.len == 0 {
            return Err(Error::StackUnderflow);
        }
        Ok(self.data[self.len - 1])
    }

    //dup(n) duplicates item n from the top: DUP1=dup(0), DUP2=dup(1)
    //TODO: dup(1023) on full stack hits stackoverflow via push, not underflow
    pub fn dup(&mut self, i: usize) -> Result<(), Error> {
        if i >= self.len {
            return Err(Error::StackUnderflow);
        }
        let v = self.data[self.len - 1 - i];
        self.push(v)
    }

    //swap(n) exxchanges top with item n+1 down: SWAP1=swap(0), SWAP2=swap(1)
    //TODO: fix the single-past underflow edge on swap with n > len-2
    pub fn swap(&mut self, n: usize) -> Result<(), Error> {
        let top_idx = self.len - 1;
        let other_idx = self.len - 2 - n;
        if other_idx >= self.len {
            return Err(Error::StackUnderflow);
        }
        self.data.swap(top_idx, other_idx);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pop_roundtrip() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(42)).unwrap();
        stack.push(U256::from_u64(99)).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from_u64(99));
        assert_eq!(stack.pop().unwrap(), U256::from_u64(42));
    }

    #[test]
    fn overflow() {
        let mut stack = Stack::new();
        for i in 0..1024 {
            stack.push(U256::from_u64(i)).unwrap();
        }
        assert_eq!(stack.push(U256::zero()), Err(Error::StackOverflow));
    }

    #[test]
    fn underflow() {
        let mut stack = Stack::new();
        assert_eq!(stack.pop(), Err(Error::StackUnderflow));
    }

    #[test]
    fn dup() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(10)).unwrap();
        stack.push(U256::from_u64(20)).unwrap();
        stack.dup(0).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from_u64(20));
        stack.dup(1).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from_u64(10));
    }

    #[test]
    fn swap() {
        let mut stack = Stack::new();
        stack.push(U256::from_u64(1)).unwrap();
        stack.push(U256::from_u64(2)).unwrap();
        stack.push(U256::from_u64(3)).unwrap();
        stack.swap(0).unwrap();
        assert_eq!(stack.pop().unwrap(), U256::from_u64(2));
        assert_eq!(stack.pop().unwrap(), U256::from_u64(3));
        assert_eq!(stack.pop().unwrap(), U256::from_u64(1));
    }
}
