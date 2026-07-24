//engine crate: fetch-decode-execute loop, stack, memory, dispatch
#![no_std]
#![deny(unsafe_code)]

extern crate alloc;

pub mod error;
pub mod machine;
pub mod memory;
pub mod opcodes;
pub mod stack;
//pub mod gas; — kept in its own crate, no reason to re-export here
//was going to add a prelude module, decided against it for now

pub use error::Error;
pub use machine::{execute, MachineState};
pub use memory::Memory;
pub use stack::Stack;
