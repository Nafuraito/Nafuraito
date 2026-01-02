//! Nafuraito Kernel Entry Point
//! 
//! This is the main entry point for the Nafuraito operating system kernel.
//! It initializes the VGA display and halts the CPU.

#![crate_type = "staticlib"]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[path = "drivers/video/video.rs"]
mod video;
use video::{COLOR_PURPLE};

// / Panic handler - halts the CPU on panic
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}

/// Kernel entry point called from assembly/bootloader
/// 
/// Receives framebuffer info from bootloader via System V AMD64 ABI:
/// - rdi (fb_addr): Framebuffer physical address
/// - rsi (pitch): Bytes per scanline
/// - rdx (width): Screen width in pixels
/// - rcx (height): Screen height in pixels
/// - r8 (bpp): Bits per pixel
///
/// Initializes the VGA terminal and displays a welcome message,
/// then halts the CPU indefinitely.
#[no_mangle]
pub extern "C" fn enter(
    fb_addr: u32,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u32
) -> ! {
    unsafe {
        // Set up framebuffer with info from bootloader
        if fb_addr != 0 {
            video::set_framebuffer(fb_addr, pitch, width, height, bpp);
        }
        
        video::init();
        video::print("Welcome to Nafuraito! \0");
        video::print("A fully self-developed operating system from scratch. \0");
        video::print("!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~ \0");
    }

    // Halt the CPU permanently
    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}
