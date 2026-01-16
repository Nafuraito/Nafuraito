//! Nafuraito Kernel Entry Point
//! 
//! This is the main entry point for the Nafuraito operating system kernel.
//! It initializes the VGA display, memory management, GDT/TSS, and halts the CPU.

#![crate_type = "staticlib"]
#![no_std]
#![no_main]
#![allow(unused)]

use core::panic::PanicInfo;

/// Panic handler for the kernel
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}

#[path = "drivers/video/video.rs"]
mod video;
use video::COLOR_PURPLE;
use core::fmt::Write;

#[path = "memory/mod.rs"]
mod memory;

#[path = "gdt/mod.rs"]
mod gdt;

#[path = "idt/mod.rs"]
mod idt;

#[path = "drivers/pic/pic.rs"]
mod pic;

/// Kernel entry point called from assembly/bootloader
/// 
/// Receives framebuffer info and memory map from bootloader via System V AMD64 ABI:
/// - rdi (fb_addr): Framebuffer physical address
/// - rsi (pitch): Bytes per scanline
/// - rdx (width): Screen width in pixels
/// - rcx (height): Screen height in pixels
/// - r8 (bpp): Bits per pixel
/// - r9 (memory_map_addr): E820 memory map address
///
/// Initializes the VGA terminal, parses memory map, and displays system info,
/// then halts the CPU indefinitely.
#[no_mangle]
pub extern "C" fn enter(
    fb_addr: u32,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u32,
    memory_map_addr: u64
) -> ! {
    unsafe {
        // Set up framebuffer with info from bootloader
        if fb_addr != 0 {
            video::set_framebuffer(fb_addr, pitch, width, height, bpp);
        }
        
        video::init();
        video::print("Welcome to Nafuraito!\n\0");
        
        // Initialize GDT and TSS
        video::print("\nInitializing GDT and TSS... \0");
        gdt::init();
        video::print("OK\n\0");
        
        // Initialize IDT
        video::print("Initializing IDT... \0");
        idt::init();
        video::print("OK\n\0");

        // Init PIC
        video::print("Initializing PIC... \0");
        pic::remap(0x20, 0x28); // Inits PIC
        video::print("OK\n\0");

        // Enable Interrupts
        // After idt::init() and pic::init()
        unsafe {
            core::arch::asm!("sti");  // Enable interrupts
        }
        video::print("Interrupts enabled\n\0");
        

        // Print GDT info
        video::print("  Kernel Code Selector: 0x\0");
        print_hex_byte(gdt::KERNEL_CODE_SELECTOR as u8);
        video::print("\n  Kernel Data Selector: 0x\0");
        print_hex_byte(gdt::KERNEL_DATA_SELECTOR as u8);
        video::print("\n  User Code Selector:   0x\0");
        print_hex_byte((gdt::USER_CODE_SELECTOR & 0xFF) as u8);
        video::print("\n  User Data Selector:   0x\0");
        print_hex_byte((gdt::USER_DATA_SELECTOR & 0xFF) as u8);
        video::print("\n  TSS Selector:         0x\0");
        print_hex_byte(gdt::TSS_SELECTOR as u8);
        video::print("\n\0");
        
        // Initialize memory map
        video::print("\nMemory map address: \0");
        
        // Simple hex print of the address without using complex functions
        let addr = memory_map_addr;
        let hex_chars: &[u8; 16] = b"0123456789ABCDEF";
        video::print("0x\0");
        
        // Print each nibble (16 nibbles for 64-bit)
        let mut shift: i32 = 60;
        while shift >= 0 {
            let nibble = ((addr >> (shift as u64)) & 0xF) as usize;
            let ch = hex_chars[nibble];
            let s: [u8; 2] = [ch, 0];
            video::print(core::str::from_utf8_unchecked(&s));
            shift = shift - 4;
        }
        video::print("\n\0");
        video::print("\nKernel is stable.\n\0");
    }

    // Halt the CPU permanently
    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}

/// Print a single byte as hex
fn print_hex_byte(b: u8) {
    const HEX_CHARS: [u8; 16] = *b"0123456789ABCDEF";
    let hi = (b >> 4) as usize;
    let lo = (b & 0x0F) as usize;
    unsafe {
        let s: [u8; 3] = [HEX_CHARS[hi], HEX_CHARS[lo], 0];
        video::print(core::str::from_utf8_unchecked(&s));
    }
}

/// Print a decimal number to the screen
fn print_number(mut n: u64) {
    if n == 0 {
        unsafe { video::print("0\0"); }
        return;
    }
    
    let mut buf = [0u8; 21]; // Max u64 is 20 digits + null
    let mut i: usize = 20;
    
    while n > 0 && i > 0 {
        i = i.wrapping_sub(1);
        unsafe {
            *buf.get_unchecked_mut(i) = b'0'.wrapping_add((n % 10) as u8);
        }
        n /= 10;
    }
    
    // Print digit by digit to avoid slice operations
    unsafe {
        while i < 20 {
            let c = *buf.get_unchecked(i);
            let s: [u8; 2] = [c, 0];
            video::print(core::str::from_utf8_unchecked(&s));
            i = i.wrapping_add(1);
        }
    }
}

/// Print a 64-bit hex value to the screen (format: 0x...)
fn print_hex(n: u64) {
    const HEX_CHARS: [u8; 16] = *b"0123456789ABCDEF";
    
    unsafe {
        video::print("0x\0");
    }
    
    // Print each nibble individually
    let mut i: u32 = 0;
    while i < 16 {
        let shift = 60u32.wrapping_sub(i.wrapping_mul(4));
        let nibble = ((n >> shift) & 0xF) as usize;
        // Use get_unchecked since nibble is masked to 0-15
        let c = unsafe { *HEX_CHARS.get_unchecked(nibble) };
        let s: [u8; 2] = [c, 0];
        unsafe {
            video::print(core::str::from_utf8_unchecked(&s));
        }
        i = i.wrapping_add(1);
    }
}
