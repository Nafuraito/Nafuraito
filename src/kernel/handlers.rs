#![no_std]
#![no_main]
#![allow(unused)]

use core::panic::PanicInfo;

mod handlers {
    // / Panic handler - halts the CPU on panic
    #[panic_handler]
    pub fn panic(_info: &PanicInfo) -> ! {
        loop {
            unsafe {
                core::arch::asm!("cli");
                core::arch::asm!("hlt");
            }
        }
    }
}

