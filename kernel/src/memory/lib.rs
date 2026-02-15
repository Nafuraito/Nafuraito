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
    E820Entry, E820EntryExtended, E820Type, MemoryMap, MemoryZone, NumaNode, DMA32_ZONE_END,
    DMA_ZONE_END, MAX_MEMORY_ENTRIES, MAX_NUMA_NODES, MEMORY_MAP_ADDR,
};

// =============================================================================
// Global State
// =============================================================================

pub(crate) struct MemoryMapStorage {
    pub map: MemoryMap,
    pub initialized: bool,
}

pub(crate) static mut MEMORY_MAP_STORAGE: MemoryMapStorage = MemoryMapStorage {
    map: MemoryMap {
        entries_ptr: core::ptr::null(),
        count: 0,
    },
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
        Self {
            code,
            message,
            fatal,
        }
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
    alloc_frame, free_frame, pmm_alloc_dma32_frame, pmm_alloc_dma_frame, pmm_alloc_frame,
    pmm_alloc_frame_in_zone, pmm_alloc_frame_numa, pmm_alloc_frames_contiguous, pmm_free_frame,
    pmm_free_frame_count, pmm_free_frames_contiguous, pmm_free_memory, pmm_init, pmm_init_c,
    pmm_is_frame_used, pmm_is_initialized, pmm_is_memory_critical, pmm_memory_usage_percent,
    pmm_numa_free_memory, pmm_numa_node_count, pmm_numa_total_memory, pmm_total_memory,
    pmm_used_frame_count, pmm_used_memory, pmm_zone_free_memory, pmm_zone_total_memory, AllocFlags,
    NumaStats, PhysicalMemoryManager, ZoneStats,
};

// =============================================================================
// Paging (Virtual Memory) - imported from paging.rs
// =============================================================================

mod paging;
pub use paging::{
    change_page_flags, check_la57_support, check_nx_support, create_page_table, disable_la57,
    enable_la57, flush_tlb, get_current_page_table, get_paging_manager, identity_map_range, invlpg,
    is_la57_enabled, map_page, map_range, paging_check_la57_support, paging_check_nx_support,
    paging_flush_tlb, paging_get_mode, paging_identity_map_range, paging_init, paging_init_c,
    paging_invlpg, paging_is_la57_enabled, paging_is_mapped, paging_map_page, paging_read_cr3,
    paging_test_simple, paging_translate_address, paging_unmap_page, read_cr3, read_cr4,
    remap_page, switch_page_table, translate_address, unmap_page, unmap_range, write_cr3,
    write_cr4, PageTable, PageTableEntry, PageTableFlags, PagingManager, PagingMode, PhysAddr,
    VirtAddr, LA48_VIRT_ADDR_BITS, LA57_VIRT_ADDR_BITS, PAGE_SIZE_1GB, PAGE_SIZE_2MB,
    PAGE_TABLE_ENTRIES,
};

// Re-export PAGE_SIZE from pmm (it's the standard 4KB page size)
pub use pmm::PAGE_SIZE;

// =============================================================================
// PAT (Page Attribute Table) - imported from pat.rs
// =============================================================================

mod pat;
pub use pat::{
    check_pat_support, flags_normal_memory, flags_uncacheable, flags_write_combining,
    flags_write_protected, flags_write_through, get_pat_manager, is_pat_initialized,
    memory_type_to_flags, pat_check_support, pat_get_entry, pat_init, pat_init_c,
    pat_is_initialized, pat_read_msr, read_pat, MemoryType, PatConfig, PatManager, IA32_PAT_MSR,
};

// =============================================================================
//  CoW (Copy-on-Write) - imported from cow.rs
// =============================================================================
mod cow;
pub use cow::{
    clone_range_cow,
    cow_clone_range,
    cow_get_refcount,
    cow_handle_fault,
    // C-compatible wrappers
    cow_init as cow_init_c,
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

// =============================================================================
// Demand Paging - imported from demand_paging.rs
// =============================================================================

mod demand_paging;
pub use demand_paging::{
    handle_demand_page_fault, mmap, mmap_file, munmap,
    register_file_region as demand_register_file_region, register_region as demand_register_region,
    BackingKind, FileBacking, MmapReadAtFn, VmRegion,
};

// =============================================================================
// C-Compatible mmap/munmap wrappers
// =============================================================================

/// Create a demand-paged anonymous mapping (C-compatible).
///
/// Returns the mapped base address on success, or 0 on failure.
#[no_mangle]
pub extern "C" fn memory_mmap_zero(start: u64, size: usize, flags: u64) -> u64 {
    let virt = VirtAddr::new(start);
    let map_flags = PageTableFlags::from_raw(flags);

    match mmap(virt, size, map_flags, BackingKind::ZeroFill) {
        Ok(mapped) => mapped.as_u64(),
        Err(_) => 0,
    }
}

/// Create a demand-paged file mapping (C-compatible).
///
/// Returns the mapped base address on success, or 0 on failure.
#[no_mangle]
pub extern "C" fn memory_mmap_file(
    start: u64,
    size: usize,
    flags: u64,
    ctx: *mut u8,
    read_at: MmapReadAtFn,
    file_offset: u64,
    file_size: u64,
) -> u64 {
    let virt = VirtAddr::new(start);
    let map_flags = PageTableFlags::from_raw(flags);
    let backing = FileBacking {
        ctx,
        read_at,
        file_offset,
        file_size,
    };

    match mmap_file(virt, size, map_flags, backing) {
        Ok(mapped) => mapped.as_u64(),
        Err(_) => 0,
    }
}

/// Unmap a demand-paged region (C-compatible).
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub extern "C" fn memory_munmap(start: u64, size: usize) -> i32 {
    let virt = VirtAddr::new(start);
    match munmap(virt, size) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}
