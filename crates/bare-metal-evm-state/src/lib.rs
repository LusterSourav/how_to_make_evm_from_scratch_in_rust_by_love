#![no_std]
#![deny(unsafe_code)]
extern crate alloc;

pub mod account;
pub mod journal;
pub mod state;

pub use account::{Account, EMPTY_CODE_HASH};
pub use journal::{Journal, JournalEntry};
pub use state::{Error, WorldState};
