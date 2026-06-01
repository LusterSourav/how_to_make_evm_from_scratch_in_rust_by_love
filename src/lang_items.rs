// Only compiled when the `runtime` feature is off. With `runtime` on,
// `std` is linked and provides its own panic handler; if we also
// defined one here, we'd get a duplicate-lang-item error at link time.
#![cfg(not(feature = "runtime"))]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
