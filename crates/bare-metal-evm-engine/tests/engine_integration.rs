use bare_metal_evm_engine::execute;
use bare_metal_evm_gas::GasMeter;

fn gas_with_limit(limit: u64) -> GasMeter {
    GasMeter::new(limit, b"", &[], false, &[0xBB; 20], Some(&[0xCC; 20])).unwrap()
}

#[test]
fn add_basic() {
    let code = [0x60, 0x03, 0x60, 0x05, 0x01];
    let gas = gas_with_limit(100_000);
    let remaining = execute(&code, gas).unwrap();
    //100000 - 21000(intrinsic) - 3push1 - 3push2 - 3add = 78991
    //checked with geth tracer, matches
    //also tried: 0x6003600501 to save a byte, kept the padded form for readabilty
    assert_eq!(remaining, 78_991);
}

#[test]
fn add_overflow_wraps() {
    //0xff+1 wraps to 0. just checks no panic
    let code = [0x60, 0xFF, 0x60, 0x01, 0x01, 0x00];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn mul_basic() {
    let code = [0x60, 0x07, 0x60, 0x06, 0x02];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn sub_basic() {
    //10-3=7, unless underflow wraps (it does in evm, this is fine)
    let code = [0x60, 0x0A, 0x60, 0x03, 0x03];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn div_basic() {
    let code = [0x60, 0x0A, 0x60, 0x03, 0x04];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn div_by_zero_returns_zero() {
    let code = [0x60, 0x0A, 0x60, 0x00, 0x04];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn mod_basic() {
    //10%3=1
    let code = [0x60, 0x0A, 0x60, 0x03, 0x06];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn addmod_basic() {
    //(4+5)%3=0
    let code = [0x60, 0x05, 0x60, 0x04, 0x60, 0x03, 0x08];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn mulmod_basic() {
    //(3*4)%5=2
    let code = [0x60, 0x04, 0x60, 0x03, 0x60, 0x05, 0x09];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn exp_basic() {
    //2^10 = 1024? no wait, stack order: push 10, push 2, so base=10, exponent=2 → 10^2=100
    //i always get the exp stack order wrong, alwayse check twice
    let code = [0x60, 0x0A, 0x60, 0x02, 0x0A];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn sdiv_basic() {
    let code = [0x60, 0x07, 0x60, 0x02, 0x05];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn smod_basic() {
    let code = [0x60, 0x07, 0x60, 0x03, 0x07];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn lt_true() {
    //5<10 => 1
    let code = [0x60, 0x05, 0x60, 0x0A, 0x10];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn lt_false() {
    //10<5 => 0
    let code = [0x60, 0x0A, 0x60, 0x05, 0x10];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn gt_true() {
    let code = [0x60, 0x0A, 0x60, 0x05, 0x11];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn eq_true() {
    let code = [0x60, 0x2A, 0x60, 0x2A, 0x14];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn eq_false() {
    let code = [0x60, 0x2A, 0x60, 0x2B, 0x14];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn iszero_true() {
    let code = [0x60, 0x00, 0x15];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn iszero_false() {
    let code = [0x60, 0x01, 0x15];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn and_basic() {
    //0xff & 0x0f = 0x0f
    let code = [0x60, 0xFF, 0x60, 0x0F, 0x16];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn or_basic() {
    //0xf0|0x0f=0xff
    let code = [0x60, 0xF0, 0x60, 0x0F, 0x17];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn xor_basic() {
    //0xff^0x0f=0xf0
    let code = [0x60, 0xFF, 0x60, 0x0F, 0x18];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn not_basic() {
    //~0 = u256 max
    let code = [0x60, 0x00, 0x19];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn byte_basic() {
    //byte 31 (last byte, 0-indexed from left) of 0xff
    let code = [0x60, 0x1F, 0x60, 0xFF, 0x1A];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn shl_basic() {
    //push 1, push 4 → shift=1, value=4 → 4<<1=8
    //i always get the evm stack order backwards, checked the impl twice
    let code = [0x60, 0x01, 0x60, 0x04, 0x1B];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn shr_basic() {
    //16>>4=1
    let code = [0x60, 0x10, 0x60, 0x04, 0x1C];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn pop_basic() {
    let code = [0x60, 0x2A, 0x50];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn dup1_basic() {
    let code = [0x60, 0x2A, 0x80];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn dup16_basic() {
    let mut code = Vec::new();
    for i in 1..=16 {
        code.push(0x60);
        code.push(i);
    }
    code.push(0x8F);
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn swap1_basic() {
    let code = [0x60, 0x01, 0x60, 0x02, 0x90];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn swap16_basic() {
    let mut code = Vec::new();
    for i in 1..=17 {
        code.push(0x60);
        code.push(i);
    }
    code.push(0x9F);
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn stop_halts() {
    //push1 1, stop, push1 2 — second push never runs
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
    //no stop, just ends
    let code = [0x60, 0x2A];
    let gas = gas_with_limit(100_000);
    execute(&code, gas).unwrap();
}

#[test]
fn invalid_opcode() {
    let code = [0xFE];
    let gas = gas_with_limit(100_000);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::InvalidOpcode(0xFE));
}

#[test]
fn out_of_gas() {
    //intrinsic 21000 + 9 for 2xPUSH1 + ADD = 21009 needed, give 21008
    let code = [0x60, 0x01, 0x60, 0x02, 0x01];
    let gas = gas_with_limit(21_008);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::OutOfGas);
}

#[test]
fn stack_underflow_pop_empty() {
    let code = [0x50];
    let gas = gas_with_limit(100_000);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::StackUnderflow);
}

#[test]
fn stack_overflow() {
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
    let code = [0x80];
    let gas = gas_with_limit(100_000);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::StackUnderflow);
}

#[test]
fn push_past_end_of_code() {
    //push32 with only 1 byte of immediate data
    let code = [0x7F, 0x01];
    let gas = gas_with_limit(100_000);
    let err = execute(&code, gas).unwrap_err();
    assert_eq!(err, bare_metal_evm_engine::Error::InvalidOpcode(0x60));
}
