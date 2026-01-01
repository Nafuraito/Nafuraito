//! Nafuraito Kernel Entry Point
//! 
//! This is the main entry point for the Nafuraito operating system kernel.
//! It initializes the VGA display and halts the CPU.

#![crate_type = "staticlib"]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[path = "drivers/drivers.rs"]
mod drivers;
use drivers::video;

/// Panic handler - halts the CPU on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}

/// Kernel entry point called from assembly
/// 
/// Initializes the VGA terminal and displays a welcome message,
/// then halts the CPU indefinitely.
#[no_mangle]
pub extern "C" fn enter() -> ! {
    unsafe {
        video::init();
        video::print("Welcome to Nafuraito!\0");
        video::print("A fully self-developed operating system from scratch.\0");
    }

    // Halt the CPU permanently
    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}
