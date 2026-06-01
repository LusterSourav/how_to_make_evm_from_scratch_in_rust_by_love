# EVM from Scratch — A Zero-Dependency Implementation in Rust

> *No crates. No standard library. Just Rust, mathematics, and the Yellow Paper.*

[![License: CC-BY-SA-4.0](https://img.shields.io/badge/License-CC--BY--SA--4.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-blue.svg)](https://www.rust-lang.org)
[![no_std](https://img.shields.io/badge/no__std-supported-green.svg)](https://docs.rust-embedded.org/book/intro/no-std.html)
[![zero deps](https://img.shields.io/badge/runtime_dependencies-zero-success.svg)]()

## Status

| Component | Status |
|---|---|
| **Layer 0 — Primitive Arithmetic** (`U256`, `U512`, Knuth division) | Shipped |
| **Layer 1 — Serialization & Sponge** (RLP, HP nibbles, Keccak-256) | Shipped |
| **Layer 2 — State Management** (MPT, account model, world state, journal) | Shipped |
| **Layer 3 — Resource Metering** (gas, access lists, 63/64 rule) | Planned |
| **Layer 4 — Engine Core** (dispatch table, jump map, Z-function, opcodes) | Planned |
| **Layer 5 — Compliance & Precompiles** (Secp256k1, MODEXP, EOF, EIP-1153) | Planned |

Shipped = implemented and tested. Planned = documented, not built yet.

---

## What This Is

A from-scratch Ethereum Virtual Machine built in Rust. No `ethers`, no `primitive-types`, no `sha3`, no `secp256k1`. Not even `std` by default.

The `#[no_std]` constraint is deliberate. It forces every primitive to be intentional: panic handlers, memory layout, arithmetic overflow -- nothing is assumed, nothing is borrowed. Every component is engineered directly against the [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf).

The goal is a Cancun-era compliant execution engine where every byte of behavior is traceable back to a formal specification.

---

## Quick Start

```bash
# Clone
git clone https://github.com/LusterSourav/how_to_make_evm_from_scratch_in_rust_by_love
cd how_to_make_evm_from_scratch_in_rust_by_love

# Run the test suite (300+ tests across 6 crates)
cargo test --workspace

# Run an example
cargo run --example basic_state
cargo run --example rlp_encode_decode

# Run with the optional `runtime` feature (std-backed, for host-side testing)
cargo test --workspace --features runtime
```

The library is **no_std + zero-dependency** at the workspace level. The `runtime` feature is a small no-op flag kept for documentation purposes — the workspace compiles cleanly on stable Rust (1.85+). For actual bare-metal deployment (`aarch64-unknown-none`, `riscv64imac-unknown-none-elf`, etc.) you will need a nightly toolchain with `-Z build-std=core,alloc`.

---

## Crate Layout

Six focused crates, each with a single responsibility. All are `#![no_std]` + `#![deny(unsafe_code)]` + zero external runtime dependencies.

| Crate | Role |
|---|---|
| `bare-metal-evm-types` | 256-bit (`U256`) and 512-bit (`U512`) unsigned integers, all arithmetic, division (Knuth D), byte conversions. |
| `bare-metal-evm-keccak` | Bit-level Keccak-256 (24-round sponge, correct Ethereum padding). |
| `bare-metal-evm-rlp` | Recursive Length Prefix encoder/decoder with strict minimalism. |
| `bare-metal-evm-nibble` | Hex-prefix nibble paths, `u4` abstraction, packed nibble buffers. |
| `bare-metal-evm-trie` | Modified Merkle Patricia Trie: 4 node types, hexary traversal, inline optimization, depth-bounded recursion. |
| `bare-metal-evm-state` | Account model, `WorldState` (caches + journal + commit), EIP-158 empty-account pruning, code storage. |

The workspace root crate (`bare-metal-evm`) re-exports everything for convenience.

---

## v0.2.0 — World State (MPT + Account Model)

> Released 2026-06-01. Closes [#10](https://github.com/LusterSourav/how_to_make_evm_from_scratch_in_rust_by_love/issues/10), [PR #15](https://github.com/LusterSourav/how_to_make_evm_from_scratch_in_rust_by_love/pull/15).

Six crates that build Ethereum's World State (σ): an address-keyed MPT
that commits to nonce, balance, code, and per-account storage, with a
journal for checkpoint/rollback.

### Crates in this release

- `bare-metal-evm-types`  — U256, U512
- `bare-metal-evm-keccak` — Keccak-256
- `bare-metal-evm-nibble` — u4 nibbles + HP encoding
- `bare-metal-evm-rlp`    — RLP encode/decode
- `bare-metal-evm-trie`   — Merkle Patricia Trie
- `bare-metal-evm-state`  — WorldState, Account, Journal

All `#![no_std]`, `#![deny(unsafe_code)]`, zero external deps.

### Stats

| | |
|---|---|
| Tests | 300 |
| Lines (src, ex. lib.rs) | 7,191 |
| Crates | 6 |

`cargo test --workspace` and `cargo clippy -- -D warnings` both clean.

### What's new since 0.1.0

- MPT with HP encoding and inline-node optimization
- WorldState with cache-then-commit and EIP-158 pruning
- Journal/rollback, depth-capped at 4096
- Trie root preserved on DB error
- `set_code(addr, code)` now hashes for you
- Typed `NibbleError` replaces `Result<(), ()>` in nibble ops
- Keccak-256 XOR block bounds: `debug_assert` → `assert`
- Removed dead public API (`drain`, `split_first`, `split_at`)

### Install

```toml
[dependencies]
bare-metal-evm-types  = "0.2.0"
bare-metal-evm-keccak = "0.2.0"
bare-metal-evm-nibble = "0.2.0"
bare-metal-evm-rlp    = "0.2.0"
bare-metal-evm-trie   = "0.2.0"
bare-metal-evm-state  = "0.2.0"
```

### OCI (ghcr.io)

```bash
docker pull ghcr.io/lustersourav/how_to_make_evm_from_scratch_in_rust_by_love/bare-metal-evm-types:0.2.0
# ... 5 more, one per crate
```

The full release notes also live on the [GitHub release page](https://github.com/LusterSourav/how_to_make_evm_from_scratch_in_rust_by_love/releases/tag/v0.2.0).

---

# The Physics (Implemented)

> *These are the parts of the machine already written, tested, and shipped.*

## 1. The 256-Bit Arithmetic Engine

Rust tops out at `u128` natively. The EVM's word size is 256 bits. So the first order of business is building the type the language doesn't give you.

```rust
pub struct U256(pub [u64; 4]);
```

Four 64-bit limbs, little-endian. Simple to describe, demanding to implement correctly. This isn't just a data structure choice — it's a deliberate mapping to how modern 64-bit CPUs actually process wide integers.

### Addition & Subtraction — Manual Carry Propagation

Carry and borrow bits propagate manually across limb boundaries using `overflowing_add` and `overflowing_sub`. The machine stays perfectly deterministic because every overflow is explicitly detected and forwarded — nothing is left to undefined behavior or platform-specific wrapping rules. Two's complement handles all signed operations (`SDIV`, `SMOD`, `SAR`) without needing a separate signed type.

### Multiplication — The `u128` Compiler Hint

Each `u64 × u64` product is computed as a `u128` intermediate. This isn't just a convenience — it's a deliberate hint to LLVM. By expressing the logic as `u64 → u128` widening multiplication, you guide the compiler to map it directly to the native `MULX` instruction on x86_64, achieving assembly-level performance without writing a single line of unsafe inline assembly.

**Schoolbook vs. Karatsuba:** At exactly 256 bits, schoolbook (brute-force) multiplication is faster than Karatsuba. Karatsuba reduces multiplications from O(n²) to O(n^1.585), but at this scale the overhead of the extra additions and subtractions outweighs the savings. Karatsuba pays off at much larger widths — for 256-bit × 256-bit, schoolbook with `u128` partial products is the right call.

The `MULMOD` opcode requires 512-bit intermediate products. Those are handled by extending the same limb-widening logic across eight 64-bit lanes before reducing modulo the divisor — the same code path is exposed as `U256::mul_full(self, rhs) -> U512`.

### Division — Knuth's Algorithm D

Division is where things get serious. The implementation follows **Knuth's Algorithm D** from *The Art of Computer Programming, Vol. 2*. It runs in three stages:

1. **Normalization** — the divisor is left-shifted until its most significant bit is set. This makes quotient digit estimation reliable by ensuring the leading divisor limb is large enough to bound the estimate error.
2. **Estimation & Refinement** — a quotient digit `q̂` is estimated from the two most significant limbs of the current dividend. The estimate can overshoot by at most two, so a correction loop brings it back into range.
3. **Unnormalization** — the remainder is right-shifted back to its original scale, undoing the normalization from step one.

Every step has a `#[assert]` (not `debug_assert!`) guarding the boundary condition, so a regression in release mode fails loudly rather than silently producing wrong results.

## 2. The `no_std` Environment — Bare Metal Glue

Stripping `std` means manually providing the things Rust normally gets for free. This is the "glue" layer that makes everything else possible.

### Lang Items

The compiler requires certain symbols to exist at link time. Without `std`, you provide them yourself:

```rust
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}  // deterministic halt — no unwinding, no stack trace, no formatting
}
```

For actual bare-metal deployment (`aarch64-unknown-none`, etc.), `build-std` is required and the `alloc_error_handler` lang item is provided by the `alloc` crate's default. The `src/lang_items.rs` file is gated on `not(feature = "runtime")` so it doesn't conflict with `std`'s panic handler when running tests on a host.

### Deterministic Memory

There is no OS heap unless you explicitly bring one. All buffers — the trie, journal, account caches — are either fixed-size arrays or manually managed `Vec`s. This guarantees absolute determinism: the same input always produces the same result, on any host architecture, with no dependency on OS memory layout or allocator behavior.

## 3. Cryptographic Primitives — Bit-Level Surgery

Ethereum's security model rests on two cryptographic foundations: a hash function and an elliptic curve. The hash is implemented here without any helper libraries.

### Keccak-256 — The Sponge Construction

Ethereum uses Keccak-256 — not the NIST SHA-3 standard. They differ in padding (`0x01` vs `0x06`), a detail that has burned people before. The implementation follows the **sponge construction**:

- **Absorb phase**: input is XOR'd into the 1600-bit state in rate-sized blocks (1088 bits for Keccak-256)
- **Squeeze phase**: output is extracted after each permutation round

The permutation runs **24 rounds**, each applying five step mappings to a 5×5 matrix of 64-bit lanes:

| Step | Role |
|------|------|
| **θ (Theta)** | Column parity mixing — diffuses bits across all columns |
| **ρ (Rho)** | Per-lane bit rotation — offsets are fixed constants from the spec |
| **π (Pi)** | Lane permutation — rearranges the 5×5 matrix positions |
| **χ (Chi)** | Non-linear row mixing — the only non-linear step, provides security |
| **ι (Iota)** | Round constant injection — breaks symmetry each round |

Every rotation offset and round constant is hardcoded from the [Keccak Reference v3.0](https://keccak.team/files/Keccak-reference-3.0.pdf). No shortcuts, no lookup tables borrowed from elsewhere.

> [FIPS 202](https://csrc.nist.gov/publications/detail/fips/202/final) documents the SHA-3 standard. Ethereum's Keccak-256 predates it and uses a different padding rule. Using the wrong padding produces a completely different hash and silently breaks everything downstream.

## 4. State and Serialization

### Recursive Length Prefix (RLP)

RLP is Ethereum's universal serialization format — transactions, blocks, trie nodes, account state, all of it. The encoding rules map to five prefix ranges:

| Range | Meaning |
|-------|---------|
| `0x00–0x7f` | Single byte — value is its own encoding |
| `0x80–0xb7` | Short string (0–55 bytes) — length encoded in prefix |
| `0xb8–0xbf` | Long string — length-of-length encoded in prefix |
| `0xc0–0xf7` | Short list — total payload length in prefix |
| `0xf8–0xff` | Long list — length-of-length in prefix |

**Strict RLP minimalism is enforced.** Integers must be big-endian with no leading zero bytes — `0x0100` must be encoded as two bytes, not three. This rule is checked on decode and rejected if violated. Getting this wrong produces different trie roots and breaks consensus.

### Modified Merkle Patricia Trie (MPT)

The world state, account storage, transaction list, and receipt list all live in MPTs. It's a hexary trie combining Merkle integrity (every node identified by the hash of its RLP encoding) with Patricia compression (shared prefixes collapsed into extension nodes).

#### The Virtual `u4` Nibble System

Rust operates on `u8` bytes, but the MPT is hexary — keys are traversed 4 bits at a time. A **virtual `u4` iterator** splits each 32-byte hash into 64 nibbles, yielding one nibble per step. This iterator drives every trie insert, lookup, and delete operation. Without it, the path logic becomes a mess of bit-shifting scattered across the codebase.

#### Four Node Types

| Node | Structure | Purpose |
|------|-----------|---------|
| **NULL** | — | Empty / base case |
| **Branch** | 17 items | 16 child slots (one per hex nibble) + optional value |
| **Extension** | 2 items | Shared prefix compression |
| **Leaf** | 2 items | Terminal node holding a value |

#### Node Splitting Surgery

When two keys share a prefix but diverge at a specific nibble, a Leaf must be surgically converted into an Extension + Branch:

1. Compute the shared prefix length between the existing key and the new key
2. Create a new Branch node at the divergence point
3. The existing Leaf becomes a child of the Branch at its diverging nibble
4. The new value becomes a child at its diverging nibble
5. If the shared prefix is non-empty, wrap the Branch in an Extension node

This is the most structurally complex operation in the trie — easy to get subtly wrong, hard to debug when you do.

#### Hex-Prefix (HP) Encoding

HP encoding resolves the ambiguity between leaf and extension nodes at the nibble level using four prefix flags:

| Prefix | Meaning |
|--------|---------|
| `0x00` | Extension, even-length path |
| `0x1_` | Extension, odd-length path (nibble in low bits) |
| `0x20` | Leaf, even-length path |
| `0x3_` | Leaf, odd-length path (nibble in low bits) |

#### Inline Optimization

Nodes whose RLP encoding is less than 32 bytes are embedded directly into their parent rather than stored by hash reference. A node that's only a few bytes doesn't need a 32-byte Keccak pointer — inlining it keeps small tries compact and avoids unnecessary hashing overhead.

#### Depth-Bounded Recursion

`insert` and `remove` recurse with a `depth: usize` counter capped at `MAX_DEPTH = 64` (matching a 32-byte 256-bit key, the largest key the trie can store). An attacker-controlled deeply-nested trie produces `Error::MaxDepth` instead of a stack overflow.

## 5. World State

The `WorldState` is the top-level container for an account-model state:

- **Account cache** (`HashMap<[u8;20], Option<Account>>`) — pending writes, `None` for deletions.
- **Storage cache** (`HashMap<([u8;20], U256), U256>`) — pending storage writes, keyed by `(address, slot)`.
- **Code cache** (`HashMap<[u8;32], Vec<u8>>`) — contract bytecode, keyed by Keccak-256 hash.
- **State trie** + per-account **storage tries** (MPT instances).
- **Journal** — append-only log of mutations for `checkpoint` / `revert`.

### Commit Pipeline

`commit()` is decomposed into five sub-functions, each responsible for one phase:

1. `commit_storage` — flush pending storage writes to per-account storage tries.
2. `commit_prune_deleted_storage` — remove storage tries of deleted accounts.
3. `commit_accounts` — apply EIP-158 empty-account pruning when writing the state trie.
4. `commit_code` — persist any newly-seen contract code to the database.
5. `commit_clear_caches` — clear in-memory caches and set the new state root.

This makes each phase independently testable and keeps the main `commit()` readable.

### Journal and Checkpoints

Nested calls (`CALL`, `DELEGATECALL`, `STATICCALL`) create sub-contexts that may revert. When a sub-context reverts, its storage writes and balance changes must be undone — but the gas consumed by that sub-context is *not* returned to the caller.

This is handled with a **journaling system**:

1. Before entering a sub-context, record a **checkpoint** (the current journal length)
2. Every state mutation (storage write, balance change, account creation) appends an entry to the journal
3. On `REVERT`, replay the journal backwards from the current end to the checkpoint, undoing each mutation
4. On success, the journal entries since the checkpoint are simply discarded

The journal is bounded at `MAX_JOURNAL_DEPTH = 4096` — beyond that, `checkpoint()` returns `false` and the caller must commit or revert.

---

# The Machine (Planned)

> *These are the higher-level EVM behaviors that will sit on top of the physics. The spec is written, the algorithms are researched, the implementation hasn't started.*

## 6. Execution Engine

### Machine State

The EVM is a stack machine. The full execution state μ is a formal tuple:

```
μ = (g, pc, m, i, s, o)
```

| Symbol | Component | Description |
|--------|-----------|-------------|
| `g` | Gas | Remaining gas budget |
| `pc` | Program Counter | Index into current bytecode |
| `m` | Memory | Byte-addressable volatile buffer |
| `i` | Active Words | Memory size in 32-byte words (for gas calculation) |
| `s` | Stack | LIFO, 256-bit words, max depth 1024 |
| `o` | Return Data | Output buffer from the last sub-call |

### The Execution Environment — The I Tuple

The machine state μ describes what's happening *inside* the current execution context. But the EVM also needs to know about the *world outside* — the block it's running in, who called it, what address owns the code. That external context is injected through the **execution environment tuple I**:

| Symbol | Component | Description |
|--------|-----------|-------------|
| `Ia` | Code owner | Address of the account whose code is executing |
| `Io` | Original transactor | The EOA that originated the top-level transaction |
| `Ip` | Effective gas price | Wei per gas unit for this transaction |
| `Id` | Call data | Input data passed to the current context |
| `Is` | Caller | Address of the immediate caller |
| `Iv` | Value | Wei transferred with this call |
| `Ib` | Bytecode | The code being executed |
| `IH` | Block header | Contains coinbase, number, timestamp, difficulty, gas limit |
| `Ie` | Call depth | Current call stack depth |
| `Iw` | Write permission | `true` for normal calls, `false` inside `STATICCALL` |

The `Iw` flag is what enforces read-only semantics in static contexts. If `Iw` is `false` and the current opcode would modify state — `SSTORE`, `LOG*`, `CREATE`, `SELFDESTRUCT` — the Z-function triggers an exceptional halt immediately.

### The Fetch-Decode-Execute Loop

Each iteration:

1. **Fetch** the byte at `pc` from the current bytecode
2. **Decode** it as an opcode
3. **Verify** gas availability and stack preconditions
4. **Execute** the operation, updating μ
5. **Advance** `pc` — unless it's a jump

### Formal Exceptional Halting — The Z Function

The Yellow Paper defines a strict set of conditions under which execution must enter an **exceptional halting state** — consuming all remaining gas and discarding all state mutations from the current context. These aren't soft errors; they're hard stops. The machine checks all of the following before executing any instruction:

| Condition | Meaning |
|-----------|---------|
| `μ_g < C_cost` | Insufficient gas for the operation |
| `δ_w` is undefined | Invalid or unrecognized opcode |
| `\|μ_s\| < δ_w` | Stack underflow — not enough items for the opcode's inputs |
| `JUMP`/`JUMPI` target ∉ jump map | Destination is not a valid `JUMPDEST` |
| `\|μ_s\| - δ_w + α_w > 1024` | Stack overflow — result would exceed the 1024-item limit |
| State-modifying opcode in `STATICCALL` | Write attempted in a read-only context |

If any condition is true, execution halts immediately. No partial state is committed. The gas is gone.

## 7. Resource Metering

### Gas Calculus — Quadratic Memory

**Quadratic memory expansion** deters memory-based DoS attacks. The exact formula:

```
G_memory(words) = 3 · words + ⌊words² / 512⌋
```

Memory isn't allocated by the OS — it's metered by the VM. The quadratic term means a contract trying to allocate gigabytes will exhaust its gas budget long before it gets there.

### The 63/64 Rule (EIP-150)

A caller must reserve 1/64 of its remaining gas before forwarding the rest to a child call. This ensures the parent context always has enough gas to finalize — clean up, emit logs, write return data — even if the child reverts and consumes everything it was given.

While the formal stack depth limit is 1024 frames, the 63/64 rule imposes a much tighter *practical* limit. After roughly **340 nested calls**, the forwarded gas per frame drops below the minimum needed to do anything useful.

### Tiered State Access (EIP-2929)

A transaction-wide access list — a `Set` of touched addresses and storage slots. The first access to any address or slot is "cold":

| Access type | Cold cost | Warm cost |
|-------------|-----------|-----------|
| Account access | 2600 gas | 100 gas |
| Storage slot (`SLOAD`) | 2100 gas | 100 gas |

## 8. Interpreter Optimizations

### Function Pointer Dispatch Table

Rather than a `match` statement over 256 possible opcodes, dispatch uses a **function pointer table**:

```rust
type Handler = fn(&mut Vm) -> Result<(), Error>;
static DISPATCH: [Handler; 256] = [ /* one entry per opcode byte */ ];
```

Indexed directly by the opcode byte, this gives O(1) dispatch and plays nicely with branch predictors.

### Hot Opcode Inlining

`PUSH`, `DUP`, `SWAP`, and `JUMP` account for a disproportionate share of executed instructions in real contract bytecode. Inlining these directly into the execution loop — bypassing the function pointer table entirely for the common case — reduces call overhead and can yield a **30–40% throughput improvement** on hot paths.

### Computed Gotos & Tail Calls

Eliminating the central dispatch bottleneck for these pairs lets the branch predictor work with the execution pattern rather than against it.

## 9. Static Jump Map Analysis

Before execution begins, the bytecode is pre-scanned to build a **jump map** — a bitset marking every valid `JUMPDEST` (`0x5B`) location. The critical detail: bytes that appear as `0x5B` inside `PUSH` data are *not* valid jump destinations.

```
PUSH2 0x5B 0x5B   ← these two 0x5B bytes are PUSH data, not JUMPDEST
JUMPDEST          ← this one is a valid jump target
```

The jump map **must be statically generated before execution begins** — checking at runtime on every jump is not sufficient. Without it, a contract could jump into the middle of `PUSH` data and execute arbitrary byte sequences as opcodes.

## 10. Secp256k1 — Finite Field Arithmetic

The `ECRECOVER` precompile — used by virtually every Ethereum transaction — requires real elliptic curve arithmetic over the finite field GF(p). The field prime is:

```
p = 2²⁵⁶ - 2³² - 977
```

That specific form makes modular reduction efficient. The curve equation is the Short Weierstrass form:

```
y² = x³ + 7 (mod p)
```

Coverage will include modular inverse via the Extended Euclidean Algorithm, point addition/doubling, scalar multiplication via double-and-add, and square roots mod p using `y = x^((p+1)/4) mod p` (valid because `p ≡ 3 (mod 4)`).

## 11. Log Processing and Bloom Filters

Contract events (`LOG0`–`LOG4`) produce structured log entries committed to the transaction receipt. **Bloom filter construction** allows efficient log searching without scanning the full chain. For every address and topic in a log entry, exactly **3 bits** are set in a 2048-bit filter. The bit positions are derived deterministically from the low-order 11 bits of each of the first three pairs of bytes of the Keccak-256 hash.

## 12. Modern EIP Support — Cancun and Beyond

### Transient Storage — EIP-1153

`TSTORE` and `TLOAD` operate on storage that exists only for the duration of a transaction — initialized to zero at the start, discarded at the end. The clean solution for re-entrancy guards in DeFi.

### EVM Object Format — EIP-3540 / EIP-3541

EOF separates code from data at the container level, with a structured header declaring code sections, data sections, and type information. EIP-3541 rejects new contracts whose bytecode starts with `0xEF`, reserving that byte for the EOF container format.

## 13. Future Research — Parallel Execution

**Optimistic Concurrency Control (OCC)** — execute independent transactions in parallel, speculatively assuming no conflicts. After execution, check for state conflicts; re-execute only the conflicting transactions serially.

**Lazy updating** — gas payments to the block beneficiary account are deferred to the end of the block rather than applied after each transaction, removing one of the most common sources of false conflicts.

**Pipelined Merkleization** — overlap transaction execution with the rehashing of the Merkle Patricia Trie, hiding Merkleization latency behind execution latency.

**Speculative pre-warming** — transactions are speculatively executed in parallel to *predict* which storage slots and accounts will be accessed, loading them into the warm access set before official execution begins.

---

## The 100% Correctness Rule

Because this is a bare metal, consensus-critical implementation, there is no "mostly correct." A few specific invariants that must hold without exception:

- **Strict RLP minimalism** — integers must be big-endian with no leading zeros. A single extra zero byte produces a different trie root and breaks consensus with every other node on the network.
- **Jump map safety** — the jump map must be statically generated before execution begins. A `0x5B` byte inside `PUSH` data is not a valid jump destination.
- **Exceptional halting** — all six Z-function conditions must be checked before every instruction.
- **`no_std` lang items** — a custom `panic_handler` must be provided for bare-metal targets.
- **Gas accounting** — every opcode, every memory expansion, every state access must charge exactly the gas specified by the Yellow Paper and the relevant EIPs.

---

## Implementation Roadmap

Built layer by layer, each depending only on what's below it:

- [x] **Layer 0 — Primitive Arithmetic**: `U256`/`U512`, schoolbook multiplication with `u128` limb-widening (`MULX` hint), Knuth's Algorithm D division, two's complement, `no_std` panic handler
- [x] **Layer 1 — Serialization & Sponge**: Recursive RLP encoder/decoder (five prefix ranges, strict minimalism), HP nibble encoding (four prefix flags), bitwise Keccak-256 (24-round sponge permutation, correct Ethereum padding)
- [x] **Layer 2 — State Management**: Modified MPT (virtual `u4` nibble iterator, four node types, node splitting surgery, inline optimization, depth-bounded recursion), account model, world state σ, state journaling and checkpoint/rollback
- [ ] **Layer 3 — Resource Metering**: Quadratic memory gas formula, EIP-2929 warm/cold access sets, EIP-150 63/64 rule for sub-call gas forwarding
- [ ] **Layer 4 — Engine Core**: Function pointer dispatch table, static jump map pre-scanner, Z-function exceptional halting checks, hot opcode inlining (`PUSH`/`DUP`/`SWAP`/`JUMP`), fetch-decode-execute loop, log processing and bloom filter construction
- [ ] **Layer 5 — Compliance & Precompiles**: All ~144 opcodes, `ECRECOVER` (Secp256k1 point arithmetic over GF(p)), `SHA256`, `RIPEMD160`, `IDENTITY`, `MODEXP`, BN256 operations, transient storage (`TSTORE`/`TLOAD`), EOF container validation (EIP-3540/3541), typed transactions (EIP-2718)

---

## References

- [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf) — the formal specification this entire project is built against
- [FIPS 202](https://csrc.nist.gov/publications/detail/fips/202/final) — SHA-3 / Keccak sponge standard (note the padding difference from Ethereum's Keccak-256)
- Knuth, D.E. — *The Art of Computer Programming, Vol. 2: Seminumerical Algorithms* — Algorithm D (multi-precision division)
- [The Keccak Reference v3.0](https://keccak.team/files/Keccak-reference-3.0.pdf) — rotation offsets, round constants, sponge parameters
- [EIP-150](https://eips.ethereum.org/EIPS/eip-150) — gas cost changes for IO-heavy operations; the 63/64 rule for sub-call gas forwarding
- [EIP-2718](https://eips.ethereum.org/EIPS/eip-2718) — typed transaction envelope
- [EIP-2929](https://eips.ethereum.org/EIPS/eip-2929) — access lists and warm/cold storage pricing
- [EIP-1153](https://eips.ethereum.org/EIPS/eip-1153) — transient storage (`TLOAD` / `TSTORE`)
- [EIP-3540](https://eips.ethereum.org/EIPS/eip-3540) — EVM Object Format (EOF) v1
- [EIP-3541](https://eips.ethereum.org/EIPS/eip-3541) — reject new contracts starting with `0xEF`
- [EIP-161](https://eips.ethereum.org/EIPS/eip-161) — state trie clearing; defines when an account is considered "empty" and eligible for deletion

---

*Built with patience, stubbornness, and an unreasonable fondness for bit manipulation.*
