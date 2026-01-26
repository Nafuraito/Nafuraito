//! E820 Memory Map Parser
//!
//! This module provides functionality to parse the BIOS E820 memory map
//! that is collected by the bootloader in Real Mode.
//!
//! The E820 memory map tells us which regions of physical memory are:
//! - Usable RAM (Type 1)
//! - Reserved by hardware (Type 2)
//! - ACPI Reclaimable (Type 3)
//! - ACPI NVS - Non-Volatile Storage (Type 4)
//! - Bad/Defective memory (Type 5)

#![allow(unused)]

/// Memory map storage address (set by bootloader)
pub const MEMORY_MAP_ADDR: usize = 0x6000;

/// Maximum number of memory map entries
pub const MAX_MEMORY_ENTRIES: usize = 64;

/// E820 Memory Region Types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E820Type {
    /// Usable RAM - can be freely used by the OS
    Usable = 1,
    /// Reserved - do not use this memory
    Reserved = 2,
    /// ACPI Reclaimable - can be reclaimed after reading ACPI tables
    AcpiReclaimable = 3,
    /// ACPI NVS - Non-Volatile Storage, must be preserved
    AcpiNvs = 4,
    /// Bad Memory - defective, do not use
    BadMemory = 5,
    /// Unknown type
    Unknown = 0xFF,
}

impl E820Type {
    /// Convert from raw u32 value
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

    /// Get a human-readable name for this memory type
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

    /// Check if this memory type represents usable RAM
    pub fn is_usable(&self) -> bool {
        *self == E820Type::Usable
    }
}

/// E820 Memory Map Entry (24 bytes)
///
/// Each entry describes a contiguous region of physical memory.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct E820Entry {
    /// Base address of the memory region
    pub base: u64,
    /// Length of the memory region in bytes
    pub length: u64,
    /// Type of memory region (see E820Type)
    pub region_type: u32,
    /// Extended attributes (ACPI 3.0+)
    /// Bit 0: If clear, entire entry should be ignored
    /// Bit 1: If set, memory is non-volatile
    pub acpi_extended: u32,
}

impl E820Entry {
    /// Get the end address of this memory region (exclusive)
    pub fn end(&self) -> u64 {
        self.base.saturating_add(self.length)
    }

    /// Get the type of this memory region
    pub fn memory_type(&self) -> E820Type {
        E820Type::from_u32(self.region_type)
    }

    /// Check if this entry is valid (non-zero length and enabled)
    pub fn is_valid(&self) -> bool {
        self.length > 0 && (self.acpi_extended & 1 != 0 || self.acpi_extended == 0)
    }

    /// Check if this region contains usable RAM
    pub fn is_usable(&self) -> bool {
        self.is_valid() && self.memory_type().is_usable()
    }

    /// Check if this region overlaps with a given range
    pub fn overlaps(&self, start: u64, end: u64) -> bool {
        self.base < end && self.end() > start
    }

    /// Check if this region contains a given address
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.end()
    }
}

/// E820 Memory Map
///
/// Represents the complete memory map collected by the bootloader.
pub struct MemoryMap {
    /// Pointer to the memory map entries
    pub(crate) entries_ptr: *const E820Entry,
    /// Number of entries in the map
    pub(crate) count: usize,
}

impl MemoryMap {
    /// Parse the E820 memory map from bootloader format
    /// 
    /// The bootloader passes entries directly at memory_map_addr (24 bytes per entry).
    /// We scan entries until we find an invalid one (type not 1-5 or length 0).
    ///
    /// # Safety
    /// This function reads from a fixed memory address that must have been
    /// populated by the bootloader with valid E820 data.
    pub unsafe fn parse(memory_map_addr: u64) -> Self {
        let entries_ptr = memory_map_addr as *const E820Entry;
        
        // Scan for valid entries - stop at invalid entry or max count
        let mut count = 0usize;
        
        while count < MAX_MEMORY_ENTRIES {
            let entry_ptr = entries_ptr.wrapping_add(count);
            let entry = core::ptr::read_unaligned(entry_ptr);
            
            let region_type = entry.region_type;
            let length = entry.length;
            
            // Valid E820 types are 1-5
            if region_type < 1 || region_type > 5 {
                break;
            }
            if length == 0 {
                break;
            }
            count += 1;
        }
        
        MemoryMap {
            entries_ptr,
            count,
        }
    }

    /// Get the number of entries in the memory map
    pub fn entry_count(&self) -> usize {
        self.count
    }

    /// Get a specific entry by index
    pub fn get(&self, index: usize) -> Option<E820Entry> {
        if index < self.count {
            unsafe {
                // Use read_unaligned for packed struct
                let ptr = self.entries_ptr.wrapping_add(index);
                Some(core::ptr::read_unaligned(ptr))
            }
        } else {
            None
        }
    }

    /// Get all entries - returns pointer and count for manual iteration
    pub fn entries_raw(&self) -> (*const E820Entry, usize) {
        (self.entries_ptr, self.count)
    }

    /// Calculate total usable RAM in bytes
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

    /// Calculate total memory (all types) in bytes
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

    /// Find the largest usable memory region
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

    /// Find usable memory region that contains an address
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

    /// Get the highest usable address
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

    /// Count usable memory regions
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

    /// Count valid entries
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

// Global state and utility functions are in lib.rs
