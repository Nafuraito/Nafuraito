//! =============================================================================
//! Nafuraito Kernel Entry Point
//! =============================================================================
//!
//! Welcome to the heart of the Nafuraito operating system!
//!
//! This file is where everything comes together. The bootloader has done its
//! job and handed control over to us. Now it's our turn to:
//!
//! 1. Set up the execution environment (GDT, IDT, interrupts)
//! 2. Initialize memory management (physical and virtual memory)
//! 3. Set up device drivers (PCI, video, etc.)
//! 4. Eventually start the first user process
//!
//! We're running in 64-bit long mode with paging enabled. The bootloader
//! set up a basic identity map for us, but we'll need to do more sophisticated
//! memory management soon.
//!
//! Note: We're using #![no_std] - this means we don't have the Rust standard
//! library. No Vec, String, println!, or any of that. We're on our own!
//!
//! =============================================================================

#![crate_type = "staticlib"]
#![no_std] // We're a kernel - no standard library for us!
#![no_main] // We define our own entry point (not the usual main())
#![allow(unused)] // Allow unused code while we're developing

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering};

extern crate alloc;

// =============================================================================
// Serial Debug Output
// =============================================================================
//
// These functions let us write to the COM1 serial port for debugging.
// When running in QEMU or with a serial cable, we can see kernel messages
// even if the screen isn't working. Super useful for debugging!
//
// =============================================================================

/// Send a single byte to the serial port (COM1)
#[inline(always)]
unsafe fn serial_outb(val: u8) {
    // Out instruction: sends a byte to an I/O port
    // COM1 is at port 0x3F8
    core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") val, options(nomem, nostack));
}

/// Print a string to the serial port
#[inline(always)]
unsafe fn serial_print(s: &str) {
    for &b in s.as_bytes() {
        serial_outb(b);
    }
}

/// Print a 64-bit hex number to serial (like 0x1234ABCD)
#[inline(always)]
unsafe fn serial_print_hex_u64(mut num: u64) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    serial_print("0x");
    if num == 0 {
        serial_print("0");
        return;
    }
    // Convert number to hex digits (right to left, then reverse)
    let mut buf = [0u8; 16];
    let mut i = 0usize;
    while num > 0 && i < 16 {
        let digit = (num & 0xF) as usize; // Get lowest 4 bits
        buf[i] = HEX[digit];
        num >>= 4; // Shift right by 4 bits
        i += 1;
    }
    // Print in reverse order (most significant digit first)
    while i > 0 {
        i -= 1;
        serial_outb(buf[i]);
    }
}

// =============================================================================
// Panic Handler
// =============================================================================
//
// When Rust code panics (like calling unwrap() on None), this function is called.
// Since we're no_std, we have to provide our own panic handler.
//
// All we can do here is halt the CPU - we're in an unrecoverable state.
// In a more sophisticated kernel, we might try to print a panic message and
// backtrace, but for now we just stop.
//
// =============================================================================

/// Panic handler for the kernel
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Infinite halt loop - the CPU will stay here forever
    loop {
        unsafe {
            core::arch::asm!("cli"); // Disable interrupts
            core::arch::asm!("hlt"); // Halt the CPU
        }
    }
}

// =============================================================================
// Required Memory Functions for no_std
// =============================================================================
//
// When we use #![no_std], Rust still needs some basic memory functions.
// These are usually provided by the C library, but we don't have one.
// So we implement them ourselves - simple byte-by-byte versions.
//
// These are marked #[no_mangle] so the linker can find them, and they
// use "C" calling convention so they match what LLVM expects.
//
// =============================================================================

/// Copy memory from src to dest (dest and src must not overlap!)
#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i); // Copy byte by byte
        i += 1;
    }
    dest
}

/// Fill memory with a constant byte
#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = c as u8; // Set each byte to c
        i += 1;
    }
    dest
}

/// Compare two memory regions
/// Returns 0 if equal, negative if s1 < s2, positive if s1 > s2
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

/// Move memory (like memcpy but handles overlapping regions)
#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if (dest as usize) < (src as usize) {
        // Dest is before src - copy forward
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
// Minimal Heap Allocator (bump)
// =============================================================================
//
// TODO: Replace this with a real kernel heap (slab + general allocator).
// TODO: Add proper freeing and reuse to avoid memory exhaustion.
//

const HEAP_SIZE: usize = 256 * 1024;

#[repr(align(16))]
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

static HEAP_OFFSET: AtomicUsize = AtomicUsize::new(0);

struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(16);
        let size = layout.size();
        let mut current = HEAP_OFFSET.load(Ordering::Relaxed);

        loop {
            let aligned = (current + align - 1) & !(align - 1);
            let next = aligned.saturating_add(size);
            if next > HEAP_SIZE {
                return core::ptr::null_mut();
            }

            match HEAP_OFFSET.compare_exchange_weak(
                current,
                next,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return HEAP.as_mut_ptr().add(aligned),
                Err(prev) => current = prev,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // TODO: Implement free once a real allocator is in place.
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: BumpAllocator = BumpAllocator;

#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Kernel heap allocation entrypoint (used by ext4 journaling).
#[no_mangle]
pub extern "C" fn kmalloc(size: usize) -> *mut u8 {
    unsafe { GLOBAL_ALLOCATOR.alloc(Layout::from_size_align(size, 16).unwrap()) }
}

/// Kernel heap free entrypoint (currently a no-op).
#[no_mangle]
pub extern "C" fn kfree(_ptr: *mut u8) {
    // TODO: Wire this into the real kernel allocator.
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

        // PMM functions (Physical Memory Manager)
        pub fn pmm_init_c(memory_map_addr: u64) -> i32;
        pub fn pmm_alloc_frame() -> u64;
        pub fn pmm_free_frame(addr: u64) -> i32;
        pub fn pmm_free_memory() -> u64;
        pub fn pmm_used_memory() -> u64;
        pub fn pmm_total_memory() -> u64;
        pub fn pmm_free_frame_count() -> u64;
        pub fn pmm_is_initialized() -> i32;

        // Paging functions
        pub fn paging_check_la57_support() -> u8;
        pub fn paging_check_nx_support() -> u8;
        pub fn paging_is_la57_enabled() -> u8;
        pub fn paging_read_cr3() -> u64;
        pub fn paging_init_c(root_table_phys: u64) -> i32;
        pub fn paging_get_mode() -> i32;
        pub fn paging_map_page(virt_addr: u64, phys_addr: u64, flags: u64) -> i32;
        pub fn paging_unmap_page(virt_addr: u64) -> i32;
        pub fn paging_translate_address(virt_addr: u64) -> u64;
        pub fn paging_is_mapped(virt_addr: u64) -> u8;

        // PAT functions
        pub fn pat_check_support() -> u8;
        pub fn pat_init_c() -> i32;
        pub fn pat_is_initialized() -> u8;
        pub fn pat_read_msr() -> u64;

        // CoW functions
        pub fn cow_init();

        // Demand-paged mmap helpers
        pub fn memory_mmap_zero(start: u64, size: usize, flags: u64) -> u64;
        pub fn memory_mmap_file(
            start: u64,
            size: usize,
            flags: u64,
            ctx: *mut u8,
            read_at: extern "C" fn(ctx: *mut u8, offset: u64, dst: *mut u8, len: usize) -> usize,
            file_offset: u64,
            file_size: u64,
        ) -> u64;
        pub fn memory_munmap(start: u64, size: usize) -> i32;
    }
}

// EXT4 journaling driver (libext4journaling.a)
type Ext4ReadBlockC = extern "C" fn(dev: *mut u8, block: u64, buffer: *mut u8, len: usize) -> i32;
type Ext4WriteBlockC =
    extern "C" fn(dev: *mut u8, block: u64, buffer: *const u8, len: usize) -> i32;
type Ext4SyncC = extern "C" fn(dev: *mut u8) -> i32;

mod ext4 {
    use super::{Ext4ReadBlockC, Ext4SyncC, Ext4WriteBlockC};

    extern "C" {
        pub fn ext4_mount_c(
            device: *mut u8,
            read_block: Option<Ext4ReadBlockC>,
            write_block: Option<Ext4WriteBlockC>,
            sync: Option<Ext4SyncC>,
        ) -> *mut u8;
        pub fn ext4_unmount_c(fs: *mut u8);
        pub fn ext4_mmap_open_c(
            fs: *const u8,
            path: *const u8,
            path_len: usize,
            out_size: *mut u64,
        ) -> *mut u8;
        pub fn ext4_mmap_close_c(file: *mut u8);
        pub fn ext4_mmap_read_at(ctx: *mut u8, offset: u64, dst: *mut u8, len: usize) -> usize;
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
        if num == 0 {
            video::print("0\0");
            return;
        }
        // Find the highest non-zero nibble
        let mut started = false;
        let mut shift: i32 = 60;
        while shift >= 0 {
            let nibble = ((num >> (shift as u64)) & 0xF) as usize;
            if nibble != 0 || started {
                started = true;
                let ch = HEX_CHARS[nibble];
                let s: [u8; 2] = [ch, 0];
                video::print(core::str::from_utf8_unchecked(&s));
            }
            shift -= 4;
        }
    }
}

// =============================================================================
// Demand paging test helpers
// =============================================================================

// Small test blob used by the file-backed mmap read_at callback.
static MMAP_TEST_BLOB: [u8; 32] = *b"MMAP_TEST_DATA_0123456789";

// TODO: Replace these stubs with a real block device driver.
extern "C" fn ext4_read_block_stub(
    _dev: *mut u8,
    _block: u64,
    _buffer: *mut u8,
    _len: usize,
) -> i32 {
    -1
}

extern "C" fn ext4_write_block_stub(
    _dev: *mut u8,
    _block: u64,
    _buffer: *const u8,
    _len: usize,
) -> i32 {
    -1
}

extern "C" fn ext4_sync_stub(_dev: *mut u8) -> i32 {
    -1
}

/// Test read_at callback for file-backed demand paging.
///
/// TODO: Replace this with an ext4-backed file once the filesystem is wired
/// into the kernel main path.
#[no_mangle]
pub extern "C" fn mmap_test_read_at(ctx: *mut u8, offset: u64, dst: *mut u8, len: usize) -> usize {
    if ctx.is_null() || dst.is_null() || len == 0 {
        return 0;
    }

    let blob = unsafe { &*(ctx as *const [u8; 32]) };
    let start = offset as usize;
    if start >= blob.len() {
        return 0;
    }

    let available = blob.len() - start;
    let to_copy = core::cmp::min(len, available);
    unsafe {
        core::ptr::copy_nonoverlapping(blob.as_ptr().add(start), dst, to_copy);
    }

    to_copy
}

fn halt_forever() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

// =============================================================================
// Exception/IRQ Handlers (called from assembly in libidt.a)
// =============================================================================
//
// When a CPU exception happens (like divide by zero, page fault, etc.) or
// a hardware interrupt fires (timer, keyboard, etc.), the IDT directs control
// to assembly stubs that eventually call these Rust functions.
//
// We print information about what happened and then halt (for exceptions) or
// handle the interrupt and continue (for IRQs).
//
// =============================================================================

/// Called when a CPU exception occurs
///
/// CPU exceptions are things like:
/// - Divide by zero (vector 0)
/// - Page faults (vector 14)
/// - General protection faults (vector 13)
/// - Double faults (vector 8) - when handling an exception causes another exception
///
/// Most of these are fatal - we print details and halt
#[no_mangle]
pub extern "C" fn exception_handler(
    vector: u64,     // Which exception (0-31)
    error_code: u64, // Some exceptions push an error code
    rip: u64,        // Instruction pointer where exception occurred
    cs: u64,         // Code segment
    rflags: u64,     // CPU flags register
) {
    // Map vector number to human-readable exception name
    let exception_name = match vector {
        0 => "Divide Error\0",
        1 => "Debug\0",
        2 => "NMI\0",
        3 => "Breakpoint\0",
        4 => "Overflow\0",
        5 => "Bound Range\0",
        6 => "Invalid Opcode\0",
        7 => "Device Not Available\0",
        8 => "Double Fault\0",
        10 => "Invalid TSS\0",
        11 => "Segment Not Present\0",
        12 => "Stack Fault\0",
        13 => "General Protection Fault\0",
        14 => "Page Fault\0",
        16 => "FPU Error\0",
        17 => "Alignment Check\0",
        18 => "Machine Check\0",
        19 => "SIMD Exception\0",
        20 => "Virtualization\0",
        21 => "Control Protection\0",
        _ => "Unknown\0",
    };

    unsafe {
        // Print to serial port first (most reliable)
        serial_print("EXCEPTION: ");
        serial_print(exception_name);
        serial_print(" (vector=");
        serial_print_hex_u64(vector);
        serial_print(", error=");
        serial_print_hex_u64(error_code);
        serial_print(")\n");
        serial_print("  RIP=");
        serial_print_hex_u64(rip);
        serial_print(" CS=");
        serial_print_hex_u64(cs);
        serial_print(" RFLAGS=");
        serial_print_hex_u64(rflags);
        serial_print("\n");

        // Also print to screen if video is initialized
        video::print("EXCEPTION: \0");
        video::print(exception_name);
        video::print(" (vector=\0");
        print_hex_u64(vector);
        video::print(", error=\0");
        print_hex_u64(error_code);
        video::print(")\n\0");
        video::print("  RIP=\0");
        print_hex_u64(rip);
        video::print(" CS=\0");
        print_hex_u64(cs);
        video::print(" RFLAGS=\0");
        print_hex_u64(rflags);
        video::print("\n\0");

        // Hang forever - this is fatal
        loop {
            core::arch::asm!("hlt");
        }
    }
}

/// Called when a hardware interrupt (IRQ) fires
///
/// Hardware interrupts are things like:
/// - IRQ 0 (vector 32): Timer interrupt - fires ~1200 times per second
/// - IRQ 1 (vector 33): Keyboard interrupt - fires when a key is pressed
/// - IRQ 14/15: Hard disk interrupts
/// - etc.
///
/// Unlike exceptions, IRQs are expected and we handle them then continue
#[no_mangle]
pub extern "C" fn irq_handler(irq: u64) {
    unsafe {
        match irq {
            32 => {
                // Timer interrupt (IRQ 0)
                // Fires about 1200 times per second
                static mut TICK: u64 = 0;
                TICK += 1;
                if TICK % 1200 == 0 {
                    // Print every second (1200 ticks)
                    video::print("Timer: \0");
                    print_hex_u64(TICK);
                    video::print("\n\0");
                }
            }
            33 => {
                // Keyboard interrupt (IRQ 1)
                // Read the scancode from the keyboard controller
                let scancode = pic::pic_inb(0x60); // Keyboard data port
                video::print("Key: \0");
                print_hex_u64(scancode as u64);
                video::print("\n\0");
            }
            _ => {
                // Unknown IRQ - just print it
                video::print("IRQ: \0");
                print_hex_u64(irq);
                video::print("\n\0");
            }
        }

        // Send End-Of-Interrupt signal to the PIC
        // This tells the interrupt controller we're done handling this IRQ
        // Important: if we don't do this, we won't get any more interrupts!
        pic::send_eoi((irq - 32) as u8);
    }
}

// =============================================================================
// Kernel Entry Point
// =============================================================================
//
// This is where the bootloader hands control to the kernel! The bootloader has:
// - Loaded the kernel into memory at 0x100000 (1MB)
// - Set up a basic identity map for paging
// - Switched to 64-bit long mode
// - Jumped to this function
//
// Parameters passed from bootloader:
//   fb_addr         - Physical address of framebuffer (for graphics)
//   pitch           - Bytes per scanline
//   width/height    - Screen resolution
//   bpp             - Bits per pixel
//   memory_map_addr - Address of E820 memory map (tells us what RAM is available)
//
// Our job:
// 1. Initialize CPU structures (GDT, IDT)
// 2. Initialize interrupt controllers (PIC)
// 3. Set up memory management (PMM, paging, PAT)
// 4. Initialize device drivers
// 5. Eventually start the first process (not implemented yet)
//
// =============================================================================

#[no_mangle]
pub extern "C" fn enter(
    fb_addr: u32,         // Framebuffer physical address
    pitch: u32,           // Bytes per scanline
    width: u32,           // Screen width in pixels
    height: u32,          // Screen height in pixels
    bpp: u32,             // Bits per pixel (usually 32)
    memory_map_addr: u64, // Address of E820 memory map
) -> ! {
    // The "!" means this function never returns
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

        // Initialize Memory Manager (E820 parsing)
        video::print("Initializing Memory... \0");
        memory::init(memory_map_addr);
        video::print("OK\n\0");

        // Debug: Print highest address
        video::print("  Highest addr: \0");
        print_hex_u64(memory::memory_get_highest_address());
        video::print("\n\0");

        // Initialize Paging subsystem
        video::print("Initializing Paging...\n\0");
        let cr3 = memory::paging_read_cr3();
        video::print("  Current CR3: \0");
        print_hex_u64(cr3);
        video::print("\n\0");

        let paging_result = memory::paging_init_c(cr3);
        if paging_result == 0 {
            video::print("  Paging init: OK\n\0");

            // Check paging mode
            let mode = memory::paging_get_mode();
            video::print("  Mode: \0");
            if mode == 1 {
                video::print("5-level (LA57)\0");
            } else if mode == 0 {
                video::print("4-level (LA48)\0");
            } else {
                video::print("Unknown\0");
            }
            video::print("\n\0");

            // Check CPU features
            let la57_supported = memory::paging_check_la57_support();
            let la57_enabled = memory::paging_is_la57_enabled();
            let nx_supported = memory::paging_check_nx_support();

            video::print("  LA57 support: \0");
            if la57_supported != 0 {
                video::print("Yes\0");
            } else {
                video::print("No\0");
            }
            video::print("\n  LA57 enabled: \0");
            if la57_enabled != 0 {
                video::print("Yes\0");
            } else {
                video::print("No\0");
            }
            video::print("\n  NX support:   \0");
            if nx_supported != 0 {
                video::print("Yes\0");
            } else {
                video::print("No\0");
            }
            video::print("\n\0");
        } else {
            video::print("  Paging init: FAILED\n\0");
        }

        // Initialize PAT (Page Attribute Table)
        video::print("Initializing PAT...\n\0");
        let pat_supported = memory::pat_check_support();
        if pat_supported != 0 {
            video::print("  PAT support: Yes\n\0");
            let pat_result = memory::pat_init_c();
            if pat_result == 0 {
                video::print("  PAT init: OK\n\0");
                video::print("    Memory types configured:\n\0");
                video::print("      PA0: WB (Normal memory)\n\0");
                video::print("      PA1: WT (Write-through)\n\0");
                video::print("      PA2: UC (MMIO)\n\0");
                video::print("      PA5: WC (Framebuffer)\n\0");
            } else {
                video::print("  PAT init: FAILED\n\0");
            }
        } else {
            video::print("  PAT support: No (not available on this CPU)\n\0");
        }

        // Initialize CoW (Copy-on-Write) system
        video::print("Initializing CoW system... \0");
        memory::cow_init();
        video::print("OK\n\0");

        // Initialize Physical Memory Manager (bitmap allocator)
        video::print("Initializing PMM...\n\0");
        video::print("  Memory map addr: \0");
        print_hex_u64(memory_map_addr);
        video::print("\n  Total usable: \0");
        print_hex_u64(memory::memory_get_total_usable());
        video::print("\n  Starting PMM init...\n\0");

        let pmm_result = memory::pmm_init_c(memory_map_addr);
        video::print("PMM init returned: \0");
        print_hex_byte(pmm_result as u8);
        video::print("\n\0");
        if pmm_result == 0 {
            video::print("OK (\0");
            print_hex_u64(memory::pmm_free_memory());
            video::print(" bytes free)\n\0");
        } else {
            video::print("FAILED (error=\0");
            print_hex_u64((-pmm_result) as u64);
            video::print(")\n\0");
        }

        // Test page mapping functionality (after PMM is initialized!)
        serial_print("About to test page mapping...\n");
        video::print("\nTesting Page Mapping...\n\0");

        // Allocate a physical frame for testing
        serial_print("Allocating test frame...\n");
        let test_phys = memory::pmm_alloc_frame();
        serial_print("Allocated frame: ");
        serial_print_hex_u64(test_phys);
        serial_print("\n");

        if test_phys != 0 {
            video::print("  Allocated test frame: \0");
            print_hex_u64(test_phys);
            video::print("\n\0");
            serial_print("About to map page...\n");

            // Map it to a virtual address (we'll use high address to avoid conflicts)
            let test_virt = 0xFFFF_8000_0000_0000u64; // Kernel space
            serial_print("Mapping virt ");
            serial_print_hex_u64(test_virt);
            serial_print(" to phys ");
            serial_print_hex_u64(test_phys);
            serial_print("\n");

            serial_print("Calling paging_map_page...\n");

            // First test a simple function to verify calling works
            {
                extern "C" {
                    fn paging_test_simple(a: u64, b: u64) -> u64;
                }
                serial_print("Testing simple C call: ");
                let result = paging_test_simple(10, 20);
                serial_print_hex_u64(result);
                serial_print("\\n");
            }

            // Try calling extern directly instead of through module
            let map_result = {
                extern "C" {
                    fn paging_map_page(virt_addr: u64, phys_addr: u64, flags: u64) -> i32;
                }
                paging_map_page(test_virt, test_phys, 0x3)
            };

            serial_print("Map result: ");
            serial_print_hex_u64(map_result as u64);
            serial_print("\n");

            if map_result == 0 {
                video::print("  Mapped virt \0");
                print_hex_u64(test_virt);
                video::print(" -> phys \0");
                print_hex_u64(test_phys);
                video::print("\n\0");

                // Verify translation
                let translated = memory::paging_translate_address(test_virt);
                video::print("  Translation check: \0");
                if translated == test_phys {
                    video::print("OK\0");
                } else {
                    video::print("FAILED (got \0");
                    print_hex_u64(translated);
                    video::print(")\0");
                }
                video::print("\n\0");

                // Unmap the page
                let unmap_result = memory::paging_unmap_page(test_virt);
                if unmap_result == 0 {
                    video::print("  Unmapped successfully\n\0");

                    // Verify it's unmapped
                    let is_mapped = memory::paging_is_mapped(test_virt);
                    if is_mapped == 0 {
                        video::print("  Unmap verification: OK\n\0");
                    } else {
                        video::print("  Unmap verification: FAILED\n\0");
                    }
                } else {
                    video::print("  Unmap FAILED\n\0");
                }
            } else {
                video::print("  Map FAILED\n\0");
            }

            // Free the test frame
            memory::pmm_free_frame(test_phys);
        } else {
            video::print("  Could not allocate test frame\n\0");
        }

        // Demand paging mmap tests (anonymous + file-backed).
        // TODO: Move these into a proper kernel test suite once available.
        video::print("\nTesting demand-paged mmap...\n\0");

        let mut mmap_ok = true;
        const MMAP_TEST_BASE: u64 = 0xFFFF_8000_0100_0000;
        const MMAP_FILE_BASE: u64 = 0xFFFF_8000_0200_0000;
        let mmap_size: usize = 2 * 4096;
        let mmap_flags: u64 = 0x2; // WRITABLE; PRESENT is added on fault.

        let anon_mapped = memory::memory_mmap_zero(MMAP_TEST_BASE, mmap_size, mmap_flags);
        if anon_mapped != 0 {
            video::print("  Anonymous mmap: OK at \0");
            print_hex_u64(anon_mapped);
            video::print("\n\0");

            unsafe {
                let ptr = anon_mapped as *mut u8;
                core::ptr::write_volatile(ptr, 0xAB);
                let v0 = core::ptr::read_volatile(ptr);
                let v1 = core::ptr::read_volatile(ptr.add(4096));

                video::print("  Anon readback: \0");
                print_hex_byte(v0);
                video::print(" / \0");
                print_hex_byte(v1);
                video::print("\n\0");
            }

            let unmap_result = memory::memory_munmap(anon_mapped, mmap_size);
            if unmap_result == 0 {
                video::print("  Anonymous munmap: OK\n\0");
            } else {
                video::print("  Anonymous munmap: FAILED\n\0");
                mmap_ok = false;
            }
        } else {
            video::print("  Anonymous mmap: FAILED\n\0");
            mmap_ok = false;
        }

        // Prefer ext4-backed file mapping; fall back to an in-memory blob.
        // TODO: Remove the fallback once a real block device is wired in.
        let mut file_backed_mapped = 0u64;
        let ext4_fs = unsafe {
            ext4::ext4_mount_c(
                core::ptr::null_mut(),
                Some(ext4_read_block_stub),
                Some(ext4_write_block_stub),
                Some(ext4_sync_stub),
            )
        };

        if !ext4_fs.is_null() {
            video::print("  EXT4 mount: OK\n\0");
            let path = b"/mmap-test.bin";
            let mut file_size = 0u64;
            let file = unsafe {
                ext4::ext4_mmap_open_c(ext4_fs, path.as_ptr(), path.len(), &mut file_size)
            };

            if file.is_null() {
                video::print("  EXT4 mmap open: FAILED\n\0");
                mmap_ok = false;
            } else {
                file_backed_mapped = memory::memory_mmap_file(
                    MMAP_FILE_BASE,
                    mmap_size,
                    0x0,
                    file as *mut u8,
                    ext4::ext4_mmap_read_at,
                    0,
                    file_size,
                );

                unsafe {
                    ext4::ext4_mmap_close_c(file);
                }
            }

            unsafe {
                ext4::ext4_unmount_c(ext4_fs);
            }
        } else {
            video::print("  EXT4 mount: SKIPPED (no block device)\n\0");
            file_backed_mapped = memory::memory_mmap_file(
                MMAP_FILE_BASE,
                mmap_size,
                0x0,
                (&MMAP_TEST_BLOB as *const [u8; 32]) as *mut u8,
                mmap_test_read_at,
                0,
                MMAP_TEST_BLOB.len() as u64,
            );
        }

        if file_backed_mapped != 0 {
            video::print("  File-backed mmap: OK at \0");
            print_hex_u64(file_backed_mapped);
            video::print("\n\0");

            unsafe {
                let ptr = file_backed_mapped as *const u8;
                let b0 = core::ptr::read_volatile(ptr);
                let b1 = core::ptr::read_volatile(ptr.add(1));
                video::print("  File readback: \0");
                print_hex_byte(b0);
                video::print(" \0");
                print_hex_byte(b1);
                video::print("\n\0");
            }

            let unmap_result = memory::memory_munmap(file_backed_mapped, mmap_size);
            if unmap_result == 0 {
                video::print("  File-backed munmap: OK\n\0");
            } else {
                video::print("  File-backed munmap: FAILED\n\0");
                mmap_ok = false;
            }
        } else {
            video::print("  File-backed mmap: FAILED\n\0");
            mmap_ok = false;
        }

        if !mmap_ok {
            video::print("mmap self-test FAILED\n\0");
            halt_forever();
        }

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
