#![no_std]
#![deny(unsafe_code)]
extern crate alloc;

pub mod nibble;

// Public re-exports forming the crate's stable surface.
//
// All items below are part of the published API. The most common
// (`low_nibble`, `high_nibble`, `nibbles_to_byte`, `from_key`,
// `from_nibbles`, `merge`, `hp_encode`, `hp_decode`, `NibbleBuf`,
// `NibbleError`, `MAX_NIBBLES`) are used internally by `bare-metal-evm-trie`.
//
// `from_byte`, `Nibble`, `NibbleIterator`, `NibblePathPacked`,
// `encode_nibble_path_padded`, and `MAX_PACKED_BYTES` are exposed for
// downstream consumers that build their own hex-prefix or Merkle-Patricia
// logic on top of this crate; they form the building-block layer that
// the trie depends on.
pub use nibble::{
    encode_nibble_path_padded, from_byte, high_nibble, hp_decode, hp_encode, low_nibble,
    nibbles_to_byte, Nibble, NibbleBuf, NibbleError, NibbleIterator, NibblePathPacked, MAX_NIBBLES,
    MAX_PACKED_BYTES,
};
