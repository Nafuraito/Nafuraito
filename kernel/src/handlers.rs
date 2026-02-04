//! Global panic handler for Nafuraito kernel

use core::panic::PanicInfo;

/// Panic handler - halts the CPU on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // TODO: Print panic info to screen when video module is available
    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}
