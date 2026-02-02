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
pub mod paging;

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
    // C-compatible wrappers
    paging_check_la57_support,
    paging_check_nx_support,
    paging_is_la57_enabled,
    paging_read_cr3,
    paging_init_c,
    paging_get_mode,
    paging_invlpg,
    paging_flush_tlb,
    // Constants
    PAGE_SIZE, PAGE_SIZE_2MB, PAGE_SIZE_1GB,
    PAGE_TABLE_ENTRIES,
    LA48_VIRT_ADDR_BITS, LA57_VIRT_ADDR_BITS,
};

/// Memory subsystem error type
#[derive(Debug, Clone, Copy)]
pub struct MemoryError {
    pub code: u32,
    pub message: &'static str,
    pub fatal: bool,
}
