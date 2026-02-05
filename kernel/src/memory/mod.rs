//! Memory Management Module
//!
//! This module provides memory management functionality for the kernel:
//! - E820 memory map parsing (BIOS memory detection)
//! - Physical memory management (PMM)
//! - Virtual memory management (paging)
//! - Copy-on-Write (CoW) implementation
//! - Heap allocation (planned)

#![allow(unused)]

pub mod e820;
pub mod pmm;
pub mod paging;
pub mod pat;
pub mod cow;

// Re-export commonly used types
pub use self::e820::{
    E820Entry, E820EntryExtended, E820Type, MemoryMap,
    MemoryZone, NumaNode,
    DMA_ZONE_END, DMA32_ZONE_END, MAX_NUMA_NODES,
};
pub use self::pmm::{
    PhysicalMemoryManager, 
    ZoneStats, NumaStats, AllocFlags,
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
    // Zone-aware functions
    pmm_alloc_frame_in_zone,
    pmm_alloc_dma_frame,
    pmm_alloc_dma32_frame,
    pmm_zone_free_memory,
    pmm_zone_total_memory,
    // NUMA-aware functions
    pmm_alloc_frame_numa,
    pmm_numa_node_count,
    pmm_numa_free_memory,
    pmm_numa_total_memory,
    // Contiguous allocation
    pmm_alloc_frames_contiguous,
    pmm_free_frames_contiguous,
    // Utility functions
    pmm_memory_usage_percent,
    pmm_is_memory_critical,
};

pub use self::paging::{
    PageTableFlags, PageTableEntry, PageTable,
    PagingMode, PagingManager, VirtAddr, PhysAddr,
    check_la57_support, check_nx_support,
    is_la57_enabled, enable_la57, disable_la57,
    read_cr3, write_cr3, read_cr4, write_cr4,
    invlpg, flush_tlb,
    paging_init, get_paging_manager,
    map_page, unmap_page, map_range, unmap_range,
    remap_page, translate_address, get_page_flags,
    identity_map_range, change_page_flags,
    get_current_page_table, switch_page_table, create_page_table,
    // C-compatible wrappers
    paging_check_la57_support,
    paging_check_nx_support,
    paging_is_la57_enabled,
    paging_read_cr3,
    paging_init_c,
    paging_get_mode,
    paging_invlpg,
    paging_flush_tlb,
    paging_map_page,
    paging_unmap_page,
    paging_translate_address,
    paging_is_mapped,
    paging_identity_map_range,
    // Constants
    PAGE_SIZE, PAGE_SIZE_2MB, PAGE_SIZE_1GB,
    PAGE_TABLE_ENTRIES,
    LA48_VIRT_ADDR_BITS, LA57_VIRT_ADDR_BITS,
};

pub use self::pat::{
    MemoryType, PatConfig, PatManager,
    check_pat_support, pat_init, get_pat_manager,
    is_pat_initialized, read_pat,
    // C-compatible wrappers
    pat_check_support, pat_init_c, pat_is_initialized,
    pat_get_entry, pat_read_msr,
    // Page table helper functions
    memory_type_to_flags,
    flags_normal_memory, flags_write_through,
    flags_uncacheable, flags_write_combining,
    flags_write_protected,
    // Constants
    IA32_PAT_MSR,
};

pub use self::cow::{
    COW_BIT,
    init as cow_init,
    get_refcount, inc_refcount, dec_refcount, set_refcount,
    is_cow_page, mark_cow, mark_cow_range,
    handle_cow_fault,
    share_page_cow, clone_range_cow, unshare_cow_page,
    // C-compatible wrappers
    cow_init_c,
    cow_mark_page, cow_handle_fault,
    cow_get_refcount, cow_share_page, cow_clone_range,
};

/// Memory subsystem error type
#[derive(Debug, Clone, Copy)]
pub struct MemoryError {
    pub code: u32,
    pub message: &'static str,
    pub fatal: bool,
}
