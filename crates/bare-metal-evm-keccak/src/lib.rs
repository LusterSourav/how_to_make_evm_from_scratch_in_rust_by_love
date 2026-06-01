#![no_std]
#![deny(unsafe_code)]
// Required for `alloc::format!` and `Vec` in `#[cfg(test)]` test code.
#[allow(unused_extern_crates)]
extern crate alloc;

pub mod keccak;

pub use keccak::keccak256;
