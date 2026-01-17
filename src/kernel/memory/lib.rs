//! Nafuraito Kernel - Memory Library
//!
//! Memory management functionality for the kernel:
//! - E820 memory map parsing (BIOS memory detection)
//! - Physical memory management
//! - Virtual memory management (planned)
//! - Heap allocation (planned)

#![crate_type = "staticlib"]
#![crate_name = "memory"]
#![no_std]
#![no_main]
#![allow(unused)]

use core::panic::PanicInfo;

/// Panic handler - halts the CPU
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("cli; hlt"); }
    }
}

// =============================================================================
// Constants
// =============================================================================

/// Memory map storage address (set by bootloader)
pub const MEMORY_MAP_ADDR: usize = 0x6000;

/// Maximum number of memory map entries
pub const MAX_MEMORY_ENTRIES: usize = 64;

// =============================================================================
// E820 Memory Types
// =============================================================================

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E820Type {
    Usable = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNvs = 4,
    BadMemory = 5,
    Unknown = 0xFF,
}

impl E820Type {
    pub fn from_u32(val: u32) -> Self {
        match val {
            1 => E820Type::Usable,
            2 => E820Type::Reserved,
            3 => E820Type::AcpiReclaimable,
            4 => E820Type::AcpiNvs,
            5 => E820Type::BadMemory,
            _ => E820Type::Unknown,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            E820Type::Usable => "Usable",
            E820Type::Reserved => "Reserved",
            E820Type::AcpiReclaimable => "ACPI Reclaimable",
            E820Type::AcpiNvs => "ACPI NVS",
            E820Type::BadMemory => "Bad Memory",
            E820Type::Unknown => "Unknown",
        }
    }

    pub fn is_usable(&self) -> bool {
        *self == E820Type::Usable
    }
}

// =============================================================================
// E820 Entry
// =============================================================================

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct E820Entry {
    pub base: u64,
    pub length: u64,
    pub region_type: u32,
    pub acpi_extended: u32,
}

impl E820Entry {
    pub fn end(&self) -> u64 {
        self.base.saturating_add(self.length)
    }

    pub fn memory_type(&self) -> E820Type {
        E820Type::from_u32(self.region_type)
    }

    pub fn is_valid(&self) -> bool {
        self.length > 0 && (self.acpi_extended & 1 != 0 || self.acpi_extended == 0)
    }

    pub fn is_usable(&self) -> bool {
        self.is_valid() && self.memory_type().is_usable()
    }

    pub fn overlaps(&self, start: u64, end: u64) -> bool {
        self.base < end && self.end() > start
    }

    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.end()
    }
}

// =============================================================================
// Memory Map
// =============================================================================

pub struct MemoryMap {
    entries_ptr: *const E820Entry,
    count: usize,
}

impl MemoryMap {
    pub unsafe fn parse(memory_map_addr: u64) -> Self {
        let addr = memory_map_addr as usize;
        let count_ptr = addr as *const u16;
        let count = core::ptr::read_unaligned(count_ptr) as usize;
        let entries_ptr = addr.wrapping_add(4) as *const E820Entry;
        let safe_count = if count > MAX_MEMORY_ENTRIES { MAX_MEMORY_ENTRIES } else { count };
        
        MemoryMap {
            entries_ptr,
            count: safe_count,
        }
    }

    pub fn entry_count(&self) -> usize {
        self.count
    }

    pub fn get(&self, index: usize) -> Option<E820Entry> {
        if index < self.count {
            unsafe {
                let ptr = self.entries_ptr.wrapping_add(index);
                Some(core::ptr::read_unaligned(ptr))
            }
        } else {
            None
        }
    }

    pub fn entries_raw(&self) -> (*const E820Entry, usize) {
        (self.entries_ptr, self.count)
    }

    pub fn total_usable_memory(&self) -> u64 {
        let mut total: u64 = 0;
        let mut i: usize = 0;
        while i < self.count {
            if let Some(entry) = self.get(i) {
                if entry.is_usable() {
                    total = total.wrapping_add(entry.length);
                }
            }
            i = i.wrapping_add(1);
        }
        total
    }

    pub fn total_memory(&self) -> u64 {
        let mut total: u64 = 0;
        let mut i: usize = 0;
        while i < self.count {
            if let Some(entry) = self.get(i) {
                if entry.is_valid() {
                    total = total.wrapping_add(entry.length);
                }
            }
            i = i.wrapping_add(1);
        }
        total
    }

    pub fn largest_usable_region(&self) -> Option<E820Entry> {
        let mut largest: Option<E820Entry> = None;
        let mut max_len: u64 = 0;
        let mut i: usize = 0;
        while i < self.count {
            if let Some(entry) = self.get(i) {
                if entry.is_usable() && entry.length > max_len {
                    max_len = entry.length;
                    largest = Some(entry);
                }
            }
            i = i.wrapping_add(1);
        }
        largest
    }

    pub fn find_usable_region_containing(&self, addr: u64) -> Option<E820Entry> {
        let mut i: usize = 0;
        while i < self.count {
            if let Some(entry) = self.get(i) {
                if entry.is_usable() && entry.contains(addr) {
                    return Some(entry);
                }
            }
            i = i.wrapping_add(1);
        }
        None
    }

    pub fn highest_usable_address(&self) -> u64 {
        let mut highest: u64 = 0;
        let mut i: usize = 0;
        while i < self.count {
            if let Some(entry) = self.get(i) {
                if entry.is_usable() {
                    let end = entry.end();
                    if end > highest {
                        highest = end;
                    }
                }
            }
            i = i.wrapping_add(1);
        }
        highest
    }

    pub fn usable_region_count(&self) -> usize {
        let mut count: usize = 0;
        let mut i: usize = 0;
        while i < self.count {
            if let Some(entry) = self.get(i) {
                if entry.is_usable() {
                    count = count.wrapping_add(1);
                }
            }
            i = i.wrapping_add(1);
        }
        count
    }

    pub fn valid_entry_count(&self) -> usize {
        let mut count: usize = 0;
        let mut i: usize = 0;
        while i < self.count {
            if let Some(entry) = self.get(i) {
                if entry.is_valid() {
                    count = count.wrapping_add(1);
                }
            }
            i = i.wrapping_add(1);
        }
        count
    }
}

// =============================================================================
// Global State
// =============================================================================

struct MemoryMapStorage {
    map: MemoryMap,
    initialized: bool,
}

static mut MEMORY_MAP_STORAGE: MemoryMapStorage = MemoryMapStorage {
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
