#![no_std]
extern crate alloc;

pub mod keccak;

pub use keccak::keccak256;
