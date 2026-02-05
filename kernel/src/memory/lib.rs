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

impl MemoryError {
    /// Create a new memory error
    pub const fn new(code: u32, message: &'static str, fatal: bool) -> Self {
        Self { code, message, fatal }
    }
}

impl From<MemoryError> for &'static str {
    fn from(error: MemoryError) -> Self {
        error.message
    }
}

// =============================================================================
// Physical Memory Manager (PMM) - imported from pmm.rs
// =============================================================================

mod pmm;
pub use pmm::{
    PhysicalMemoryManager, ZoneStats, NumaStats, AllocFlags,
    alloc_frame, free_frame, pmm_init,
    pmm_init_c, pmm_alloc_frame, pmm_free_frame,
    pmm_free_memory, pmm_used_memory, pmm_total_memory,
    pmm_free_frame_count, pmm_used_frame_count,
    pmm_is_initialized, pmm_is_frame_used,
    pmm_alloc_frame_in_zone, pmm_alloc_dma_frame, pmm_alloc_dma32_frame,
    pmm_zone_free_memory, pmm_zone_total_memory,
    pmm_alloc_frame_numa, pmm_numa_node_count,
    pmm_numa_free_memory, pmm_numa_total_memory,
    pmm_alloc_frames_contiguous, pmm_free_frames_contiguous,
    pmm_memory_usage_percent, pmm_is_memory_critical,
};

// =============================================================================
// Paging (Virtual Memory) - imported from paging.rs
// =============================================================================

mod paging;
pub use paging::{
    PageTableFlags, PageTableEntry, PageTable,
    PagingMode, PagingManager, VirtAddr, PhysAddr,
    check_la57_support, check_nx_support,
    is_la57_enabled, enable_la57, disable_la57,
    read_cr3, write_cr3, read_cr4, write_cr4,
    invlpg, flush_tlb,
    paging_init, get_paging_manager,
    map_page, unmap_page, translate_address,
    identity_map_range, get_current_page_table,
    switch_page_table, create_page_table,
    map_range, unmap_range, remap_page, change_page_flags,
    paging_check_la57_support, paging_check_nx_support,
    paging_is_la57_enabled, paging_read_cr3,
    paging_init_c, paging_get_mode,
    paging_invlpg, paging_flush_tlb,
    paging_test_simple,
    paging_map_page, paging_unmap_page,
    paging_translate_address, paging_is_mapped,
    paging_identity_map_range,
    PAGE_SIZE_2MB, PAGE_SIZE_1GB, PAGE_TABLE_ENTRIES,
    LA48_VIRT_ADDR_BITS, LA57_VIRT_ADDR_BITS,
};

// Re-export PAGE_SIZE from pmm (it's the standard 4KB page size)
pub use pmm::PAGE_SIZE;

// =============================================================================
// PAT (Page Attribute Table) - imported from pat.rs
// =============================================================================

mod pat;
pub use pat::{
    MemoryType, PatConfig, PatManager,
    check_pat_support, pat_init, get_pat_manager,
    is_pat_initialized, read_pat,
    pat_check_support, pat_init_c, pat_is_initialized,
    pat_get_entry, pat_read_msr,
    memory_type_to_flags,
    flags_normal_memory, flags_write_through,
    flags_uncacheable, flags_write_combining,
    flags_write_protected,
    IA32_PAT_MSR,
};

// =============================================================================
//  CoW (Copy-on-Write) - imported from cow.rs
// =============================================================================
mod cow;
pub use cow::{
    COW_BIT,
    init as cow_init,
    get_refcount, inc_refcount, dec_refcount, set_refcount,
    is_cow_page, mark_cow, mark_cow_range,
    handle_cow_fault,
    share_page_cow, clone_range_cow, unshare_cow_page,
    // C-compatible wrappers
    cow_init as cow_init_c,
    cow_mark_page, cow_handle_fault,
    cow_get_refcount, cow_share_page, cow_clone_range,
};
