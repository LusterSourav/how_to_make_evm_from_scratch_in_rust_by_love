#![no_std]
#![deny(unsafe_code)]

extern crate alloc;

pub mod error;
pub mod machine;
pub mod memory;
pub mod opcodes;
pub mod stack;

pub use error::Error;
pub use machine::{execute, MachineState};
pub use memory::Memory;
pub use stack::Stack;
