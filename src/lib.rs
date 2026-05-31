#![no_std]
#![cfg_attr(feature = "runtime", feature(alloc_error_handler))]

extern crate alloc;

pub use bare_metal_evm_types::{U256, U256_MAX, U512};
pub use bare_metal_evm_keccak::keccak256;
pub use bare_metal_evm_rlp::{
    decode, decode_strict, encode_list, encode_list_from_iter, encode_str, encode_u256, RlpError,
    RlpItem,
};
pub use bare_metal_evm_nibble::{
    encode_nibble_path_padded, from_byte, high_nibble, hp_decode, hp_encode, low_nibble,
    nibbles_to_byte, Nibble, NibbleBuf, NibbleIterator, NibblePathPacked, MAX_NIBBLES,
    MAX_PACKED_BYTES,
};
pub use bare_metal_evm_trie::{delete_trie_nodes, Database, Error as TrieError, MemoryDB, Node as TrieNode, NodeRef as TrieNodeRef, Trie, EMPTY_ROOT_HASH};
pub use bare_metal_evm_state::{Account, EMPTY_CODE_HASH, Error as StateError, Journal, JournalEntry, WorldState};

#[cfg(feature = "runtime")]
pub mod lang_items;
