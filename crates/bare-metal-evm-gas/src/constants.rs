// Gas cost constants, sourced from go-ethereum params/protocol_params.go
// and the relevant EIP specifications.

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
pub const SSTORE_RESET_GAS_EIP2929: u64 = 5_000 - COLD_SLOAD_COST;
pub const SSTORE_CLEARS_SCHEDULE: u64 = 4_800;
pub const SSTORE_SENTRY_GAS: u64 = 2_300;

// CALL variants (EIP-150 63/64 rule)

pub const CALL_GAS: u64 = 700;
pub const CALL_VALUE_TRANSFER_GAS: u64 = 9_000;
pub const CALL_STIPEND: u64 = 2_300;
pub const CALL_NEW_ACCOUNT_GAS: u64 = 25_000;

// CREATE / CREATE2

pub const CREATE_GAS: u64 = 32_000;
pub const CREATE2_GAS: u64 = 32_000;
pub const CREATE_DATA_GAS: u64 = 200;
// EIP-3860 — initcode cost metering
pub const INIT_CODE_WORD_GAS: u64 = 2;
pub const MAX_INIT_CODE_SIZE: u64 = 49_152;
pub const MAX_CODE_SIZE: u64 = 24_576;

// SELFDESTRUCT (EIP-150, EIP-6780)

pub const SELFDESTRUCT_GAS_EIP150: u64 = 5_000;
pub const SELFDESTRUCT_REFUND_GAS: u64 = 24_000;

// LOG

pub const LOG_GAS: u64 = 375;
pub const LOG_TOPIC_GAS: u64 = 375;
pub const LOG_DATA_GAS: u64 = 8;

// EXP

pub const EXP_GAS: u64 = 10;
pub const EXP_BYTE_GAS: u64 = 50;

// SHA3 (KECCAK256)

pub const KECCAK256_GAS: u64 = 30;
pub const KECCAK256_WORD_GAS: u64 = 6;

// Copy operations (CALLDATACOPY, CODECOPY, EXTCODECOPY, RETURNDATACOPY, MCOPY)

pub const COPY_GAS: u64 = 3;

// JUMPDEST

pub const JUMPDEST_GAS: u64 = 1;

// EIP-1153 transient storage

pub const TLOAD_GAS: u64 = WARM_STORAGE_READ_COST;
pub const TSTORE_GAS: u64 = WARM_STORAGE_READ_COST;

// Refund (EIP-3529)

pub const MAX_REFUND_QUOTIENT: u64 = 5;

// EIP-7623 — calldata floor divisor (Pectra)

pub const EIP7623_CALLDATA_FLOOR_DIVISOR: u64 = 10;

// ── Precompile gas costs ──────────────────────────────────────────

// ECRECOVER
pub const ECRECOVER_GAS: u64 = 3_000;

// SHA256
pub const SHA256_BASE_GAS: u64 = 60;
pub const SHA256_PER_WORD_GAS: u64 = 12;

// RIPEMD160
pub const RIPEMD160_BASE_GAS: u64 = 600;
pub const RIPEMD160_PER_WORD_GAS: u64 = 120;

// IDENTITY
pub const IDENTITY_BASE_GAS: u64 = 15;
pub const IDENTITY_PER_WORD_GAS: u64 = 3;

// MODEXP (precompile 05) — handled via quadratic formula, constants here approximate
pub const MODEXP_HEADER_LEN: u64 = 96;

// Bn256Add (precompile 06) — updated by EIP-1108 (Istanbul)
pub const BN256ADD_GAS_ISTANBUL: u64 = 150;

// Bn256ScalarMul (precompile 07) — updated by EIP-1108 (Istanbul)
pub const BN256SCALARMUL_GAS_ISTANBUL: u64 = 6_000;

// Bn256Pairing (precompile 08) — updated by EIP-1108 (Istanbul)
pub const BN256PAIRING_BASE_GAS_ISTANBUL: u64 = 45_000;
pub const BN256PAIRING_PER_POINT_GAS_ISTANBUL: u64 = 34_000;

// Bls12381G1Add (precompile 10)
pub const BLS12381_G1ADD_GAS: u64 = 375;

// Bls12381G1Mul (precompile 11)
pub const BLS12381_G1MUL_GAS: u64 = 12_000;

// Bls12381G2Add (precompile 12)
pub const BLS12381_G2ADD_GAS: u64 = 600;

// Bls12381G2Mul (precompile 13)
pub const BLS12381_G2MUL_GAS: u64 = 22_500;

// Bls12381Pairing (precompile 14)
pub const BLS12381_PAIRING_BASE_GAS: u64 = 37_700;
pub const BLS12381_PAIRING_PER_PAIR_GAS: u64 = 32_600;

// Bls12381MapG1 (precompile 15)
pub const BLS12381_MAP_G1_GAS: u64 = 5_500;

// Bls12381MapG2 (precompile 16)
pub const BLS12381_MAP_G2_GAS: u64 = 23_800;

// P256Verify (precompile 17, added by EIP-7212)
pub const P256VERIFY_GAS: u64 = 6_900;

// BlobPointEval (precompile 18, added by EIP-4844)
pub const BLOB_POINT_EVAL_GAS: u64 = 50_000;
