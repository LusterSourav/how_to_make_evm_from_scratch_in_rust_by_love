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

extern crate alloc;

pub mod keccak;
pub mod nibble;
pub mod ops;
pub mod rlp;
pub mod types;

#[cfg(feature = "runtime")]
pub mod lang_items;

pub use keccak::keccak256;
pub use nibble::encode_nibble_path_padded;
pub use nibble::from_byte;
pub use nibble::high_nibble;
pub use nibble::low_nibble;
pub use nibble::nibbles_to_byte;
pub use nibble::Nibble;
pub use nibble::NibbleIterator;
pub use nibble::NibblePathPacked;
pub use rlp::decode;
pub use rlp::decode_strict;
pub use rlp::encode_list;
pub use rlp::encode_list_from_iter;
pub use rlp::encode_str;
pub use rlp::encode_u256;
pub use rlp::RlpError;
pub use rlp::RlpItem;
pub use types::U256;
pub use types::U256_MAX;
pub use types::U512;
