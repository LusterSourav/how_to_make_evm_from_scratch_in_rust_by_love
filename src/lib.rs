#![no_std]

// Public API — workspace root re-exports all crate-level APIs for convenience.

// --- Core EVM types ---
pub use bare_metal_evm_types::{U256, U256_MAX, U512};

// --- Keccak-256 hash ---
pub use bare_metal_evm_keccak::keccak256;

// --- RLP encoding / decoding ---
pub use bare_metal_evm_rlp::{
    decode, decode_strict, encode_list, encode_list_from_iter, encode_str, encode_u256, RlpError,
    RlpItem,
};

// --- Nibble / hex-prefix encoding for MPT ---
pub use bare_metal_evm_nibble::{
    // Reserved for future EVM use (nibble path packing at the EVM API layer):
    encode_nibble_path_padded,
    from_byte,
    high_nibble,
    hp_decode,
    hp_encode,
    low_nibble,
    nibbles_to_byte,
    Nibble,
    NibbleBuf,
    NibbleError,
    NibbleIterator,
    NibblePathPacked,
    MAX_NIBBLES,
    MAX_PACKED_BYTES,
};

// --- Merkle Patricia Trie ---
pub use bare_metal_evm_trie::{
    delete_trie_nodes, Database, Error as TrieError, MemoryDB, Node as TrieNode,
    NodeRef as TrieNodeRef, Trie, EMPTY_ROOT_HASH,
};

// --- World State ---
pub use bare_metal_evm_state::{
    Account, Error as StateError, Journal, JournalEntry, WorldState, EMPTY_CODE_HASH,
};

// --- Gas Metering ---
pub use bare_metal_evm_gas::{GasError, GasMeter};

// Reserved for future use: arithmetic extension
// U512, U256_MAX — extended arithmetic for EVM operations that need
// larger intermediate results (e.g. MUL, ADD with overflow detection).
//
// Reserved for future use: list encoding
// encode_list_from_iter — zero-allocation RLP list encoding from an
// iterator; currently all callers use `encode_list` with slices.
//
// Reserved for future use: MPT decoding
// decode — lenient top-level RLP decode for MPT node reconstruction.
// decode_strict and RlpItem are used for the EVM RLP data path.

#[cfg(feature = "runtime")]
pub mod lang_items;
