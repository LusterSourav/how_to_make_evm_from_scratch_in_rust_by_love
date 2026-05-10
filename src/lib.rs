// Bare Metal EVM — Arithmetic & Glue (Issue #7)
// =================================================
// A zero-dependency, first-principles implementation of 256-bit arithmetic
// for the Ethereum Virtual Machine, targeting bare-metal environments.
//
// Division uses Knuth's Algorithm D (TAOCP Vol 2, §4.3.1).
// Multiplication uses u128 intermediates for MULX instruction hints.
// Division by zero returns 0 per the EVM Yellow Paper.

#![no_std]
#![cfg_attr(feature = "runtime", feature(alloc_error_handler))]

pub mod ops;
pub mod types;

#[cfg(feature = "runtime")]
pub mod lang_items;

pub use types::U256;
pub use types::U256_MAX;
pub use types::U512;
