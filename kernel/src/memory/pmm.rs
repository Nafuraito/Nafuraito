//! Physical Memory Manager (PMM)
//! =============================================================================
//!
//! The PMM (Physical Memory Manager) is like a librarian for RAM. It keeps
//! track of which "pages" (4KB chunks) of physical memory are free and which
//! are in use.
//!
//! How does it work?
//! 
//! It uses a **bitmap** - one bit per page frame (4KB chunk of RAM):
//!   - Bit = 0: Page is free, available for allocation
//!   - Bit = 1: Page is in use (contains kernel code, page tables, data, etc.)
//! 
//! For example, if we have 4GB of RAM:
//!   - 4GB / 4KB = 1,048,576 pages
//!   - 1,048,576 bits = 131,072 bytes = 128KB of bitmap
//! 
//! That's pretty efficient! 128KB to track 4GB of memory.
//!
//! **Memory Zones**:
//! Not all RAM is created equal. Some devices (like old DMA controllers)
//! can only access certain regions of memory:
//!   - DMA Zone (0-16MB): For ancient ISA DMA controllers
//!   - DMA32 Zone (0-4GB): For 32-bit DMA (most modern devices)
//!   - Normal Zone (4GB+): Regular memory
//!   - HighMem Zone: Memory above kernel's direct mapping (not used yet)
//!
//! **NUMA Support**:
//! On multi-socket servers, different CPUs have different "local" RAM.
//! Accessing local RAM is fast, remote RAM is slower. We track this to
//! allocate memory close to the CPU that will use it.
//!
//! =============================================================================

#![allow(unused)]

use super::{E820Entry, MemoryMap, MemoryError, MEMORY_MAP_STORAGE, MemoryZone, NumaNode, DMA_ZONE_END, DMA32_ZONE_END};

pub const PAGE_SIZE: usize = 4096; // 4 KB

/// Allocation flags for memory allocation hints
#[derive(Debug, Clone, Copy)]
pub struct AllocFlags {
    /// Prefer allocation from DMA zone
    pub dma_capable: bool,
    /// Prefer allocation from DMA32 zone (32-bit addressable)
    pub dma32_capable: bool,
    /// Allow allocation from any zone (fallback)
    pub allow_fallback: bool,
    /// Prefer specific NUMA node
    pub numa_node: Option<NumaNode>,
    /// Zero the allocated frame(s)
    pub zero_memory: bool,
}

impl AllocFlags {
    /// Default allocation flags (no preferences)
    pub const fn new() -> Self {
        AllocFlags {
            dma_capable: false,
            dma32_capable: false,
            allow_fallback: true,
            numa_node: None,
            zero_memory: false,
        }
    }

    /// Flags for DMA allocation
    pub const fn dma() -> Self {
        AllocFlags {
            dma_capable: true,
            dma32_capable: false,
            allow_fallback: false,
            numa_node: None,
            zero_memory: false,
        }
    }

    /// Flags for DMA32 allocation
    pub const fn dma32() -> Self {
        AllocFlags {
            dma_capable: false,
            dma32_capable: true,
            allow_fallback: true,
            numa_node: None,
            zero_memory: false,
        }
    }

    /// Flags for NUMA-local allocation
    pub const fn numa(node: NumaNode) -> Self {
        AllocFlags {
            dma_capable: false,
            dma32_capable: false,
            allow_fallback: true,
            numa_node: Some(node),
            zero_memory: false,
        }
    }
}

/// Zone statistics
#[derive(Debug, Clone, Copy)]
pub struct ZoneStats {
    /// Total frames in this zone
    pub total_frames: usize,
    /// Free frames in this zone
    pub free_frames: usize,
    /// Last allocation index hint
    pub last_alloc_index: usize,
}

impl ZoneStats {
    pub const fn new() -> Self {
        ZoneStats {
            total_frames: 0,
            free_frames: 0,
            last_alloc_index: 0,
        }
    }
}

/// NUMA node statistics
#[derive(Debug, Clone, Copy)]
pub struct NumaStats {
    /// Total frames in this NUMA node
    pub total_frames: usize,
    /// Free frames in this NUMA node
    pub free_frames: usize,
}

impl NumaStats {
    pub const fn new() -> Self {
        NumaStats {
            total_frames: 0,
            free_frames: 0,
        }
    }
}

/// Physical Memory Manager
///
/// The main data structure that tracks all physical memory.
/// 
/// Think of this as the "master ledger" - it knows about every single
/// 4KB page of RAM in the system and whether it's available or in use.
pub struct PhysicalMemoryManager {
    bitmap: *mut u8,
    bitmap_size: usize,          // in bytes
    bitmap_size_aligned: usize,  // in bytes (page-aligned)
    total_frames: usize,
    free_frames: usize,
    last_alloc_index: usize,     // optimization: start search here
    highest_address: u64,
    
    // Zone tracking
    zones: [ZoneStats; 4],       // DMA, DMA32, Normal, HighMem
    
    // NUMA tracking
    numa_nodes: [NumaStats; 8],  // Support up to 8 NUMA nodes
    numa_count: usize,           // Actual number of NUMA nodes
}

impl PhysicalMemoryManager {
    #[inline(always)]
    unsafe fn serial_outb(val: u8) {
        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") val, options(nomem, nostack));
    }

    #[inline(always)]
    unsafe fn serial_print_str(s: &str) {
        for &b in s.as_bytes() {
            Self::serial_outb(b);
        }
    }

    #[inline(always)]
    unsafe fn serial_print_hex_u64(mut num: u64) {
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        Self::serial_print_str("0x");
        if num == 0 {
            Self::serial_print_str("0");
            return;
        }
        let mut buf = [0u8; 16];
        let mut i = 0usize;
        while num > 0 && i < 16 {
            let digit = (num & 0xF) as usize;
            buf[i] = HEX[digit];
            num >>= 4;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            Self::serial_outb(buf[i]);
        }
    }

    #[inline(always)]
    unsafe fn debug_print_hex_u64(mut num: u64) {
        extern "C" {
            fn writestring_vga(data: *const i8);
        }
        const HEX: &[u8; 16] = b"0123456789ABCDEF";
        writestring_vga("0x\0".as_ptr() as *const i8);
        if num == 0 {
            writestring_vga("0\0".as_ptr() as *const i8);
            return;
        }
        let mut buf = [0u8; 17];
        let mut i = 0usize;
        while num > 0 && i < 16 {
            let digit = (num & 0xF) as usize;
            buf[i] = HEX[digit];
            num >>= 4;
            i += 1;
        }
        while i > 0 {
            i -= 1;
            let s = [buf[i], 0];
            writestring_vga(s.as_ptr() as *const i8);
        }
    }

    /// Initialize the Physical Memory Manager
    ///
    /// This is called during kernel boot to set up memory management.
    /// 
    /// What it does:
    /// 1. Reads the E820 memory map (from BIOS) to see what RAM exists
    /// 2. Calculates how much memory we need for the bitmap itself
    /// 3. Finds a safe place to store the bitmap (must not overwrite kernel!)
    /// 4. Marks all memory as used, then walks the memory map and marks
    ///    free regions as free
    /// 5. Marks special regions as used (kernel, bootloader, bitmap itself)
    ///
    /// # Safety
    /// - Must be called exactly once during kernel initialization
    /// - Must be called before any memory allocation
    /// - Writes to physical memory to store the bitmap
    pub unsafe fn init(memory_map: &MemoryMap) -> Result<(), MemoryError> {
        extern "C" {
            fn writestring_vga(data: *const i8);
        }
        
        Self::serial_print_str("PMM: Initializing...\n");
        
        //// Step 1: Get highest address and validate
        let highest_address = memory_map.highest_usable_address();

        if highest_address == 0 {
            return Err(MemoryError {
                code: 1,
                message: "No memory detected!",
                fatal: true,
            });
        }

        //// Step 2: Calculate bitmap requirements
        let total_frames = (highest_address / PAGE_SIZE as u64) as usize;
        let bitmap_size = (total_frames + 7) / 8; // round up to full bytes
        let bitmap_size_aligned = ((bitmap_size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;

        //// Step 3: Find a suitable region for the bitmap
        // Kernel occupies 1MB to 10MB (0x100000 to 0xA00000), so bitmap must go after
        // Reserve extra space for the kernel stack (bootloader sets RSP to 0x900000)
        const KERNEL_END: u64 = 0x100000 + (9 * 1024 * 1024); // 10MB
        
        let entry_count = memory_map.entry_count();
        let mut bitmap_base_address: u64 = 0;
        let mut found_region = false;

        for i in 0..entry_count {
            if let Some(entry) = memory_map.get(i) {
                if entry.is_usable() {
                    // Calculate the usable start within this region (after kernel if overlapping)
                    let region_start = if entry.base < KERNEL_END {
                        KERNEL_END
                    } else {
                        entry.base
                    };
                    
                    // Check if region extends past our start point
                    let region_end = entry.base + entry.length;
                    if region_end <= region_start {
                        continue; // Region ends before our usable start
                    }
                    
                    let usable_length = region_end - region_start;
                    
                    // Skip if too small for bitmap
                    if usable_length < bitmap_size_aligned as u64 {
                        continue;
                    }
                    
                    bitmap_base_address = region_start;
                    found_region = true;
                    break;
                }
            }
        }

        if !found_region {
            return Err(MemoryError {
                code: 2,
                message: "No suitable region for bitmap!",
                fatal: true,
            });
        }

        //// Step 4: Page-align the bitmap base address
        if bitmap_base_address % PAGE_SIZE as u64 != 0 {
            bitmap_base_address = ((bitmap_base_address / PAGE_SIZE as u64) + 1) * PAGE_SIZE as u64;
        }

        let bitmap_ptr = bitmap_base_address as *mut u8;
        
        Self::serial_print_str("PMM: bitmap at ");
        Self::serial_print_hex_u64(bitmap_base_address);
        Self::serial_print_str(", ");
        Self::serial_print_hex_u64(total_frames as u64);
        Self::serial_print_str(" frames\n");
        
        //// Step 5: Initialize bitmap to "all used" (0xFF)
        // Use volatile writes to prevent compiler from optimizing into SIMD memset
        // (SSE is disabled in kernel, so SIMD instructions cause GPF)
        for byte_idx in 0..bitmap_size {
            core::ptr::write_volatile(bitmap_ptr.add(byte_idx), 0xFF);
        }

        let mut free_frames: usize = 0;

        //// Step 6: Mark usable regions as free
        for i in 0..entry_count {
            if let Some(entry) = memory_map.get(i) {
                if entry.is_usable() {
                    let region_base = entry.base;
                    let region_end = entry.base + entry.length;

                    // Align to page boundaries (round inward to be safe)
                    let start_frame = ((region_base + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;
                    let mut end_frame = (region_end / PAGE_SIZE as u64) as usize;

                    // Clamp to valid range
                    if start_frame >= total_frames {
                        continue;
                    }
                    if end_frame > total_frames {
                        end_frame = total_frames;
                    }

                    // Clear bits (mark as free)
                    for frame in start_frame..end_frame {
                        let byte_idx = frame / 8;
                        let bit_offset = frame % 8;
                        *bitmap_ptr.add(byte_idx) &= !(1 << bit_offset);
                        free_frames += 1;
                    }
                }
            }
        }

        //// Step 7: Re-mark reserved areas as used

        // First 1 MB (0x00 to 0x100000)
        let reserved_end_frame: usize = 0x100000 / PAGE_SIZE;
        Self::mark_region_used_static(0, reserved_end_frame, bitmap_ptr, &mut free_frames);

        // Kernel memory region (1MB to 10MB)
        let kernel_start_frame: usize = 0x100000 / PAGE_SIZE;
        let kernel_end_frame: usize = (0x100000 + 9 * 1024 * 1024) / PAGE_SIZE;
        Self::mark_region_used_static(kernel_start_frame, kernel_end_frame, bitmap_ptr, &mut free_frames);

        // Kernel stack region (32MB - 1MB)
        const KERNEL_STACK_TOP: u64 = 0x2000000; // 32MB
        const KERNEL_STACK_SIZE: u64 = 0x100000; // 1MB
        let stack_start_frame: usize = ((KERNEL_STACK_TOP - KERNEL_STACK_SIZE) / PAGE_SIZE as u64) as usize;
        let stack_end_frame: usize = (KERNEL_STACK_TOP / PAGE_SIZE as u64) as usize;
        Self::mark_region_used_static(stack_start_frame, stack_end_frame, bitmap_ptr, &mut free_frames);

        // Bitmap itself
        let bitmap_start_frame = (bitmap_base_address / PAGE_SIZE as u64) as usize;
        let bitmap_end_frame = ((bitmap_base_address + bitmap_size as u64 + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;
        Self::mark_region_used_static(bitmap_start_frame, bitmap_end_frame, bitmap_ptr, &mut free_frames);

        // Memory map data structure
        let (entries_ptr, map_entry_count) = memory_map.entries_raw();
        let memmap_base = entries_ptr as u64;
        let memmap_size = map_entry_count * core::mem::size_of::<E820Entry>();
        let memmap_start_frame = (memmap_base / PAGE_SIZE as u64) as usize;
        let memmap_end_frame = ((memmap_base + memmap_size as u64 + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;
        Self::mark_region_used_static(memmap_start_frame, memmap_end_frame, bitmap_ptr, &mut free_frames);

        //// Step 8: Handle phantom bits in last byte
        let remaining_bits = total_frames % 8;
        if remaining_bits != 0 {
            let last_byte_index = total_frames / 8;
            for bit in remaining_bits..8 {
                *bitmap_ptr.add(last_byte_index) |= 1 << bit;
            }
        }

        //// Step 9: Initialize zone statistics
        let mut zones = [ZoneStats::new(); 4];
        
        // Calculate zone boundaries in frames
        let dma_end_frame = (DMA_ZONE_END / PAGE_SIZE as u64) as usize;
        let dma32_end_frame = (DMA32_ZONE_END / PAGE_SIZE as u64) as usize;
        
        zones[0].total_frames = if total_frames < dma_end_frame { total_frames } else { dma_end_frame };
        zones[1].total_frames = if total_frames > dma_end_frame { 
            if total_frames < dma32_end_frame { total_frames - dma_end_frame } else { dma32_end_frame - dma_end_frame }
        } else { 0 };
        zones[2].total_frames = if total_frames > dma32_end_frame { total_frames - dma32_end_frame } else { 0 };

        //// Step 10: Initialize NUMA statistics (default to single node)
        let numa_count = 1;
        let mut numa_nodes = [NumaStats::new(); 8];
        numa_nodes[0].total_frames = total_frames;
        numa_nodes[0].free_frames = free_frames;

        //// Step 11: Write directly to PMM_INSTANCE to avoid struct return ABI issues
        let pmm = PhysicalMemoryManager {
            bitmap: bitmap_ptr,
            bitmap_size,
            bitmap_size_aligned,
            total_frames,
            free_frames,
            last_alloc_index: 0,
            highest_address,
            zones,
            numa_nodes,
            numa_count,
        };
        
        // Write to the global static directly
        core::ptr::write_volatile(&raw mut PMM_INSTANCE, Some(pmm));
        
        Self::serial_print_str("PMM: ");
        Self::serial_print_hex_u64(free_frames as u64);
        Self::serial_print_str(" free frames (");
        Self::serial_print_hex_u64((free_frames * PAGE_SIZE / 1024 / 1024) as u64);
        Self::serial_print_str(" MB)\n");
        
        // Return simple success value (no struct copy needed)
        Self::serial_print_str("PMM: returning success\n");
        Ok(())
    }

    /// Static helper to mark a region as used during initialization
    fn mark_region_used_static(
        start_frame: usize,
        end_frame: usize,
        bitmap_ptr: *mut u8,
        free_frames: &mut usize,
    ) {
        for frame in start_frame..end_frame {
            let byte_idx = frame / 8;
            let bit_offset = frame % 8;

            // Only decrement if bit was free (0)
            if (unsafe { *bitmap_ptr.add(byte_idx) } & (1 << bit_offset)) == 0 {
                unsafe { *bitmap_ptr.add(byte_idx) |= 1 << bit_offset };
                *free_frames = free_frames.saturating_sub(1);
            }
        }
    }

    /// Allocate a single physical frame
    /// 
    /// Returns the physical address of the allocated frame, or an error if
    /// no free frames are available.
    pub fn alloc_frame(&mut self) -> Result<u64, MemoryError> {
        // Check if any free frames available
        if self.free_frames == 0 {
            return Err(MemoryError {
                code: 3,
                message: "No free memory available!",
                fatal: false,
            });
        }

        // Search for a free frame starting from last allocation index
        let mut current_idx = self.last_alloc_index;
        let mut loop_count: usize = 0;

        while loop_count < self.bitmap_size {
            let byte_value = unsafe { *self.bitmap.add(current_idx) };

            // Check if this byte has at least one free bit
            if byte_value != 0xFF {
                // Find the first free bit in this byte
                for bit_offset in 0..8 {
                    if (byte_value & (1 << bit_offset)) == 0 {
                        // Calculate frame index
                        let frame_index = (current_idx * 8) + bit_offset;

                        // Validate frame index
                        if frame_index >= self.total_frames {
                            break; // Phantom bit, skip to next byte
                        }

                        // Set the bit (mark as used)
                        unsafe { *self.bitmap.add(current_idx) |= 1 << bit_offset };

                        // Update state
                        self.free_frames -= 1;
                        self.last_alloc_index = current_idx;

                        // Calculate and return physical address
                        let physical_address = (frame_index * PAGE_SIZE) as u64;
                        return Ok(physical_address);
                    }
                }
            }

            // Move to next byte (with wrap-around)
            current_idx = (current_idx + 1) % self.bitmap_size;
            loop_count += 1;
        }

        // Should not reach here if free_frames > 0, but handle anyway
        Err(MemoryError {
            code: 4,
            message: "Failed to find free frame!",
            fatal: false,
        })
    }

    /// Allocate a frame from a specific memory zone
    /// 
    /// # Arguments
    /// * `zone` - The memory zone to allocate from (DMA, DMA32, Normal)
    /// 
    /// Returns the physical address of the allocated frame from the specified zone.
    pub fn alloc_frame_in_zone(&mut self, zone: MemoryZone) -> Result<u64, MemoryError> {
        let zone_idx = zone as usize;
        
        if self.zones[zone_idx].free_frames == 0 {
            return Err(MemoryError {
                code: 10,
                message: "No free frames in requested zone!",
                fatal: false,
            });
        }

        // Determine zone boundaries in frames
        let zone_start = (zone.start_address() / PAGE_SIZE as u64) as usize;
        let zone_end = core::cmp::min(
            (zone.end_address() / PAGE_SIZE as u64) as usize,
            self.total_frames
        );

        // Search for a free frame in this zone
        let start_byte = zone_start / 8;
        let end_byte = (zone_end + 7) / 8;

        for byte_idx in start_byte..end_byte {
            let byte_value = unsafe { *self.bitmap.add(byte_idx) };

            if byte_value != 0xFF {
                for bit_offset in 0..8 {
                    let frame_index = (byte_idx * 8) + bit_offset;

                    // Check if frame is in zone bounds
                    if frame_index < zone_start || frame_index >= zone_end {
                        continue;
                    }

                    // Check if frame is free
                    if (byte_value & (1 << bit_offset)) == 0 {
                        // Set the bit (mark as used)
                        unsafe { *self.bitmap.add(byte_idx) |= 1 << bit_offset };

                        // Update statistics
                        self.free_frames -= 1;
                        self.zones[zone_idx].free_frames -= 1;
                        self.last_alloc_index = byte_idx;

                        let physical_address = (frame_index * PAGE_SIZE) as u64;
                        return Ok(physical_address);
                    }
                }
            }
        }

        Err(MemoryError {
            code: 11,
            message: "Failed to find free frame in zone!",
            fatal: false,
        })
    }

    /// Allocate a frame preferring a specific NUMA node
    /// 
    /// # Arguments
    /// * `preferred_node` - The NUMA node to prefer for allocation
    /// 
    /// For now, this is a placeholder that falls back to normal allocation.
    /// Full NUMA support requires ACPI SRAT table parsing.
    pub fn alloc_frame_numa(&mut self, preferred_node: NumaNode) -> Result<u64, MemoryError> {
        // TODO: Implement NUMA-aware allocation when ACPI SRAT is available
        // For now, fall back to normal allocation
        self.alloc_frame()
    }

    /// Allocate a DMA-capable frame (within first 16MB)
    /// 
    /// This is a convenience wrapper for allocating from the DMA zone.
    pub fn alloc_dma_frame(&mut self) -> Result<u64, MemoryError> {
        self.alloc_frame_in_zone(MemoryZone::Dma)
    }

    /// Allocate a DMA32-capable frame (within first 4GB)
    /// 
    /// This tries DMA zone first, then DMA32 zone.
    pub fn alloc_dma32_frame(&mut self) -> Result<u64, MemoryError> {
        // Try DMA zone first
        if let Ok(addr) = self.alloc_frame_in_zone(MemoryZone::Dma) {
            return Ok(addr);
        }
        
        // Fall back to DMA32 zone
        self.alloc_frame_in_zone(MemoryZone::Dma32)
    }

    /// Allocate contiguous physical frames
    /// 
    /// # Arguments
    /// * `count` - Number of contiguous frames to allocate
    /// * `zone` - Optional zone constraint
    /// 
    /// Returns the base physical address of the first frame in the contiguous range.
    /// This is essential for DMA buffers that require physically contiguous memory.
    pub fn alloc_frames_contiguous(&mut self, count: usize, zone: Option<MemoryZone>) -> Result<u64, MemoryError> {
        if count == 0 {
            return Err(MemoryError {
                code: 12,
                message: "Cannot allocate zero frames!",
                fatal: false,
            });
        }

        if count == 1 {
            // Single frame - use regular allocation
            return match zone {
                Some(z) => self.alloc_frame_in_zone(z),
                None => self.alloc_frame(),
            };
        }

        // Determine search range
        let (start_frame, end_frame) = match zone {
            Some(z) => {
                let start = (z.start_address() / PAGE_SIZE as u64) as usize;
                let end = core::cmp::min(
                    (z.end_address() / PAGE_SIZE as u64) as usize,
                    self.total_frames
                );
                (start, end)
            }
            None => (0, self.total_frames),
        };

        // Search for contiguous free frames
        let mut current_frame = start_frame;
        
        while current_frame + count <= end_frame {
            // Check if we have 'count' contiguous free frames starting at current_frame
            let mut all_free = true;
            
            for offset in 0..count {
                let frame = current_frame + offset;
                if self.test_bit(frame) {
                    // Frame is used
                    all_free = false;
                    // Jump past this frame
                    current_frame = frame + 1;
                    break;
                }
            }

            if all_free {
                // Found contiguous region! Allocate all frames
                for offset in 0..count {
                    let frame = current_frame + offset;
                    let byte_idx = frame / 8;
                    let bit_offset = frame % 8;
                    
                    unsafe {
                        *self.bitmap.add(byte_idx) |= 1 << bit_offset;
                    }
                    
                    // Update zone statistics
                    let addr = (frame * PAGE_SIZE) as u64;
                    let frame_zone = MemoryZone::from_address(addr);
                    let zone_idx = frame_zone as usize;
                    self.zones[zone_idx].free_frames -= 1;
                }
                
                self.free_frames -= count;
                let base_addr = (current_frame * PAGE_SIZE) as u64;
                return Ok(base_addr);
            }

            if !all_free {
                continue; // current_frame was already incremented
            }
            
            current_frame += 1;
        }

        Err(MemoryError {
            code: 13,
            message: "No contiguous frames available!",
            fatal: false,
        })
    }

    /// Free contiguous physical frames
    /// 
    /// # Arguments
    /// * `base_addr` - Base physical address of the contiguous region
    /// * `count` - Number of frames to free
    pub fn free_frames_contiguous(&mut self, base_addr: u64, count: usize) -> Result<(), MemoryError> {
        if count == 0 {
            return Ok(());
        }

        // Validate base address alignment
        if base_addr % PAGE_SIZE as u64 != 0 {
            return Err(MemoryError {
                code: 5,
                message: "Address not page-aligned!",
                fatal: false,
            });
        }

        // Free each frame in the range
        for i in 0..count {
            let addr = base_addr + (i * PAGE_SIZE) as u64;
            self.free_frame(addr)?;
        }

        Ok(())
    }

    /// Free a previously allocated physical frame
    /// 
    /// # Arguments
    /// * `addr` - Physical address of the frame to free (must be page-aligned)
    pub fn free_frame(&mut self, addr: u64) -> Result<(), MemoryError> {
        // Validate: address must be page-aligned
        if addr % PAGE_SIZE as u64 != 0 {
            return Err(MemoryError {
                code: 5,
                message: "Address not page-aligned!",
                fatal: false,
            });
        }

        // Calculate frame index
        let frame_index = (addr / PAGE_SIZE as u64) as usize;

        // Validate: frame must be within bounds
        if frame_index >= self.total_frames {
            return Err(MemoryError {
                code: 6,
                message: "Address out of bounds!",
                fatal: false,
            });
        }

        // Calculate bitmap position
        let byte_idx = frame_index / 8;
        let bit_offset = frame_index % 8;

        // Check for double-free
        let byte_value = unsafe { *self.bitmap.add(byte_idx) };
        if (byte_value & (1 << bit_offset)) == 0 {
            return Err(MemoryError {
                code: 7,
                message: "Double free detected!",
                fatal: false,
            });
        }

        // Clear the bit (mark as free)
        unsafe { *self.bitmap.add(byte_idx) &= !(1 << bit_offset) };

        // Update global state
        self.free_frames += 1;

        // Update zone statistics
        let zone = MemoryZone::from_address(addr);
        let zone_idx = zone as usize;
        self.zones[zone_idx].free_frames += 1;

        // Optimization: if freed frame is before last_alloc_index, update hint
        if byte_idx < self.last_alloc_index {
            self.last_alloc_index = byte_idx;
        }

        Ok(())
    }

    /// Set a bit in the bitmap (mark frame as used)
    fn set_bit(&mut self, frame: usize) {
        if frame >= self.total_frames {
            return;
        }

        let byte_idx = frame / 8;
        let bit_offset = frame % 8;

        // Only decrement free_frames if bit was free
        let byte_value = unsafe { *self.bitmap.add(byte_idx) };
        if (byte_value & (1 << bit_offset)) == 0 {
            unsafe { *self.bitmap.add(byte_idx) |= 1 << bit_offset };
            self.free_frames = self.free_frames.saturating_sub(1);
        }
    }

    /// Clear a bit in the bitmap (mark frame as free)
    fn clear_bit(&mut self, frame: usize) {
        if frame >= self.total_frames {
            return;
        }

        let byte_idx = frame / 8;
        let bit_offset = frame % 8;

        // Only increment free_frames if bit was set
        let byte_value = unsafe { *self.bitmap.add(byte_idx) };
        if (byte_value & (1 << bit_offset)) != 0 {
            unsafe { *self.bitmap.add(byte_idx) &= !(1 << bit_offset) };
            self.free_frames += 1;
        }
    }

    /// Test if a bit is set (frame is used)
    fn test_bit(&self, frame: usize) -> bool {
        if frame >= self.total_frames {
            return true; // Out of bounds = treated as used
        }

        let byte_idx = frame / 8;
        let bit_offset = frame % 8;

        let byte_value = unsafe { *self.bitmap.add(byte_idx) };
        (byte_value & (1 << bit_offset)) != 0
    }

    /// Mark a region of frames as used
    pub fn mark_region_used(&mut self, base: u64, length: u64) {
        let start_frame = (base / PAGE_SIZE as u64) as usize;
        let end_frame = ((base + length + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;

        for frame in start_frame..end_frame {
            self.set_bit(frame);
        }
    }

    /// Mark a region of frames as free
    pub fn mark_region_free(&mut self, base: u64, length: u64) {
        let start_frame = (base / PAGE_SIZE as u64) as usize;
        let end_frame = ((base + length + PAGE_SIZE as u64 - 1) / PAGE_SIZE as u64) as usize;

        for frame in start_frame..end_frame {
            self.clear_bit(frame);
        }
    }

    /// Get the number of free frames
    pub fn free_frame_count(&self) -> usize {
        self.free_frames
    }

    /// Get the total number of frames
    pub fn total_frame_count(&self) -> usize {
        self.total_frames
    }

    /// Get free memory in bytes
    pub fn free_memory(&self) -> u64 {
        (self.free_frames * PAGE_SIZE) as u64
    }

    /// Get used memory in bytes
    pub fn used_memory(&self) -> u64 {
        ((self.total_frames - self.free_frames) * PAGE_SIZE) as u64
    }

    /// Get total tracked memory in bytes
    pub fn total_memory(&self) -> u64 {
        (self.total_frames * PAGE_SIZE) as u64
    }

    /// Check if a specific frame is allocated
    pub fn is_frame_used(&self, addr: u64) -> bool {
        let frame = (addr / PAGE_SIZE as u64) as usize;
        self.test_bit(frame)
    }

    /// Get zone statistics
    pub fn zone_stats(&self, zone: MemoryZone) -> ZoneStats {
        self.zones[zone as usize]
    }

    /// Get free memory in a specific zone
    pub fn zone_free_memory(&self, zone: MemoryZone) -> u64 {
        (self.zones[zone as usize].free_frames * PAGE_SIZE) as u64
    }

    /// Get total memory in a specific zone
    pub fn zone_total_memory(&self, zone: MemoryZone) -> u64 {
        (self.zones[zone as usize].total_frames * PAGE_SIZE) as u64
    }

    /// Get NUMA node statistics
    pub fn numa_stats(&self, node: NumaNode) -> Option<NumaStats> {
        let node_id = node.id() as usize;
        if node_id < self.numa_count {
            // Use unsafe get to avoid bounds check
            unsafe {
                Some(*self.numa_nodes.get_unchecked(node_id))
            }
        } else {
            None
        }
    }

    /// Get number of NUMA nodes
    pub fn numa_node_count(&self) -> usize {
        self.numa_count
    }

    /// Get free memory on a NUMA node
    pub fn numa_free_memory(&self, node: NumaNode) -> u64 {
        if let Some(stats) = self.numa_stats(node) {
            (stats.free_frames * PAGE_SIZE) as u64
        } else {
            0
        }
    }

    /// Get total memory on a NUMA node
    pub fn numa_total_memory(&self, node: NumaNode) -> u64 {
        if let Some(stats) = self.numa_stats(node) {
            (stats.total_frames * PAGE_SIZE) as u64
        } else {
            0
        }
    }

    /// Get allocation percentage (used/total) as integer percentage
    pub fn memory_usage_percent(&self) -> u8 {
        if self.total_frames == 0 {
            return 0;
        }
        let used = self.total_frames - self.free_frames;
        ((used * 100) / self.total_frames) as u8
    }

    /// Get zone allocation percentage
    pub fn zone_usage_percent(&self, zone: MemoryZone) -> u8 {
        let stats = self.zones[zone as usize];
        if stats.total_frames == 0 {
            return 0;
        }
        let used = stats.total_frames - stats.free_frames;
        ((used * 100) / stats.total_frames) as u8
    }

    /// Check if memory is critically low (< 10% free)
    pub fn is_memory_critical(&self) -> bool {
        self.memory_usage_percent() > 90
    }

    /// Check if a zone is critically low (< 10% free)
    pub fn is_zone_critical(&self, zone: MemoryZone) -> bool {
        self.zone_usage_percent(zone) > 90
    }

    /// Get fragmentation estimate (percentage)
    /// This is a simple heuristic - could be improved with better algorithms
    pub fn fragmentation_estimate(&self) -> u8 {
        // If we can't allocate large contiguous blocks despite having free memory,
        // we're fragmented. This is a simplified check.
        if self.free_frames == 0 {
            return 0;
        }
        
        // Try to find largest contiguous block
        let mut max_contiguous = 0;
        let mut current_run = 0;
        
        for frame in 0..self.total_frames {
            if !self.test_bit(frame) {
                current_run += 1;
                if current_run > max_contiguous {
                    max_contiguous = current_run;
                }
            } else {
                current_run = 0;
            }
        }
        
        if self.free_frames == 0 {
            return 100;
        }
        
        // Fragmentation = 100 - (largest_block * 100 / total_free)
        let ratio = (max_contiguous * 100) / self.free_frames;
        (100 - ratio) as u8
    }
}

// =============================================================================
// Global PMM Instance
// =============================================================================

static mut PMM_INSTANCE: Option<PhysicalMemoryManager> = None;

/// Get mutable reference to global PMM instance
#[inline]
#[allow(static_mut_refs)]
unsafe fn get_pmm_mut() -> Option<&'static mut PhysicalMemoryManager> {
    PMM_INSTANCE.as_mut()
}

/// Get immutable reference to global PMM instance
#[inline]
unsafe fn get_pmm() -> Option<&'static PhysicalMemoryManager> {
    PMM_INSTANCE.as_ref()
}

/// Initialize the global PMM
/// 
/// # Safety
/// Must be called exactly once during kernel initialization.
pub unsafe fn pmm_init(memory_map: &MemoryMap) -> Result<(), MemoryError> {
    // init() now writes directly to PMM_INSTANCE and returns Result<(), MemoryError>
    PhysicalMemoryManager::init(memory_map)
}

/// Allocate a physical frame using the global PMM
#[no_mangle]
pub fn alloc_frame() -> Result<u64, MemoryError> {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => pmm.alloc_frame(),
            None => Err(MemoryError {
                code: 8,
                message: "PMM not initialized!",
                fatal: true,
            }),
        }
    }
}

/// Free a physical frame using the global PMM
#[no_mangle]
pub fn free_frame(addr: u64) -> Result<(), MemoryError> {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => pmm.free_frame(addr),
            None => Err(MemoryError {
                code: 8,
                message: "PMM not initialized!",
                fatal: true,
            }),
        }
    }
}

/// Get free memory in bytes
#[no_mangle]
pub fn pmm_free_memory() -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => pmm.free_memory(),
            None => 0,
        }
    }
}

/// Get used memory in bytes
#[no_mangle]
pub fn pmm_used_memory() -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => pmm.used_memory(),
            None => 0,
        }
    }
}

// =============================================================================
// C-Compatible Wrapper Functions
// =============================================================================

/// Initialize PMM using the global memory map
/// 
/// # Returns
/// - 0 on success
/// - Negative error code on failure
/// 
/// # Safety
/// Must be called after memory_init() has been called.
#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe extern "C" fn pmm_init_c(_memory_map_addr: u64) -> i32 {
    // Check if memory map is initialized
    if !MEMORY_MAP_STORAGE.initialized {
        PhysicalMemoryManager::serial_print_str("PMM: ERROR - Memory map not initialized!\n");
        return -9;
    }
    
    // Call init (writes directly to PMM_INSTANCE)
    match PhysicalMemoryManager::init(&MEMORY_MAP_STORAGE.map) {
        Ok(()) => {
            // Verify PMM_INSTANCE is now Some
            if PMM_INSTANCE.is_some() {
                0
            } else {
                PhysicalMemoryManager::serial_print_str("PMM: ERROR - PMM_INSTANCE is None!\n");
                -10
            }
        }
        Err(e) => {
            PhysicalMemoryManager::serial_print_str("PMM: Init failed: ");
            PhysicalMemoryManager::serial_print_str(e.message);
            PhysicalMemoryManager::serial_print_str("\n");
            -(e.code as i32)
        }
    }
}

/// Allocate a physical frame (C-compatible)
/// 
/// # Returns
/// - Physical address of allocated frame on success
/// - 0 on failure (no memory available or PMM not initialized)
#[no_mangle]
pub extern "C" fn pmm_alloc_frame() -> u64 {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => {
                match pmm.alloc_frame() {
                    Ok(addr) => addr,
                    Err(_) => 0, // Allocation failed
                }
            }
            None => 0, // PMM not initialized
        }
    }
}

/// Free a physical frame (C-compatible)
/// 
/// # Arguments
/// * `addr` - Physical address of the frame to free (must be page-aligned)
/// 
/// # Returns
/// - 0 on success
/// - Negative error code on failure
#[no_mangle]
pub extern "C" fn pmm_free_frame(addr: u64) -> i32 {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => {
                match pmm.free_frame(addr) {
                    Ok(()) => 0, // Success
                    Err(e) => -(e.code as i32), // Return negative error code
                }
            }
            None => -8, // PMM not initialized
        }
    }
}

/// Get total memory tracked by PMM in bytes (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_total_memory() -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => pmm.total_memory(),
            None => 0,
        }
    }
}

/// Get number of free frames (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_free_frame_count() -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => pmm.free_frame_count() as u64,
            None => 0,
        }
    }
}

/// Get number of used frames (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_used_frame_count() -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => (pmm.total_frame_count() - pmm.free_frame_count()) as u64,
            None => 0,
        }
    }
}

/// Check if PMM is initialized (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_is_initialized() -> i32 {
    unsafe {
        match get_pmm() {
            Some(_) => 1, // Initialized
            None => 0,    // Not initialized
        }
    }
}

/// Check if a frame is used (C-compatible)
/// 
/// # Returns
/// - 1 if frame is used
/// - 0 if frame is free
/// - -1 if PMM not initialized
#[no_mangle]
pub extern "C" fn pmm_is_frame_used(addr: u64) -> i32 {
    unsafe {
        match get_pmm() {
            Some(pmm) => {
                if pmm.is_frame_used(addr) { 1 } else { 0 }
            }
            None => -1,
        }
    }
}

// =============================================================================
// Zone-Aware C-Compatible Functions
// =============================================================================

/// Allocate a frame from a specific zone (C-compatible)
/// 
/// # Arguments
/// * `zone` - Zone ID: 0=DMA, 1=DMA32, 2=Normal, 3=HighMem
/// 
/// # Returns
/// - Physical address on success
/// - 0 on failure
#[no_mangle]
pub extern "C" fn pmm_alloc_frame_in_zone(zone: u8) -> u64 {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => {
                let memory_zone = match zone {
                    0 => MemoryZone::Dma,
                    1 => MemoryZone::Dma32,
                    2 => MemoryZone::Normal,
                    3 => MemoryZone::HighMem,
                    _ => return 0,
                };
                pmm.alloc_frame_in_zone(memory_zone).unwrap_or(0)
            }
            None => 0,
        }
    }
}

/// Allocate a DMA-capable frame (first 16MB) - C-compatible
#[no_mangle]
pub extern "C" fn pmm_alloc_dma_frame() -> u64 {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => pmm.alloc_dma_frame().unwrap_or(0),
            None => 0,
        }
    }
}

/// Allocate a DMA32-capable frame (first 4GB) - C-compatible
#[no_mangle]
pub extern "C" fn pmm_alloc_dma32_frame() -> u64 {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => pmm.alloc_dma32_frame().unwrap_or(0),
            None => 0,
        }
    }
}

/// Get free memory in a zone (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_zone_free_memory(zone: u8) -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => {
                let memory_zone = match zone {
                    0 => MemoryZone::Dma,
                    1 => MemoryZone::Dma32,
                    2 => MemoryZone::Normal,
                    3 => MemoryZone::HighMem,
                    _ => return 0,
                };
                pmm.zone_free_memory(memory_zone)
            }
            None => 0,
        }
    }
}

/// Get total memory in a zone (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_zone_total_memory(zone: u8) -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => {
                let memory_zone = match zone {
                    0 => MemoryZone::Dma,
                    1 => MemoryZone::Dma32,
                    2 => MemoryZone::Normal,
                    3 => MemoryZone::HighMem,
                    _ => return 0,
                };
                pmm.zone_total_memory(memory_zone)
            }
            None => 0,
        }
    }
}

/// Allocate contiguous frames (C-compatible)
/// 
/// # Arguments
/// * `count` - Number of contiguous frames to allocate
/// * `zone` - Zone ID (0=DMA, 1=DMA32, 2=Normal, 0xFF=Any)
/// 
/// # Returns
/// - Base physical address on success
/// - 0 on failure
#[no_mangle]
pub extern "C" fn pmm_alloc_frames_contiguous(count: u64, zone: u8) -> u64 {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => {
                let memory_zone = if zone == 0xFF {
                    None
                } else {
                    Some(match zone {
                        0 => MemoryZone::Dma,
                        1 => MemoryZone::Dma32,
                        2 => MemoryZone::Normal,
                        3 => MemoryZone::HighMem,
                        _ => return 0,
                    })
                };
                pmm.alloc_frames_contiguous(count as usize, memory_zone).unwrap_or(0)
            }
            None => 0,
        }
    }
}

/// Free contiguous frames (C-compatible)
/// 
/// # Arguments
/// * `base_addr` - Base physical address
/// * `count` - Number of frames to free
/// 
/// # Returns
/// - 0 on success
/// - Negative error code on failure
#[no_mangle]
pub extern "C" fn pmm_free_frames_contiguous(base_addr: u64, count: u64) -> i32 {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => {
                match pmm.free_frames_contiguous(base_addr, count as usize) {
                    Ok(()) => 0,
                    Err(e) => -(e.code as i32),
                }
            }
            None => -1,
        }
    }
}

/// Get memory usage percentage (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_memory_usage_percent() -> u8 {
    unsafe {
        match get_pmm() {
            Some(pmm) => pmm.memory_usage_percent(),
            None => 0,
        }
    }
}

/// Check if memory is critically low (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_is_memory_critical() -> i32 {
    unsafe {
        match get_pmm() {
            Some(pmm) => if pmm.is_memory_critical() { 1 } else { 0 },
            None => 0,
        }
    }
}

// =============================================================================
// NUMA-Aware C-Compatible Functions
// =============================================================================

/// Allocate a frame with NUMA node preference (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_alloc_frame_numa(node_id: u8) -> u64 {
    unsafe {
        match get_pmm_mut() {
            Some(pmm) => {
                let node = NumaNode::new(node_id);
                pmm.alloc_frame_numa(node).unwrap_or(0)
            }
            None => 0,
        }
    }
}

/// Get number of NUMA nodes (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_numa_node_count() -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => pmm.numa_node_count() as u64,
            None => 0,
        }
    }
}

/// Get free memory on a NUMA node (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_numa_free_memory(node_id: u8) -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => {
                let node = NumaNode::new(node_id);
                pmm.numa_free_memory(node)
            }
            None => 0,
        }
    }
}

/// Get total memory on a NUMA node (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_numa_total_memory(node_id: u8) -> u64 {
    unsafe {
        match get_pmm() {
            Some(pmm) => {
                let node = NumaNode::new(node_id);
                pmm.numa_total_memory(node)
            }
            None => 0,
        }
    }
}
