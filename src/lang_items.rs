// Bare Metal Environment — Runtime Glue
// =================================================
// Provides `#[panic_handler]` and `#[alloc_error_handler]` for bare-metal
// targets that do not have a standard library runtime.
//
// Enable with: `cargo build --features runtime`
//
// These items should only be used when this crate is compiled as the
// root binary for a bare-metal target (e.g., arm-unknown-none,
// riscv64gc-unknown-none-elf).  When used as a library in a hosted
// environment, omit the `runtime` feature.

extern crate alloc;

use core::alloc::Layout;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    loop {}
}