//! Nafuraito Kernel - Memory Library
//!
//! Memory management functionality for the kernel:
//! - E820 memory map parsing (BIOS memory detection)
//! - Physical memory management
//! - Virtual memory management (planned)
//! - Heap allocation (planned)

#![crate_type = "rlib"]
#![crate_name = "memory"]
#![no_std]
#![allow(unused)]

// =============================================================================
// E820 Memory Map - imported from e820.rs
// =============================================================================

mod e820;
pub use e820::{
    E820Type, E820Entry, E820EntryExtended, MemoryMap, 
    MEMORY_MAP_ADDR, MAX_MEMORY_ENTRIES,
    MemoryZone, NumaNode,
    DMA_ZONE_END, DMA32_ZONE_END, MAX_NUMA_NODES,
};

// =============================================================================
// Global State
// =============================================================================

pub(crate) struct MemoryMapStorage {
    pub map: MemoryMap,
    pub initialized: bool,
}

pub(crate) static mut MEMORY_MAP_STORAGE: MemoryMapStorage = MemoryMapStorage {
    map: MemoryMap { entries_ptr: core::ptr::null(), count: 0 },
    initialized: false,
};

// =============================================================================
// Public API
// =============================================================================

/// Initialize the global memory map
#[no_mangle]
pub unsafe fn memory_init(memory_map_addr: u64) {
    MEMORY_MAP_STORAGE.map = MemoryMap::parse(memory_map_addr);
    MEMORY_MAP_STORAGE.initialized = true;
}

/// Get a reference to the global memory map
#[no_mangle]
#[allow(static_mut_refs)]
pub fn memory_get() -> Option<&'static MemoryMap> {
    unsafe {
        if MEMORY_MAP_STORAGE.initialized {
            Some(&MEMORY_MAP_STORAGE.map)
        } else {
            None
        }
    }
}

/// Get total usable memory in bytes
#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe extern "C" fn memory_get_total_usable() -> u64 {
    if MEMORY_MAP_STORAGE.initialized {
        MEMORY_MAP_STORAGE.map.total_usable_memory()
    } else {
        0
    }
}

/// Get highest usable memory address
#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe extern "C" fn memory_get_highest_address() -> u64 {
    if MEMORY_MAP_STORAGE.initialized {
        MEMORY_MAP_STORAGE.map.highest_usable_address()
    } else {
        0
    }
}

/// Format a byte size as a human-readable string (KB, MB, GB)
#[no_mangle]
pub fn format_size(bytes: u64) -> (u64, &'static str) {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        (bytes / GB, "GB")
    } else if bytes >= MB {
        (bytes / MB, "MB")
    } else if bytes >= KB {
        (bytes / KB, "KB")
    } else {
        (bytes, "B")
    }
}

// =============================================================================
// Memory Error Type
// =============================================================================

/// Memory subsystem error type
#[derive(Debug, Clone, Copy)]
pub struct MemoryError {
    pub code: u32,
    pub message: &'static str,
    pub fatal: bool,
}

// =============================================================================
// Physical Memory Manager (PMM) - imported from pmm.rs
// =============================================================================

mod pmm;
pub use pmm::*;
