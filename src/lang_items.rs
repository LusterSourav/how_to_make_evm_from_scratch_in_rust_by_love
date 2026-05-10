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
