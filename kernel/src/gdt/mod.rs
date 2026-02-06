//! Global Descriptor Table (GDT) Module
//! =============================================================================
//!
//! The GDT is one of the fundamental data structures in x86-64 architecture.
//!
//! In 64-bit mode, segmentation is mostly "flat" (all segments cover
//! all memory), but we still need the GDT for privilege levels and the TSS.
//!
//! Our GDT layout:
//! - Entry 0 (0x00): Null descriptor (required by CPU, never used)
//! - Entry 1 (0x08): Kernel code segment (Ring 0)
//! - Entry 2 (0x10): Kernel data segment (Ring 0)
//! - Entry 3 (0x18): User data segment (Ring 3)
//! - Entry 4 (0x20): User code segment (Ring 3)
//! - Entry 5-6 (0x28): TSS descriptor (16 bytes in 64-bit mode)
//!
//! =============================================================================

#![allow(unused)]

pub mod tss;

use core::mem::size_of;
use self::tss::TaskStateSegment;

// =============================================================================
// Segment Selectors  
//
// A "selector" is a 16-bit value that points to a GDT entry.
// Format: | Index (13 bits) | TI (1 bit) | RPL (2 bits) |
//         - Index: Which GDT entry (multiply by 8 for byte offset)
//         - TI (Table Indicator): 0 = GDT, 1 = LDT
//         - RPL (Requested Privilege Level): 0-3
//
// So 0x08 = index 1, GDT, ring 0 -> Kernel Code
//    0x10 = index 2, GDT, ring 0 -> Kernel Data
//    0x1B = index 3, GDT, ring 3 -> User Data
// =============================================================================

/// Segment selector for the kernel code segment (Ring 0)
pub const KERNEL_CODE_SELECTOR: u16 = 0x08;

/// Segment selector for the kernel data segment (Ring 0)
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;

/// Segment selector for the user data segment (Ring 3)
/// Note: User data comes before user code for syscall/sysret compatibility
pub const USER_DATA_SELECTOR: u16 = 0x18 | 3; // RPL = 3

/// Segment selector for the user code segment (Ring 3)
pub const USER_CODE_SELECTOR: u16 = 0x20 | 3; // RPL = 3

/// Segment selector for the TSS
pub const TSS_SELECTOR: u16 = 0x28;

// =============================================================================
// GDT Entry Structure
// =============================================================================

/// A single GDT entry (8 bytes for regular segments)
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct GdtEntry {
    /// Lower 16 bits of the limit
    pub limit_low: u16,
    /// Lower 16 bits of the base address
    pub base_low: u16,
    /// Middle 8 bits of the base address
    pub base_middle: u8,
    /// Access byte (P, DPL, S, Type)
    pub access: u8,
    /// Flags (G, DB, L) and upper 4 bits of limit
    pub flags_limit_high: u8,
    /// Upper 8 bits of the base address
    pub base_high: u8,
}

impl GdtEntry {
    /// Create a null GDT entry
    pub const fn null() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            flags_limit_high: 0,
            base_high: 0,
        }
    }

    /// Create a new GDT entry with the specified parameters
    ///
    /// # Arguments
    /// * `base` - Base address (ignored in 64-bit mode for code/data segments)
    /// * `limit` - Segment limit (ignored in 64-bit mode for code/data segments)
    /// * `access` - Access byte
    /// * `flags` - Flags (upper 4 bits)
    pub const fn new(base: u32, limit: u32, access: u8, flags: u8) -> Self {
        GdtEntry {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access,
            flags_limit_high: ((limit >> 16) & 0x0F) as u8 | ((flags & 0x0F) << 4),
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }

    /// Create a kernel code segment (64-bit, Ring 0)
    pub const fn kernel_code_segment() -> Self {
        // Access: Present | DPL 0 | Code/Data | Executable | Readable
        // 0b10011010 = 0x9A
        // Flags: Long mode (L=1, D=0)
        // 0b0010 = 0x2 (shifted to upper nibble becomes 0xA when combined with granularity)
        GdtEntry::new(0, 0xFFFFF, 0x9A, 0x0A)
    }

    /// Create a kernel data segment (64-bit, Ring 0)
    pub const fn kernel_data_segment() -> Self {
        // Access: Present | DPL 0 | Code/Data | Writable
        // 0b10010010 = 0x92
        // Flags: 32-bit (for compatibility), page granularity
        // 0b1100 = 0xC
        GdtEntry::new(0, 0xFFFFF, 0x92, 0x0C)
    }

    /// Create a user code segment (64-bit, Ring 3)
    pub const fn user_code_segment() -> Self {
        // Access: Present | DPL 3 | Code/Data | Executable | Readable
        // 0b11111010 = 0xFA
        // Flags: Long mode (L=1, D=0)
        GdtEntry::new(0, 0xFFFFF, 0xFA, 0x0A)
    }

    /// Create a user data segment (64-bit, Ring 3)
    pub const fn user_data_segment() -> Self {
        // Access: Present | DPL 3 | Code/Data | Writable
        // 0b11110010 = 0xF2
        // Flags: 32-bit, page granularity
        GdtEntry::new(0, 0xFFFFF, 0xF2, 0x0C)
    }
}

// =============================================================================
// TSS Entry Structure (16 bytes in 64-bit mode)
// =============================================================================

/// TSS descriptor entry (16 bytes in 64-bit mode)
/// System segment descriptors are 16 bytes in long mode
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TssEntry {
    /// Lower part (same as regular GDT entry)
    pub low: GdtEntry,
    /// Upper 32 bits of base address
    pub base_upper: u32,
    /// Reserved (must be zero)
    pub reserved: u32,
}

impl TssEntry {
    /// Create a null TSS entry
    pub const fn null() -> Self {
        TssEntry {
            low: GdtEntry::null(),
            base_upper: 0,
            reserved: 0,
        }
    }

    /// Create a TSS descriptor for the given TSS address
    pub fn new(tss_addr: u64, tss_size: u32) -> Self {
        let base = tss_addr;
        let limit = tss_size - 1;

        // Access byte for 64-bit TSS (Available): 0x89
        // P=1, DPL=0, Type=0x9 (64-bit TSS Available)
        let access: u8 = 0x89;

        // Flags: Granularity=0 (byte), others=0
        let flags: u8 = 0x00;

        TssEntry {
            low: GdtEntry {
                limit_low: (limit & 0xFFFF) as u16,
                base_low: (base & 0xFFFF) as u16,
                base_middle: ((base >> 16) & 0xFF) as u8,
                access,
                flags_limit_high: ((limit >> 16) & 0x0F) as u8 | ((flags & 0x0F) << 4),
                base_high: ((base >> 24) & 0xFF) as u8,
            },
            base_upper: ((base >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }
}

// =============================================================================
// GDT Pointer Structure
// =============================================================================

/// GDTR - GDT Register structure
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct GdtPointer {
    /// Size of GDT minus 1
    pub limit: u16,
    /// Linear address of GDT
    pub base: u64,
}

// =============================================================================
// Global Descriptor Table
// =============================================================================

/// The Global Descriptor Table
/// 
/// Layout:
/// - Entry 0: Null descriptor
/// - Entry 1: Kernel code segment (0x08)
/// - Entry 2: Kernel data segment (0x10)
/// - Entry 3: User data segment (0x18)
/// - Entry 4: User code segment (0x20)
/// - Entry 5-6: TSS descriptor (0x28) - takes 16 bytes
#[repr(C, packed)]
pub struct Gdt {
    pub null: GdtEntry,
    pub kernel_code: GdtEntry,
    pub kernel_data: GdtEntry,
    pub user_data: GdtEntry,
    pub user_code: GdtEntry,
    pub tss: TssEntry,
}

impl Gdt {
    /// Create a new GDT with default segments
    pub const fn new() -> Self {
        Gdt {
            null: GdtEntry::null(),
            kernel_code: GdtEntry::kernel_code_segment(),
            kernel_data: GdtEntry::kernel_data_segment(),
            user_data: GdtEntry::user_data_segment(),
            user_code: GdtEntry::user_code_segment(),
            tss: TssEntry::null(),
        }
    }

    /// Set the TSS entry
    pub fn set_tss(&mut self, tss_addr: u64) {
        self.tss = TssEntry::new(tss_addr, size_of::<TaskStateSegment>() as u32);
    }

    /// Get the GDT pointer for loading with LGDT
    pub fn pointer(&self) -> GdtPointer {
        GdtPointer {
            limit: (size_of::<Gdt>() - 1) as u16,
            base: self as *const _ as u64,
        }
    }
}

// =============================================================================
// Global State
// =============================================================================

/// Static GDT instance
static mut GDT: Gdt = Gdt::new();

/// Static TSS instance
static mut TSS: TaskStateSegment = TaskStateSegment::new();

/// Static GDT pointer
static mut GDT_POINTER: GdtPointer = GdtPointer { limit: 0, base: 0 };

// =============================================================================
// External Assembly Functions
// =============================================================================

extern "C" {
    fn gdt_load(gdt_ptr: *const GdtPointer);
    fn tss_load(selector: u16);
    fn reload_segments();
}

// =============================================================================
// Public API
// =============================================================================

/// Initialize the GDT with TSS
///
/// This function:
/// 1. Sets up the TSS with appropriate stack pointers
/// 2. Updates the GDT's TSS entry
/// 3. Loads the new GDT
/// 4. Loads the TSS
///
/// # Safety
/// Must be called only once during kernel initialization
#[allow(static_mut_refs)]
pub unsafe fn init() {
    // Set up the TSS with kernel stack for ring 0
    // RSP0 is used when transitioning from ring 3 to ring 0
    TSS.rsp0 = 0x900000; // Kernel stack pointer
    
    // Set up Interrupt Stack Table entries for critical interrupts
    // IST1: Double Fault stack
    TSS.ist1 = 0x8F0000;
    // IST2: NMI stack
    TSS.ist2 = 0x8E0000;
    // IST3: Machine Check stack
    TSS.ist3 = 0x8D0000;

    // Set I/O Permission Bitmap offset (no IOPB = size of TSS)
    TSS.iopb_offset = size_of::<TaskStateSegment>() as u16;

    // Update GDT with TSS entry
    let tss_addr = &raw const TSS as u64;
    GDT.set_tss(tss_addr);

    // Create GDT pointer
    GDT_POINTER = GDT.pointer();

    // Load GDT
    gdt_load(&raw const GDT_POINTER);

    // Reload segment registers
    reload_segments();

    // Load TSS
    tss_load(TSS_SELECTOR);
}

/// Get the kernel code selector
pub fn kernel_code_selector() -> u16 {
    KERNEL_CODE_SELECTOR
}

/// Get the kernel data selector
pub fn kernel_data_selector() -> u16 {
    KERNEL_DATA_SELECTOR
}

/// Get the user code selector
pub fn user_code_selector() -> u16 {
    USER_CODE_SELECTOR
}

/// Get the user data selector
pub fn user_data_selector() -> u16 {
    USER_DATA_SELECTOR
}

/// Set the kernel stack pointer in the TSS
/// This should be called on context switch to set up the proper
/// kernel stack for the next process
///
/// # Safety
/// Must be called with a valid stack pointer
pub unsafe fn set_kernel_stack(stack_ptr: u64) {
    TSS.rsp0 = stack_ptr;
}

/// Get a raw pointer to the TSS
pub fn get_tss_ptr() -> *const TaskStateSegment {
    unsafe { &raw const TSS }
}

/// Get a mutable raw pointer to the TSS
///
/// # Safety
/// Caller must ensure exclusive access
pub unsafe fn get_tss_mut_ptr() -> *mut TaskStateSegment {
    &raw mut TSS
}
