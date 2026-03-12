//! Memory Management Module
//!
//! This module provides memory management functionality for the kernel:
//! - E820 memory map parsing (BIOS memory detection)
//! - Physical memory management (PMM)
//! - Virtual memory management (paging)
//! - Copy-on-Write (CoW) implementation
//! - Heap allocation (planned)

#![allow(unused)]

pub mod cow;
pub mod demand_paging;
pub mod e820;
pub mod paging;
pub mod pat;
pub mod pmm;
pub mod slab;
pub mod heap;

// Re-export commonly used types
pub use self::e820::{
    E820Entry, E820EntryExtended, E820Type, MemoryMap, MemoryZone, NumaNode, DMA32_ZONE_END,
    DMA_ZONE_END, MAX_NUMA_NODES,
};
pub use self::pmm::{
    alloc_frame,
    free_frame,
    pmm_alloc_dma32_frame,
    pmm_alloc_dma_frame,
    pmm_alloc_frame,
    // Zone-aware functions
    pmm_alloc_frame_in_zone,
    // NUMA-aware functions
    pmm_alloc_frame_numa,
    // Contiguous allocation
    pmm_alloc_frames_contiguous,
    pmm_free_frame,
    pmm_free_frame_count,
    pmm_free_frames_contiguous,
    pmm_free_memory,
    pmm_init,
    // C-compatible wrappers
    pmm_init_c,
    pmm_is_frame_used,
    pmm_is_initialized,
    pmm_is_memory_critical,
    // Utility functions
    pmm_memory_usage_percent,
    pmm_numa_free_memory,
    pmm_numa_node_count,
    pmm_numa_total_memory,
    pmm_total_memory,
    pmm_used_frame_count,
    pmm_used_memory,
    pmm_zone_free_memory,
    pmm_zone_total_memory,
    AllocFlags,
    NumaStats,
    PhysicalMemoryManager,
    ZoneStats,
};

pub use self::paging::{
    change_page_flags,
    check_la57_support,
    check_nx_support,
    create_page_table,
    disable_la57,
    enable_la57,
    flush_tlb,
    get_current_page_table,
    get_page_flags,
    get_paging_manager,
    identity_map_range,
    invlpg,
    is_la57_enabled,
    map_page,
    map_range,
    // C-compatible wrappers
    paging_check_la57_support,
    paging_check_nx_support,
    paging_flush_tlb,
    paging_get_mode,
    paging_identity_map_range,
    paging_init,
    paging_init_c,
    paging_invlpg,
    paging_is_la57_enabled,
    paging_is_mapped,
    paging_map_page,
    paging_read_cr3,
    paging_translate_address,
    paging_unmap_page,
    read_cr3,
    read_cr4,
    remap_page,
    switch_page_table,
    translate_address,
    unmap_page,
    unmap_range,
    write_cr3,
    write_cr4,
    PageTable,
    PageTableEntry,
    PageTableFlags,
    PagingManager,
    PagingMode,
    PhysAddr,
    VirtAddr,
    LA48_VIRT_ADDR_BITS,
    LA57_VIRT_ADDR_BITS,
    // Constants
    PAGE_SIZE,
    PAGE_SIZE_1GB,
    PAGE_SIZE_2MB,
    PAGE_TABLE_ENTRIES,
};

pub use self::pat::{
    check_pat_support,
    flags_normal_memory,
    flags_uncacheable,
    flags_write_combining,
    flags_write_protected,
    flags_write_through,
    get_pat_manager,
    is_pat_initialized,
    // Page table helper functions
    memory_type_to_flags,
    // C-compatible wrappers
    pat_check_support,
    pat_get_entry,
    pat_init,
    pat_init_c,
    pat_is_initialized,
    pat_read_msr,
    read_pat,
    MemoryType,
    PatConfig,
    PatManager,
    // Constants
    IA32_PAT_MSR,
};

pub use self::cow::{
    clone_range_cow,
    cow_clone_range,
    cow_get_refcount,
    cow_handle_fault,
    // C-compatible wrappers
    cow_init_c,
    cow_mark_page,
    cow_share_page,
    dec_refcount,
    get_refcount,
    handle_cow_fault,
    inc_refcount,
    init as cow_init,
    is_cow_page,
    mark_cow,
    mark_cow_range,
    set_refcount,
    share_page_cow,
    unshare_cow_page,
    COW_BIT,
};

pub use self::demand_paging::{
    handle_demand_page_fault, mmap, mmap_file, munmap,
    register_file_region as demand_register_file_region, register_region as demand_register_region,
    BackingKind, FileBacking, MmapReadAtFn, VmRegion,
};

pub use self::heap::{
    heap_alloc,
    heap_alloc_c,
    heap_dealloc,
    heap_dealloc_c,
    heap_init,
    heap_init_c,
    heap_is_initialized,
    heap_is_initialized_c,
};

/// Memory subsystem error type
#[derive(Debug, Clone, Copy)]
pub struct MemoryError {
    pub code: u32,
    pub message: &'static str,
    pub fatal: bool,
}
