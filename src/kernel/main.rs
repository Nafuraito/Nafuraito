//! Nafuraito Kernel Entry Point
//! 
//! This is the main entry point for the Nafuraito operating system kernel.
//! It initializes all subsystems and halts the CPU.

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

// =============================================================================
// Required Memory Functions for no_std
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = c as u8;
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return (a as i32) - (b as i32);
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if (dest as usize) < (src as usize) {
        memcpy(dest, src, n)
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
        dest
    }
}

// =============================================================================
// External Library Functions (linked from separate libraries)
// =============================================================================

// Video library (libvideo.a)
mod video {
    extern "C" {
        pub fn set_framebuffer_info(addr: u32, pitch: u32, width: u32, height: u32, bpp: u32);
        pub fn initialize_vga_graphics();
        pub fn writestring_vga(data: *const i8);
    }
    
    pub unsafe fn print(s: &str) {
        writestring_vga(s.as_ptr() as *const i8);
    }
}

// GDT library (libgdt.a)
mod gdt {
    extern "C" {
        pub fn gdt_init();
        pub fn gdt_load(gdt_ptr: *const u8);
        pub fn reload_segments();
        pub fn tss_load(selector: u16);
    }
}

// IDT library (libidt.a)
mod idt {
    extern "C" {
        pub fn idt_init();
        pub fn idt_load(idt_ptr: *const u8);
    }
}

// PIC library (libpic.a)
mod pic {
    extern "C" {
        #[link_name = "pic_init"]
        pub fn init();
        pub fn remap(offset1: u8, offset2: u8);
        pub fn send_eoi(irq: u8);
        pub fn set_mask(irq: u8);
        pub fn clear_mask(irq: u8);
        pub fn pic_outb(port: u16, val: u8);
        pub fn pic_inb(port: u16) -> u8;
        pub fn io_wait();
    }
}

// Memory library (libmemory.a)
mod memory {
    extern "C" {
        #[link_name = "memory_init"]
        pub fn init(memory_map_addr: u64);
        pub fn memory_get_total_usable() -> u64;
        pub fn memory_get_highest_address() -> u64;
    }
}

// PCI library (libpci.a)
mod pci {
    extern "C" {
        #[link_name = "pci_init"]
        pub fn init() -> usize;
        #[link_name = "pci_enumerate"]
        pub fn scan() -> usize;
        #[link_name = "pci_get_device_count"]
        pub fn get_device_count() -> usize;
    }
}

// =============================================================================
// GDT Constants (must match libgdt.a)
// =============================================================================

const KERNEL_CODE_SELECTOR: u16 = 0x08;
const KERNEL_DATA_SELECTOR: u16 = 0x10;
const USER_DATA_SELECTOR: u16 = 0x18 | 3;
const USER_CODE_SELECTOR: u16 = 0x20 | 3;
const TSS_SELECTOR: u16 = 0x28;

// =============================================================================
// Helper Functions
// =============================================================================

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

fn print_hex_u64(num: u64) {
    const HEX_CHARS: [u8; 16] = *b"0123456789ABCDEF";
    unsafe {
        video::print("0x\0");
        let mut shift: i32 = 60;
        while shift >= 0 {
            let nibble = ((num >> (shift as u64)) & 0xF) as usize;
            let ch = HEX_CHARS[nibble];
            let s: [u8; 2] = [ch, 0];
            video::print(core::str::from_utf8_unchecked(&s));
            shift -= 4;
        }
    }
}

// =============================================================================
// Exception/IRQ Handlers (called from assembly in libidt.a)
// =============================================================================

#[no_mangle]
pub extern "C" fn exception_handler(vector: u64, error_code: u64) {
    let exception_name = match vector {
        0 => "Divide Error", 1 => "Debug", 2 => "NMI", 3 => "Breakpoint",
        4 => "Overflow", 5 => "Bound Range", 6 => "Invalid Opcode", 7 => "Device Not Available",
        8 => "Double Fault", 10 => "Invalid TSS", 11 => "Segment Not Present",
        12 => "Stack Fault", 13 => "General Protection Fault", 14 => "Page Fault",
        16 => "FPU Error", 17 => "Alignment Check", 18 => "Machine Check",
        19 => "SIMD Exception", 20 => "Virtualization", 21 => "Control Protection",
        _ => "Unknown",
    };
    unsafe {
        video::print("EXCEPTION: \0");
        video::print(exception_name);
        video::print(" (vector=\0");
        print_hex_u64(vector);
        video::print(", error=\0");
        print_hex_u64(error_code);
        video::print(")\n\0");
        loop { core::arch::asm!("hlt"); }
    }
}

#[no_mangle]
pub extern "C" fn irq_handler(irq: u64) {
    unsafe {
        match irq {
            32 => {
                static mut TICK: u64 = 0;
                TICK += 1;
                if TICK % 100 == 0 {
                    video::print("Timer: \0");
                    print_hex_u64(TICK);
                    video::print("\n\0");
                }
            }
            33 => {
                let scancode = pic::pic_inb(0x60);
                video::print("Key: \0");
                print_hex_u64(scancode as u64);
                video::print("\n\0");
            }
            _ => {
                video::print("IRQ: \0");
                print_hex_u64(irq);
                video::print("\n\0");
            }
        }
        
        // Send EOI
        pic::send_eoi((irq - 32) as u8);
    }
}

// =============================================================================
// Kernel Entry Point
// =============================================================================

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
        // Initialize GDT and TSS first
        gdt::gdt_init();
        
        // Initialize IDT so we can catch exceptions
        idt::idt_init();
        
        // Initialize PIC to remap and mask interrupts
        pic::init();
        
        // Set up framebuffer for graphics output
        if fb_addr != 0 {
            video::set_framebuffer_info(fb_addr, pitch, width, height, bpp);
        }
        
        video::initialize_vga_graphics();
        video::print("Welcome to Nafuraito OS!\n\0");
        video::print("GDT, IDT, PIC initialized... OK\n\0");

        // Initialize Memory Manager
        video::print("Initializing Memory... \0");
        memory::init(memory_map_addr);
        video::print("OK\n\0");

        // Initialize PCI
        video::print("Initializing PCI... \0");
        pci::init();
        let device_count = pci::scan();
        video::print("OK (\0");
        // Print device count
        if device_count > 0 {
            let mut buf = [0u8; 4];
            let mut n = device_count;
            let mut i = 0;
            while n > 0 && i < 3 {
                buf[2 - i] = b'0' + (n % 10) as u8;
                n /= 10;
                i += 1;
            }
            let start = if i == 0 { 2 } else { 3 - i };
            for j in start..3 {
                let c = buf[j];
                let s: [u8; 2] = [c, 0];
                video::print(core::str::from_utf8_unchecked(&s));
            }
        } else {
            video::print("0\0");
        }
        video::print(" devices)\n\0");

        // Enable Interrupts
        core::arch::asm!("sti");
        video::print("Interrupts enabled\n\0");
        
        video::print("\nGDT Info:\n\0");
        video::print("  Kernel Code Selector: 0x\0");
        print_hex_byte(KERNEL_CODE_SELECTOR as u8);
        video::print("\n  Kernel Data Selector: 0x\0");
        print_hex_byte(KERNEL_DATA_SELECTOR as u8);
        video::print("\n  TSS Selector:         0x\0");
        print_hex_byte(TSS_SELECTOR as u8);
        video::print("\n\0");

        // Print memory info
        video::print("\nMemory Info:\n\0");
        video::print("  Total Usable: \0");
        print_hex_u64(memory::memory_get_total_usable());
        video::print(" bytes\n\0");
        video::print("  Highest Addr: \0");
        print_hex_u64(memory::memory_get_highest_address());
        video::print("\n\0");

        video::print("\nKernel is stable.\n\0");
    }

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
