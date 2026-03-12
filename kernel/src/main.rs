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
#![feature(alloc_error_handler)]

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering};

extern crate alloc;

#[path = "drivers/ata.rs"]
mod ata;
mod boot_info;

use ata::{ext4_read_block_ata, ext4_sync_ata, ext4_write_block_ata, EXT4_ATA_DEVICE};

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

/// Send a single byte to any I/O port (used during UART initialisation)
#[inline(always)]
unsafe fn serial_outb_port(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
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
// Kernel Heap Allocator Bridge
// =============================================================================
//
// Stage 1 (early boot):
//   A tiny bump allocator is used before the memory library heap is initialized.
//
// Stage 2 (runtime):
//   Allocation/deallocation is routed to `memory::heap_*` (slab + page fallback).
//
// This two-stage design lets us allocate during bootstrapping and still switch
// to a real reclaiming allocator once PMM/paging are online.
//

const EARLY_HEAP_SIZE: usize = 256 * 1024;

#[repr(align(16))]
struct EarlyHeap([u8; EARLY_HEAP_SIZE]);

static mut EARLY_HEAP: EarlyHeap = EarlyHeap([0; EARLY_HEAP_SIZE]);
static EARLY_HEAP_OFFSET: AtomicUsize = AtomicUsize::new(0);

struct KernelAllocator;

impl KernelAllocator {
    /// Early-boot fallback allocator used until `heap_init_c()` succeeds.
    unsafe fn alloc_early(layout: Layout) -> *mut u8 {
        let align = layout.align().max(16);
        let size = layout.size();
        let mut current = EARLY_HEAP_OFFSET.load(Ordering::Relaxed);

        loop {
            let aligned = (current + align - 1) & !(align - 1);
            let next = aligned.saturating_add(size);
            if next > EARLY_HEAP_SIZE {
                return core::ptr::null_mut();
            }

            match EARLY_HEAP_OFFSET.compare_exchange_weak(
                current,
                next,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return EARLY_HEAP.0.as_mut_ptr().add(aligned),
                Err(prev) => current = prev,
            }
        }
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Once initialized, always use the runtime heap.
        if memory::heap_is_initialized_c() != 0 {
            let ptr = memory::heap_alloc_c(layout.size(), layout.align());
            if !ptr.is_null() {
                return ptr;
            }
        }

        // Fallback path for the early-boot window and OOM fallback.
        Self::alloc_early(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // Early-heap allocations are intentionally leaked because bump storage
        // does not maintain per-allocation metadata. Runtime allocations are
        // fully reclaimed by the slab/page heap backend.
        if memory::heap_is_initialized_c() != 0 {
            memory::heap_dealloc_c(ptr);
        }
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: KernelAllocator = KernelAllocator;

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
    // Don't unwrap - just create layout directly to avoid Debug trait requirement
    if let Ok(layout) = Layout::from_size_align(size, 16) {
        unsafe { GLOBAL_ALLOCATOR.alloc(layout) }
    } else {
        core::ptr::null_mut()
    }
}

/// Kernel heap free entrypoint.
#[no_mangle]
pub extern "C" fn kfree(ptr: *mut u8) {
    unsafe {
        if memory::heap_is_initialized_c() != 0 {
            memory::heap_dealloc_c(ptr);
        }
    }
}

// =============================================================================
// Allocation Runtime Shims (for nightly alloc library)
// =============================================================================

/// Called by nightly's alloc library to check if the alloc shim is available.
#[no_mangle]
pub extern "C" fn __rust_no_alloc_shim_is_unstable_v2() {
    // This is a marker function that indicates we handle allocations ourselves.
    // The alloc library will call this to detect if a custom allocator is set.
}

/// Direct allocation for nightly alloc library.
#[no_mangle]
pub unsafe extern "C" fn __rust_alloc(size: usize, align: usize) -> *mut u8 {
    let layout = Layout::from_size_align(size, align);
    match layout {
        Ok(l) => GLOBAL_ALLOCATOR.alloc(l),
        Err(_) => core::ptr::null_mut(),
    }
}

/// Direct deallocation for nightly alloc library.
#[no_mangle]
pub unsafe extern "C" fn __rust_dealloc(ptr: *mut u8, size: usize, align: usize) {
    let layout = Layout::from_size_align(size, align);
    if let Ok(l) = layout {
        GLOBAL_ALLOCATOR.dealloc(ptr, l);
    }
}

/// Reallocation for nightly alloc library.
#[no_mangle]
pub unsafe extern "C" fn __rust_realloc(
    ptr: *mut u8,
    old_size: usize,
    align: usize,
    new_size: usize,
) -> *mut u8 {
    // Simple realloc: allocate new, copy, free old
    let new_layout = Layout::from_size_align(new_size, align);
    if new_layout.is_err() {
        return core::ptr::null_mut();
    }
    let new_ptr = __rust_alloc(new_size, align);
    if !new_ptr.is_null() {
        let copy_size = core::cmp::min(old_size, new_size);
        core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
        let old_layout = Layout::from_size_align(old_size, align);
        if let Ok(l) = old_layout {
            __rust_dealloc(ptr, old_size, align);
        }
    }
    new_ptr
}

/// Zero-allocating variant for nightly alloc library.
#[no_mangle]
pub unsafe extern "C" fn __rust_alloc_zeroed(size: usize, align: usize) -> *mut u8 {
    let ptr = __rust_alloc(size, align);
    if !ptr.is_null() {
        core::ptr::write_bytes(ptr, 0, size);
    }
    ptr
}

// =============================================================================
// Panicking and Error Handling Runtime Shims
// =============================================================================

/// Called when division by zero occurs in debug mode.
#[no_mangle]
pub fn panic_const_div_by_zero() -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Called when a slice index is out of bounds.
#[no_mangle]
pub fn slice_index_fail(len: usize, index: usize, _index_type: &str) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Called when bounds checking fails.
#[no_mangle]
pub fn panic_bounds_check(_index: usize, _len: usize) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Called when an allocation error occurs (alloc library).
#[no_mangle]
pub fn handle_alloc_error(_: core::alloc::Layout) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Called when Vec/Vec reallocation fails.
#[no_mangle]
pub fn handle_error(_: core::alloc::Layout) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Called when unwrap fails on an error.
#[no_mangle]
pub fn unwrap_failed(msg: &str, _panic_info: &core::panic::Location) -> ! {
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

/// Memory comparison (missing from bare-metal).
#[no_mangle]
pub extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe {
        for i in 0..n {
            let a = *s1.add(i);
            let b = *s2.add(i);
            if a != b {
                return (a as i32) - (b as i32);
            }
        }
    }
    0
}

/// Formatter write_str (for Debug formatting).
#[no_mangle]
pub fn formatter_write_str(_formatter: *mut u8, _s: *const str) -> core::fmt::Result {
    // Stub - just succeed and do nothing
    Ok(())
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
        /// Initialise the VGA text-mode fallback (used when no VESA framebuffer
        /// is available, e.g. when booted via UEFI/OVMF).  After this call
        /// writestring_vga() writes to the VGA text buffer at 0xB8000 instead
        /// of trying to write pixel data to the unmapped default address.
        pub fn init_textmode_fallback();
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
        pub fn disable();
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

        // Kernel heap (slab + page fallback)
        pub fn heap_init_c() -> i32;
        pub fn heap_is_initialized_c() -> u8;
        pub fn heap_alloc_c(size: usize, align: usize) -> *mut u8;
        pub fn heap_dealloc_c(ptr: *mut u8);

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

        // Guard page subsystem
        //
        // guard_page_init()         – must be called once after PMM + paging are online.
        // guard_page_register()     – mark a virtual page as a guard page in the registry.
        // guard_page_is_guard_page()– query the registry; used in the #PF handler.
        // guard_page_alloc_kernel_stack() – allocate a stack with a guard page at its bottom.
        pub fn guard_page_init();
        pub fn guard_page_register(virt_addr: u64) -> i32;
        /// Returns 1 if `addr` falls within a registered guard page, 0 otherwise.
        pub fn guard_page_is_guard_page(addr: u64) -> i32;
        pub fn guard_page_alloc_kernel_stack() -> u64;
    }
}

// EXT4 journaling driver (libext4journaling.a)
// TODO: Re-enable ext4 once linking issues are resolved
/*
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
*/

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
static MMAP_TEST_BLOB: [u8; 32] = *b"MMAP_TEST_DATA_0123456789_PADPAD";

// TODO: ext4_mmap_read_at_safe removed - ext4 support disabled

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
/// Most of these are fatal - we print details and halt.
///
/// **Page fault special handling (vector 14):**
/// Before treating a `#PF` as a generic error, we read `CR2` (the faulting
/// virtual address) and check it against the guard-page registry.  A hit means
/// the kernel stack has grown past its usable region – a stack overflow – and
/// we print a dedicated diagnostic rather than a generic page-fault message.
#[no_mangle]
pub extern "C" fn exception_handler(
    vector: u64,     // Which exception (0-31)
    error_code: u64, // Some exceptions push an error code
    rip: u64,        // Instruction pointer where exception occurred
    cs: u64,         // Code segment
    rflags: u64,     // CPU flags register
) {
    unsafe {
        // =====================================================================
        // Special handling for Page Fault (#PF, vector 14)
        //
        // Step 1: Read CR2 – the CPU stores the faulting virtual address there
        //         immediately on fault entry, before any handler code runs.
        // Step 2: Check the error-code "present" bit (bit 0):
        //           0 = not-present fault (page not mapped)  ← guard pages land here
        //           1 = protection violation (page present but access denied)
        // Step 3: If it is a not-present fault AND CR2 falls inside a registered
        //         guard page, declare a stack overflow and halt.
        // Step 4: Otherwise fall through to the generic fault printer.
        // =====================================================================
        if vector == 14 {
            // Read the faulting virtual address from CR2.
            // SAFETY: CR2 is always valid immediately after a #PF; reading it
            //         here (before any Rust code that might clobber it) is safe.
            let fault_addr: u64;
            core::arch::asm!("mov {}, cr2", out(reg) fault_addr, options(nomem, nostack));

            // bit 0 of the error code: 0 = page not present, 1 = protection fault.
            // Guard pages are *not mapped*, so they always produce a not-present fault.
            let is_not_present = (error_code & 0x1) == 0;

            if is_not_present && memory::guard_page_is_guard_page(fault_addr) != 0 {
                // *** STACK OVERFLOW DETECTED ***
                //
                // The faulting address is inside a registered guard page.
                // The kernel stack grew beyond its usable region.  We cannot
                // safely unwind or recover; print diagnostics and halt.
                //
                // NOTE: We are running on the IST5 emergency stack (set up in
                //       gdt/lib.rs), so this handler itself has a valid stack
                //       even though the task's stack is exhausted.
                serial_print("*** KERNEL STACK OVERFLOW ***\n");
                serial_print("  Guard page hit at: ");
                serial_print_hex_u64(fault_addr);
                serial_print("\n  RIP: ");
                serial_print_hex_u64(rip);
                serial_print("\n  Error code: ");
                serial_print_hex_u64(error_code);
                serial_print("\n");

                video::print("*** KERNEL STACK OVERFLOW ***\n\0");
                video::print("  Guard page hit at: \0");
                print_hex_u64(fault_addr);
                video::print("\n  RIP: \0");
                print_hex_u64(rip);
                video::print("\n\0");

                // Halt – the kernel state is corrupt; no recovery is possible.
                loop {
                    core::arch::asm!("hlt");
                }
            }

            // Not a guard-page hit – print the generic page-fault diagnostic.
            let is_write      = (error_code & 0x2) != 0;
            let is_user       = (error_code & 0x4) != 0;
            let is_rsvd       = (error_code & 0x8) != 0;
            let is_instr_fetch = (error_code & 0x10) != 0;

            serial_print("PAGE FAULT: addr=");
            serial_print_hex_u64(fault_addr);
            serial_print(" error=");
            serial_print_hex_u64(error_code);
            serial_print(" [");
            if !is_not_present { serial_print("P"); }    // page was present
            if is_write        { serial_print("W"); }    // write access
            if is_user         { serial_print("U"); }    // user mode
            if is_rsvd         { serial_print("RSVD"); } // reserved PTE bit set
            if is_instr_fetch  { serial_print("IF"); }   // instruction fetch
            serial_print("]\n  RIP=");
            serial_print_hex_u64(rip);
            serial_print("\n");

            video::print("PAGE FAULT: addr=\0");
            print_hex_u64(fault_addr);
            video::print(" error=\0");
            print_hex_u64(error_code);
            video::print("\n  RIP=\0");
            print_hex_u64(rip);
            video::print("\n\0");

            loop {
                core::arch::asm!("hlt");
            }
        }

        // =====================================================================
        // Generic exception handler (all vectors except 14)
        // =====================================================================

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
            }
            33 => {
                // Keyboard interrupt (IRQ 1)
                // Read the scancode from the keyboard controller
                // let scancode = pic::pic_inb(0x60); // Keyboard data port
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
        // Temporary: keep hardware interrupts masked to avoid spurious IRQs
        // triggering before the IRQ path is fully stabilized. Flip to true
        // once IRQ handling is validated again.
        const ENABLE_IRQS: bool = true;

        // -------------------------------------------------------------------------
        // Initialize COM1 (8N1, 115200 baud) as the very first thing so that
        // all subsequent serial_print() calls produce clean output.
        // Without this the 16550 UART is in an undefined state; QEMU emulates
        // the framing-error behaviour (0xFF/0x00 byte pairs) which shows up as
        // □ in the terminal.
        //
        // Register map (base = 0x3F8):
        //   +0  THR/RBR/DLL  +1  IER/DLM  +2  IIR/FCR
        //   +3  LCR           +4  MCR       +5  LSR   +6 MSR
        // -------------------------------------------------------------------------
        serial_outb_port(0x3F9, 0x00); // +1 IER  — disable all UART interrupts
        serial_outb_port(0x3FB, 0x80); // +3 LCR  — set DLAB to access baud divisor
        serial_outb_port(0x3F8, 0x01); // +0 DLL  — divisor low  = 1 → 115200 baud
        serial_outb_port(0x3F9, 0x00); // +1 DLM  — divisor high = 0
        serial_outb_port(0x3FB, 0x03); // +3 LCR  — clear DLAB; 8N1 (8 data, no parity, 1 stop)
        serial_outb_port(0x3FA, 0xC7); // +2 FCR  — enable & clear FIFOs, 14-byte threshold
        serial_outb_port(0x3FC, 0x0B); // +4 MCR  — DTR + RTS + OUT2 (enables IRQ line)

        serial_print("COM1 initialized\n");

        // Read BootInfo from physical 0x6000 (written by bootloader before
        // jumping to the kernel). This identifies the boot path (BIOS or UEFI)
        // and provides the ACPI RSDP address.
        boot_info::init();
        serial_print("Boot path: ");
        serial_print(boot_info::boot_type_str());
        serial_print("\n");

        // Initialize GDT and TSS first
        gdt::gdt_init();

        // Initialize IDT so we can catch exceptions
        idt::idt_init();

        // Initialize PIC to remap and mask interrupts
        pic::init();
        if !ENABLE_IRQS {
            pic::disable();
        }

        // -------------------------------------------------------------------------
        // Set up display output.
        //
        // If the bootloader provided a valid VESA framebuffer address use it.
        // Otherwise (VESA is a real-mode BIOS service and fails when booted via
        // OVMF/UEFI, so fb_addr arrives as 0) fall back to VGA text mode.
        // VGA text mode writes character+attribute bytes to 0xB8000 and is
        // always available on x86 without any special setup.
        // -------------------------------------------------------------------------
        if fb_addr != 0 {
            // VBE/BIOS passes bits per pixel (e.g. 32); vga.c uses BYTES per pixel.
            // UEFI boot path already passes bytes directly (4 for 32 bpp).
            let bpp_bytes = if bpp > 8 { bpp / 8 } else { bpp };
            video::set_framebuffer_info(fb_addr, pitch, width, height, bpp_bytes);
            video::initialize_vga_graphics();
        } else {
            // No VESA framebuffer: initialise the text-mode fallback so that
            // writestring_vga() routes to VGA text mode (0xB8000) instead of
            // writing pixel data to the unmapped default address 0xFD000000.
            video::init_textmode_fallback();
        }
        video::print("Welcome to Nafuraito OS!\n\0");
        video::print("GDT, IDT, PIC initialized... OK\n\0");

        // Initialize Memory Manager (E820 parsing).
        // Both BIOS and UEFI boot paths provide an E820-format memory map:
        //   BIOS: BIOS INT 15h E820 entries at 0x0500 (passed in R9)
        //   UEFI: UEFI memory descriptors converted to E820 at 0x7000 (passed in R9)
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

        // Initialize the runtime kernel heap allocator.
        //
        // After this point, Rust global allocation and C `kmalloc/kfree`
        // are routed through the memory library's slab/page heap backend.
        video::print("Initializing kernel heap... \0");
        let heap_result = memory::heap_init_c();
        if heap_result == 0 && memory::heap_is_initialized_c() != 0 {
            video::print("OK\n\0");
        } else {
            video::print("FAILED\n\0");
        }

        // Initialize guard page subsystem.
        //
        // Must come after both the PMM (we need physical frames for stack pages)
        // and paging (we call map_page to set up the usable stack pages).
        // The guard pages themselves are NOT mapped – that is intentional.
        //
        // After this call, any kernel stack allocated with
        // `guard_page_alloc_kernel_stack()` will have a not-present guard page
        // at its bottom.  When the stack overflows, the resulting #PF is caught
        // in `exception_handler` which checks CR2 against the registry and
        // prints "KERNEL STACK OVERFLOW" instead of a generic page-fault panic.
        video::print("Initializing guard pages... \0");
        memory::guard_page_init();
        video::print("OK\n\0");

        // Page-mapping self-test (disabled by default due to instability in some environments).
        const RUN_PAGING_SELFTEST: bool = false;
        if RUN_PAGING_SELFTEST {
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

                // Map it to a virtual address (use a kernel-space high address)
                let test_virt = 0xFFFF_8000_0000_0000u64;
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
        }

        // Demand paging self-test (disabled to avoid early page faults; enable when stable).
        const RUN_DEMAND_PAGING_TEST: bool = false;
        if RUN_DEMAND_PAGING_TEST {
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

            // TODO: Ext4 mount disabled for now due to linker symbol issues.
            // Using fallback in-memory test blob instead.
            video::print("  EXT4: disabled (linking issues)\n\0");
            file_backed_mapped = memory::memory_mmap_file(
                MMAP_FILE_BASE,
                mmap_size,
                0x0,
                (&MMAP_TEST_BLOB as *const [u8; 32]) as *mut u8,
                mmap_test_read_at,
                0,
                MMAP_TEST_BLOB.len() as u64,
            );

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

        // Enable Interrupts (guarded while IRQ path is unstable)
        if ENABLE_IRQS {
            core::arch::asm!("sti");
            video::print("Interrupts enabled\n\0");
        } else {
            video::print("Interrupts stay masked (IRQ path disabled)\n\0");
        }

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
