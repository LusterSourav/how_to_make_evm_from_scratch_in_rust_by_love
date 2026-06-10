// Gas cost constants, sourced from go-ethereum params/protocol_params.go
// and the relevant EIP specifications.

// Opcode gas tiers

pub const ZERO: u64 = 0;
pub const BASE: u64 = 2;
pub const VERYLOW: u64 = 3;
pub const LOW: u64 = 5;
pub const MID: u64 = 8;
pub const HIGH: u64 = 10;

// Transaction intrinsic gas

pub const TX_GAS: u64 = 21_000;
pub const TX_CREATE_GAS: u64 = 53_000;
pub const TX_DATA_ZERO_GAS: u64 = 4;
pub const TX_DATA_NON_ZERO_GAS: u64 = 16;

// Memory expansion — quadratic cost formula

pub const MEMORY_GAS: u64 = 3;
pub const MEMORY_MAX_SIZE: usize = 1 << 20;
pub const QUAD_COEFF_DIV: u64 = 512;

// EIP-2929 access list costs

pub const COLD_ACCOUNT_ACCESS_COST: u64 = 2_600;
pub const COLD_SLOAD_COST: u64 = 2_100;
pub const WARM_STORAGE_READ_COST: u64 = 100;
pub const TX_ACCESS_LIST_ADDRESS_GAS: u64 = 2_400;
pub const TX_ACCESS_LIST_STORAGE_KEY_GAS: u64 = 1_900;

// SSTORE costs (EIP-2200 + EIP-2929 + EIP-3529 + EIP-6780)

pub const SSTORE_SET_GAS: u64 = 20_000;
pub const SSTORE_RESET_GAS: u64 = 5_000;
pub const SSTORE_RESET_GAS_EIP2929: u64 =
    SSTORE_RESET_GAS + COLD_SLOAD_COST - WARM_STORAGE_READ_COST;
pub const SSTORE_CLEARS_SCHEDULE: u64 = 4_800;
pub const SSTORE_SENTRY_GAS: u64 = 2_300;

// CALL / CREATE (EIP-150 63/64 rule)

pub const CALL_GAS: u64 = 700;
pub const CALL_COLD_GAS: u64 = COLD_ACCOUNT_ACCESS_COST;
pub const CALL_VALUE_TRANSFER_GAS: u64 = 9_000;
pub const CALL_STIPEND: u64 = 2_300;
pub const CALL_NEW_ACCOUNT_GAS: u64 = 25_000;

// EIP-1153 transient storage

pub const TLOAD_GAS: u64 = WARM_STORAGE_READ_COST;
pub const TSTORE_GAS: u64 = WARM_STORAGE_READ_COST;

// Refund (EIP-3529)

pub const SSTORE_REFUND: u64 = 4_800;
pub const MAX_REFUND_QUOTIENT: u64 = 5;

// KECCAK256 (SHA3)

pub const KECCAK256: u64 = 30;
pub const KECCAK256_WORD: u64 = 6;

// Copy operations (CALLDATACOPY, CODECOPY, RETURNDATACOPY)

pub const COPY: u64 = 3;
pub const COPY_QUAD_COEFF: u64 = 3;

// LOG

pub const LOG_TOPIC: u64 = 375;
pub const LOG_DATA: u64 = 8;

// EXP

pub const EXP: u64 = 10;
pub const EXP_BYTE: u64 = 50;

// CREATE / CREATE2 (EIP-3860)

pub const CREATE: u64 = 32_000;
pub const CREATE2: u64 = 32_000;
pub const CREATE2_DATA: u64 = 6;
pub const INITCODE_WORD_GAS: u64 = 2;

// SELFDESTRUCT (EIP-6780 — only sends value to beneficiary)

pub const SELFDESTRUCT: u64 = 5_000;
pub const SELFDESTRUCT_COLD_GAS: u64 = COLD_ACCOUNT_ACCESS_COST;
pub const SELFDESTRUCT_BENEFICIARY_GAS: u64 = 2_500;

// Code deposit cost per byte

pub const CODE_DEPOSIT: u64 = 200;

// EIP-7623 — calldata floor divisor (Pectra)

pub const EIP7623_CALLDATA_FLOOR_DIVISOR: u64 = 10;
