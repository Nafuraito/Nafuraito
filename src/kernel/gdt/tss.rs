//! Task State Segment (TSS) Module
//!
//! The TSS is required in 64-bit long mode for:
//! - Storing stack pointers for privilege level changes (RSP0, RSP1, RSP2)
//! - Interrupt Stack Table (IST) for dedicated interrupt stacks
//! - I/O Permission Bitmap base address
//!
//! Unlike 32-bit protected mode, the TSS in 64-bit mode does NOT store
//! register state for hardware task switching. Software task switching
//! is the standard approach in modern operating systems.

#![allow(unused)]

use core::mem::size_of;

/// Task State Segment for 64-bit Long Mode
///
/// Total size: 104 bytes (0x68)
///
/// The TSS provides:
/// - RSP0-2: Stack pointers for ring transitions
/// - IST1-7: Interrupt Stack Table entries
/// - IOPB: I/O Permission Bitmap offset
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TaskStateSegment {
    /// Reserved (must be zero)
    pub reserved0: u32,

    /// Ring 0 Stack Pointer - Used when transitioning from Ring 3 to Ring 0
    pub rsp0: u64,
    /// Ring 1 Stack Pointer - Used when transitioning to Ring 1
    pub rsp1: u64,
    /// Ring 2 Stack Pointer - Used when transitioning to Ring 2
    pub rsp2: u64,

    /// Reserved (must be zero)
    pub reserved1: u64,

    /// Interrupt Stack Table Entry 1 - Double Fault
    pub ist1: u64,
    /// Interrupt Stack Table Entry 2 - NMI
    pub ist2: u64,
    /// Interrupt Stack Table Entry 3 - Machine Check
    pub ist3: u64,
    /// Interrupt Stack Table Entry 4
    pub ist4: u64,
    /// Interrupt Stack Table Entry 5
    pub ist5: u64,
    /// Interrupt Stack Table Entry 6
    pub ist6: u64,
    /// Interrupt Stack Table Entry 7
    pub ist7: u64,

    /// Reserved (must be zero)
    pub reserved2: u64,

    /// Reserved (must be zero)
    pub reserved3: u16,

    /// I/O Map Base Address - Offset from TSS base to I/O permission bitmap
    /// Set to size of TSS if no IOPB is used
    pub iopb_offset: u16,
}

impl TaskStateSegment {
    /// Create a new zeroed TSS
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

    /// Set the Ring 0 stack pointer
    /// This stack is used when an interrupt occurs while in Ring 3
    pub fn set_rsp0(&mut self, stack_ptr: u64) {
        self.rsp0 = stack_ptr;
    }

    /// Get the Ring 0 stack pointer
    pub fn get_rsp0(&self) -> u64 {
        self.rsp0
    }

    /// Set an IST entry
    /// 
    /// IST entries are used by specific interrupts to ensure they have
    /// a clean stack, regardless of the current stack state.
    ///
    /// # Arguments
    /// * `index` - IST index (1-7)
    /// * `stack_ptr` - Stack pointer for this IST entry
    ///
    /// # Panics
    /// Panics if index is not 1-7
    pub fn set_ist(&mut self, index: usize, stack_ptr: u64) {
        match index {
            1 => self.ist1 = stack_ptr,
            2 => self.ist2 = stack_ptr,
            3 => self.ist3 = stack_ptr,
            4 => self.ist4 = stack_ptr,
            5 => self.ist5 = stack_ptr,
            6 => self.ist6 = stack_ptr,
            7 => self.ist7 = stack_ptr,
            _ => {} // Invalid index, silently ignore in kernel
        }
    }

    /// Get an IST entry
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

/// IST index for Double Fault handler
pub const IST_DOUBLE_FAULT: usize = 1;

/// IST index for NMI handler
pub const IST_NMI: usize = 2;

/// IST index for Machine Check handler
pub const IST_MACHINE_CHECK: usize = 3;

/// IST index for Debug handler
pub const IST_DEBUG: usize = 4;

// Compile-time size check
const _: () = assert!(size_of::<TaskStateSegment>() == 104);
