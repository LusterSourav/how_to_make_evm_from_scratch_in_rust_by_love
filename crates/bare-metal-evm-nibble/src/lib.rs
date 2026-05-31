#![no_std]
extern crate alloc;

pub mod nibble;

pub use nibble::{encode_nibble_path_padded, from_byte, high_nibble, hp_decode, hp_encode, low_nibble, MAX_NIBBLES, MAX_PACKED_BYTES, nibbles_to_byte, Nibble, NibbleBuf, NibbleIterator, NibblePathPacked};
