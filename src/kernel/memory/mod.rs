//! Memory Management Module
//!
//! This module provides memory management functionality for the kernel:
//! - E820 memory map parsing (BIOS memory detection)
//! - Physical memory management (PMM)
//! - Virtual memory management (planned)
//! - Heap allocation (planned)

#![allow(unused)]

pub mod e820;
pub mod pmm;

// Re-export commonly used types
pub use self::e820::{E820Entry, E820Type, MemoryMap};
pub use self::pmm::{
    PhysicalMemoryManager, 
    alloc_frame, 
    free_frame, 
    pmm_init,
    // C-compatible wrappers
    pmm_init_c,
    pmm_alloc_frame,
    pmm_free_frame,
    pmm_free_memory,
    pmm_used_memory,
    pmm_total_memory,
    pmm_free_frame_count,
    pmm_used_frame_count,
    pmm_is_initialized,
    pmm_is_frame_used,
};

/// Memory subsystem error type
#[derive(Debug, Clone, Copy)]
pub struct MemoryError {
    pub code: u32,
    pub message: &'static str,
    pub fatal: bool,
}
