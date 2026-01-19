//! Nafuraito Kernel - GDT Library
//!
//! Global Descriptor Table (GDT) Module for x86-64 including:
//! - Kernel code segment (Ring 0)
//! - Kernel data segment (Ring 0)
//! - User code segment (Ring 3)
//! - User data segment (Ring 3)
//! - Task State Segment (TSS)

#![crate_type = "staticlib"]
#![crate_name = "gdt"]
#![no_std]
#![no_main]
#![allow(unused)]

use core::mem::size_of;
use core::panic::PanicInfo;

/// Panic handler - halts the CPU
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("cli; hlt"); }
    }
}

// =============================================================================
// TSS Module (inline to avoid path issues)
// =============================================================================

/// Task State Segment for 64-bit Long Mode
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TaskStateSegment {
    pub reserved0: u32,
    pub rsp0: u64,
    pub rsp1: u64,
    pub rsp2: u64,
    pub reserved1: u64,
    pub ist1: u64,
    pub ist2: u64,
    pub ist3: u64,
    pub ist4: u64,
    pub ist5: u64,
    pub ist6: u64,
    pub ist7: u64,
    pub reserved2: u64,
    pub reserved3: u16,
    pub iopb_offset: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        TaskStateSegment {
            reserved0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved1: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved2: 0,
            reserved3: 0,
            iopb_offset: size_of::<TaskStateSegment>() as u16,
        }
    }

    pub fn set_rsp0(&mut self, stack_ptr: u64) {
        self.rsp0 = stack_ptr;
    }

    pub fn get_rsp0(&self) -> u64 {
        self.rsp0
    }

    pub fn set_ist(&mut self, index: usize, stack_ptr: u64) {
        match index {
            1 => self.ist1 = stack_ptr,
            2 => self.ist2 = stack_ptr,
            3 => self.ist3 = stack_ptr,
            4 => self.ist4 = stack_ptr,
            5 => self.ist5 = stack_ptr,
            6 => self.ist6 = stack_ptr,
            7 => self.ist7 = stack_ptr,
            _ => {}
        }
    }

    pub fn get_ist(&self, index: usize) -> u64 {
        match index {
            1 => self.ist1,
            2 => self.ist2,
            3 => self.ist3,
            4 => self.ist4,
            5 => self.ist5,
            6 => self.ist6,
            7 => self.ist7,
            _ => 0,
        }
    }
}

/// IST index constants
pub const IST_DOUBLE_FAULT: usize = 1;
pub const IST_NMI: usize = 2;
pub const IST_MACHINE_CHECK: usize = 3;
pub const IST_DEBUG: usize = 4;

// =============================================================================
// Segment Selectors
// =============================================================================

pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const USER_DATA_SELECTOR: u16 = 0x18 | 3;
pub const USER_CODE_SELECTOR: u16 = 0x20 | 3;
pub const TSS_SELECTOR: u16 = 0x28;

// =============================================================================
// GDT Entry Structure
// =============================================================================

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct GdtEntry {
    pub limit_low: u16,
    pub base_low: u16,
    pub base_middle: u8,
    pub access: u8,
    pub flags_limit_high: u8,
    pub base_high: u8,
}

impl GdtEntry {
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

    pub const fn kernel_code_segment() -> Self {
        GdtEntry::new(0, 0xFFFFF, 0x9A, 0x0A)
    }

    pub const fn kernel_data_segment() -> Self {
        GdtEntry::new(0, 0xFFFFF, 0x92, 0x0C)
    }

    pub const fn user_code_segment() -> Self {
        GdtEntry::new(0, 0xFFFFF, 0xFA, 0x0A)
    }

    pub const fn user_data_segment() -> Self {
        GdtEntry::new(0, 0xFFFFF, 0xF2, 0x0C)
    }
}

// =============================================================================
// TSS Entry Structure (16 bytes in 64-bit mode)
// =============================================================================

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TssEntry {
    pub low: GdtEntry,
    pub base_upper: u32,
    pub reserved: u32,
}

impl TssEntry {
    pub const fn null() -> Self {
        TssEntry {
            low: GdtEntry::null(),
            base_upper: 0,
            reserved: 0,
        }
    }

    pub fn new(tss_addr: u64, tss_size: u32) -> Self {
        let base = tss_addr;
        let limit = tss_size - 1;
        let access: u8 = 0x89;
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

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct GdtPointer {
    pub limit: u16,
    pub base: u64,
}

// =============================================================================
// Global Descriptor Table
// =============================================================================

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

    pub fn set_tss(&mut self, tss_addr: u64) {
        self.tss = TssEntry::new(tss_addr, size_of::<TaskStateSegment>() as u32);
    }

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

static mut GDT: Gdt = Gdt::new();
static mut TSS: TaskStateSegment = TaskStateSegment::new();
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
#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe fn init() {
    TSS.rsp0 = 0x900000;
    TSS.ist1 = 0x8F0000;
    TSS.ist2 = 0x8E0000;
    TSS.ist3 = 0x8D0000;
    TSS.iopb_offset = size_of::<TaskStateSegment>() as u16;

    let tss_addr = &raw const TSS as u64;
    GDT.set_tss(tss_addr);
    GDT_POINTER = GDT.pointer();

    gdt_load(&raw const GDT_POINTER);
    reload_segments();
    tss_load(TSS_SELECTOR);
}

#[no_mangle]
pub fn kernel_code_selector() -> u16 {
    KERNEL_CODE_SELECTOR
}

#[no_mangle]
pub fn kernel_data_selector() -> u16 {
    KERNEL_DATA_SELECTOR
}

#[no_mangle]
pub fn user_code_selector() -> u16 {
    USER_CODE_SELECTOR
}

#[no_mangle]
pub fn user_data_selector() -> u16 {
    USER_DATA_SELECTOR
}

#[no_mangle]
pub unsafe fn set_kernel_stack(stack_ptr: u64) {
    TSS.rsp0 = stack_ptr;
}

#[no_mangle]
pub fn get_tss_ptr() -> *const TaskStateSegment {
    unsafe { &raw const TSS }
}

#[no_mangle]
pub unsafe fn get_tss_mut_ptr() -> *mut TaskStateSegment {
    &raw mut TSS
}
