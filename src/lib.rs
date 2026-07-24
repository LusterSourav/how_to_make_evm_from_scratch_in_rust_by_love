#![no_std]

//workspace root re-exports, one stop for all crate-level types
//this file started as a two-liner and grew, neeeds a trim pass eventually

pub use bare_metal_evm_engine::{execute, Error as EngineError, MachineState, Memory, Stack};
pub use bare_metal_evm_gas::{
    constants::{
        BLOB_POINT_EVAL_GAS, BLS12381_G1ADD_GAS, BLS12381_G1MUL_GAS, BLS12381_G2ADD_GAS,
        BLS12381_G2MUL_GAS, BLS12381_MAP_G1_GAS, BLS12381_MAP_G2_GAS, BLS12381_PAIRING_BASE_GAS,
        BLS12381_PAIRING_PER_PAIR_GAS, BN256ADD_GAS_ISTANBUL, BN256PAIRING_BASE_GAS_ISTANBUL,
        BN256PAIRING_PER_POINT_GAS_ISTANBUL, BN256SCALARMUL_GAS_ISTANBUL, CALL_GAS,
        CALL_NEW_ACCOUNT_GAS, CALL_STIPEND, CALL_VALUE_TRANSFER_GAS, COLD_ACCOUNT_ACCESS_COST,
        COLD_SLOAD_COST, COPY_GAS, CREATE_DATA_GAS, CREATE_GAS, ECRECOVER_GAS,
        EIP7623_CALLDATA_FLOOR_DIVISOR, EXP_BYTE_GAS, EXP_GAS, IDENTITY_BASE_GAS,
        IDENTITY_PER_WORD_GAS, INIT_CODE_WORD_GAS, JUMPDEST_GAS, KECCAK256_GAS, KECCAK256_WORD_GAS,
        LOG_DATA_GAS, LOG_GAS, LOG_TOPIC_GAS, MAX_INIT_CODE_SIZE, MAX_REFUND_QUOTIENT, MEMORY_GAS,
        MEMORY_MAX_SIZE, P256VERIFY_GAS, QUAD_COEFF_DIV, RIPEMD160_BASE_GAS,
        RIPEMD160_PER_WORD_GAS, SELFDESTRUCT_GAS_EIP150, SHA256_BASE_GAS, SHA256_PER_WORD_GAS,
        SSTORE_CLEARS_SCHEDULE, SSTORE_RESET_GAS_EIP2929, SSTORE_SENTRY_GAS, SSTORE_SET_GAS,
        TLOAD_GAS, TSTORE_GAS, TX_ACCESS_LIST_ADDRESS_GAS, TX_ACCESS_LIST_STORAGE_KEY_GAS,
        TX_CREATE_GAS, TX_DATA_NON_ZERO_GAS, TX_DATA_ZERO_GAS, TX_GAS, WARM_STORAGE_READ_COST,
    },
    copy::copy_gas,
    create::{create_gas, initcode_word_cost},
    exp::exp_gas,
    log::log_gas,
    precompile::{bls12381_pairing_gas, bn256_pairing_gas, precompile_gas},
    selfdestruct::selfdestruct_gas,
    sha3::sha3_gas,
    AccessListItem, GasError, GasMeter,
};
pub use bare_metal_evm_keccak::keccak256;
pub use bare_metal_evm_nibble::{
    encode_nibble_path_padded, from_byte, high_nibble, hp_decode, hp_encode, low_nibble,
    nibbles_to_byte, Nibble, NibbleBuf, NibbleError, NibbleIterator, NibblePathPacked, MAX_NIBBLES,
    MAX_PACKED_BYTES,
};
pub use bare_metal_evm_rlp::{
    decode, decode_strict, encode_list, encode_list_from_iter, encode_str, encode_u256, RlpError,
    RlpItem,
};
pub use bare_metal_evm_state::{
    Account, Error as StateError, Journal, JournalEntry, WorldState, EMPTY_CODE_HASH,
};
pub use bare_metal_evm_trie::{
    delete_trie_nodes, Database, Error as TrieError, MemoryDB, Node as TrieNode,
    NodeRef as TrieNodeRef, Trie, EMPTY_ROOT_HASH,
};
pub use bare_metal_evm_types::{U256, U256_MAX, U512};

//re-exports so callers dont dig into sub-crates
//these will get trimmed once the api surface stabilizes

#[cfg(feature = "runtime")]
pub mod lang_items;
