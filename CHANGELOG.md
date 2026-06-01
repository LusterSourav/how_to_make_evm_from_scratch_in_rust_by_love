# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-06-01

### Added

- **NibbleError typed enum** — `NibbleError { TooLong, InvalidValue }` replaces
  bare `Result<(), ()>` in nibble operations (`from_nibbles`, `merge`,
  `encode_nibble_path_padded`). Exported from `bare-metal-evm-nibble` and the
  root `bare-metal-evm` crate.
- **Account::Default impl** — delegates to `Account::new_empty()`.
- **Safe `set_code` API** — two-argument `set_code(&mut self, address, code: Vec<u8>)`
  computes `keccak256(&code)` internally and returns the hash. The old three-argument
  form is deprecated as `set_code_with_hash`.
- **Journal depth cap** — `Journal::MAX_JOURNAL_DEPTH = 4096`. `checkpoint()` returns
  `false` when the cap is reached instead of silently over-allocating.
- **Journal depth cap on WorldState** — `WorldState::checkpoint()` returns `bool`.
- **Commit refactor** — `commit()` is decomposed into five private sub-methods:
  `commit_storage`, `commit_prune_deleted_storage`, `commit_accounts`,
  `commit_code`, `commit_clear_caches`.
- **Trie root preservation on error** — `Trie::insert` and `Trie::remove` no longer
  destroy the in-memory root on DB errors. The root is cloned before mutation and
  restored on failure.

### Fixed

- **keccak.rs safety** — `debug_assert!` changed to `assert!` on XOR block slice
  bounds check. Division by `RATE` uses `wrapping_div` with a safety comment.
- **Nibble operation panic safety** — removed `#[allow(clippy::result_unit_err)]`
  from nibble crate (three locations).

### Changed

- **`encode_list_from_iter`** — simplified implementation to use idiomatic
  `collect` + `iter().map().sum()` instead of a manual double-accumulation loop.
- **Deprecated** — `set_code(address, code_hash, code)` renamed to
  `set_code_with_hash`. Use `set_code(address, code)` for automatic hashing.

### Removed

- Three `#[allow(clippy::result_unit_err)]` annotations from nibble operations.

## [0.1.0] - 2025

### Added

- Initial release with six crates:
  - `bare-metal-evm-types` — U256, U512 arithmetic (schoolbook mul, Knuth D div)
  - `bare-metal-evm-keccak` — Keccak-256 sponge (24 rounds, correct Ethereum padding)
  - `bare-metal-evm-rlp` — RLP encode/decode (strict minimalism enforced)
  - `bare-metal-evm-nibble` — virtual u4 nibble system, HP encoding
  - `bare-metal-evm-trie` — Modified Merkle Patricia Trie (4 node types, inline opt)
  - `bare-metal-evm-state` — World state (accounts, storage, journal, EIP-158)
- Secp256k1 elliptic curve arithmetic (field ops, point addition/doubling, ECDSA)
- EVM execution engine (fetch-decode-execute loop, function pointer dispatch table)
- Static jump map analysis (PUSH-aware JUMPDEST validation)
- State journaling with checkpoint/rollback
- Gas metering (quadratic memory expansion, EIP-150 63/64 rule, EIP-2929 access sets)
- Comprehensive test suite (300+ tests)
- `#![no_std]` + `#![deny(unsafe_code)]` on all crates
