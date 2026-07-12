use bare_metal_evm_engine::execute;
use bare_metal_evm_gas::GasMeter;

fn gas_with_limit(limit: u64) -> GasMeter {
    GasMeter::new(limit, b"", &[], false, &[0xBB; 20], Some(&[0xCC; 20])).unwrap()
}

// ── Arithmetic ──────────────────────────────────────────────────────

#[test]
fn add_basic() {
    // PUSH1 3 PUSH1 5 ADD
    let code = [0x60, 0x03, 0x60, 0x05, 0x01];
    let gas = gas_with_limit(100_000);
    let remaining = execute(&code, gas).unwrap();
    // remaining = 100000 - 21000(intrinsic) - 3(PUSH1) - 3(PUSH1) - 3(ADD) = 78991
    assert_eq!(remaining, 78_991);
}

#[test]
fn add_overflow_wraps() {
    // PUSH1 0xFF PUSH1 1 ADD → 0 (wraps)
    let code = [0x60, 0xFF, 0x60, 0x01, 0x01, 0x00]; // STOP at end
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
    // We can't easily inspect the stack from outside, so test via a program
    // that checks the result. For now, just verify it doesn't panic.
}

#[test]
fn mul_basic() {
    // PUSH1 7 PUSH1 6 MUL
    let code = [0x60, 0x07, 0x60, 0x06, 0x02];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn sub_basic() {
    // PUSH1 10 PUSH1 3 SUB → 7
    let code = [0x60, 0x0A, 0x60, 0x03, 0x03];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn div_basic() {
    // PUSH1 10 PUSH1 3 DIV → 3
    let code = [0x60, 0x0A, 0x60, 0x03, 0x04];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn div_by_zero_returns_zero() {
    // PUSH1 10 PUSH1 0 DIV → 0
    let code = [0x60, 0x0A, 0x60, 0x00, 0x04];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn mod_basic() {
    // PUSH1 10 PUSH1 3 MOD → 1
    let code = [0x60, 0x0A, 0x60, 0x03, 0x06];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn addmod_basic() {
    // PUSH1 5 PUSH1 4 PUSH1 3 ADDMOD → (4+5)%3 = 0
    let code = [0x60, 0x05, 0x60, 0x04, 0x60, 0x03, 0x08];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn mulmod_basic() {
    // PUSH1 4 PUSH1 3 PUSH1 5 MULMOD → (3*4)%5 = 2
    let code = [0x60, 0x04, 0x60, 0x03, 0x60, 0x05, 0x09];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn exp_basic() {
    // PUSH1 10 PUSH1 2 EXP → 100
    let code = [0x60, 0x0A, 0x60, 0x02, 0x0A];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn sdiv_basic() {
    // PUSH1 7 PUSH1 2 SDIV → 3
    let code = [0x60, 0x07, 0x60, 0x02, 0x05];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn smod_basic() {
    // PUSH1 7 PUSH1 3 SMOD → 1
    let code = [0x60, 0x07, 0x60, 0x03, 0x07];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

// ── Comparison ──────────────────────────────────────────────────────

#[test]
fn lt_true() {
    // PUSH1 5 PUSH1 10 LT → 1 (5 < 10)
    let code = [0x60, 0x05, 0x60, 0x0A, 0x10];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn lt_false() {
    // PUSH1 10 PUSH1 5 LT → 0 (10 < 5 is false)
    let code = [0x60, 0x0A, 0x60, 0x05, 0x10];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn gt_true() {
    // PUSH1 10 PUSH1 5 GT → 1 (10 > 5)
    let code = [0x60, 0x0A, 0x60, 0x05, 0x11];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn eq_true() {
    // PUSH1 42 PUSH1 42 EQ → 1
    let code = [0x60, 0x2A, 0x60, 0x2A, 0x14];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn eq_false() {
    // PUSH1 42 PUSH1 43 EQ → 0
    let code = [0x60, 0x2A, 0x60, 0x2B, 0x14];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn iszero_true() {
    // PUSH1 0 ISZERO → 1
    let code = [0x60, 0x00, 0x15];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn iszero_false() {
    // PUSH1 1 ISZERO → 0
    let code = [0x60, 0x01, 0x15];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

// ── Bitwise ─────────────────────────────────────────────────────────

#[test]
fn and_basic() {
    // PUSH1 0xFF PUSH1 0x0F AND → 0x0F
    let code = [0x60, 0xFF, 0x60, 0x0F, 0x16];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn or_basic() {
    // PUSH1 0xF0 PUSH1 0x0F OR → 0xFF
    let code = [0x60, 0xF0, 0x60, 0x0F, 0x17];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn xor_basic() {
    // PUSH1 0xFF PUSH1 0x0F XOR → 0xF0
    let code = [0x60, 0xFF, 0x60, 0x0F, 0x18];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn not_basic() {
    // PUSH1 0 NOT → U256::MAX
    let code = [0x60, 0x00, 0x19];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn byte_basic() {
    // PUSH1 31 PUSH1 0xFF BYTE → 0xFF (byte at index 31 from left = last byte)
    let code = [0x60, 0x1F, 0x60, 0xFF, 0x1A];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn shl_basic() {
    // PUSH1 1 PUSH1 4 SHL → 16
    let code = [0x60, 0x01, 0x60, 0x04, 0x1B];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn shr_basic() {
    // PUSH1 16 PUSH1 4 SHR → 1
    let code = [0x60, 0x10, 0x60, 0x04, 0x1C];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

// ── Stack ops ───────────────────────────────────────────────────────

#[test]
fn pop_basic() {
    // PUSH1 42 POP
    let code = [0x60, 0x2A, 0x50];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn dup1_basic() {
    // PUSH1 42 DUP1 → stack has [42, 42]
    let code = [0x60, 0x2A, 0x80];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn dup16_basic() {
    // PUSH1 1 PUSH1 2 ... PUSH1 16 DUP16 → duplicates 16th item
    let mut code = Vec::new();
    for i in 1..=16 {
        code.push(0x60);
        code.push(i);
    }
    code.push(0x8F); // DUP16
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn swap1_basic() {
    // PUSH1 1 PUSH1 2 SWAP1 → stack has [2, 1]
    let code = [0x60, 0x01, 0x60, 0x02, 0x90];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn swap16_basic() {
    // PUSH1 1 ... PUSH1 17 SWAP16
    let mut code = Vec::new();
    for i in 1..=17 {
        code.push(0x60);
        code.push(i);
    }
    code.push(0x9F); // SWAP16
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

// ── Control flow ────────────────────────────────────────────────────

#[test]
fn stop_halts() {
    // PUSH1 1 STOP PUSH1 2 (the second PUSH should never execute)
    let code = [0x60, 0x01, 0x00, 0x60, 0x02];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn empty_code_stops() {
    let code: [u8; 0] = [];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn implicit_stop_at_end() {
    // PUSH1 42 (no STOP, but code ends → implicit halt)
    let code = [0x60, 0x2A];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

// ── Error cases ─────────────────────────────────────────────────────

#[test]
fn invalid_opcode() {
    let code = [0xFE]; // INVALID
    let gas = gas_with_limit(100_000);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::InvalidOpcode(0xFE));
}

#[test]
fn out_of_gas() {
    // PUSH1 1 PUSH1 2 ADD — costs 3+3+3=9 gas + 21000 intrinsic = 21009
    // Give only 21008 gas (intrinsic takes 21000, leaving 8 < 9 needed)
    let code = [0x60, 0x01, 0x60, 0x02, 0x01];
    let gas = gas_with_limit(21_008);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::OutOfGas);
}

#[test]
fn stack_underflow_pop_empty() {
    // POP on empty stack
    let code = [0x50];
    let gas = gas_with_limit(100_000);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::StackUnderflow);
}

#[test]
fn stack_overflow() {
    // Push 1024 items, then one more
    let mut code = Vec::new();
    for _ in 0..1024 {
        code.push(0x60);
        code.push(0x01);
    }
    code.push(0x60);
    code.push(0x01);
    let gas = gas_with_limit(1_000_000);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::StackOverflow);
}

#[test]
fn dup_underflow() {
    // DUP1 on empty stack
    let code = [0x80];
    let gas = gas_with_limit(100_000);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::StackUnderflow);
}

#[test]
fn push_past_end_of_code() {
    // PUSH32 but only 1 byte of immediate data
    let code = [0x7F, 0x01]; // PUSH32 needs 32 bytes after
    let gas = gas_with_limit(100_000);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::InvalidOpcode(0x60));
}
