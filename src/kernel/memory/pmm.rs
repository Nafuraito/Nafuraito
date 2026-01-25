//! Physical Memory Manager (PMM)
//!
//! Bitmap-based physical frame allocator for the kernel.
//! Tracks which 4KB page frames are free or allocated.

#![allow(unused)]

use super::{E820Entry, MemoryMap, MemoryError, MEMORY_MAP_STORAGE};

pub const PAGE_SIZE: usize = 4096; // 4 KB

/// Physical Memory Manager
/// 
/// Uses a bitmap to track allocation status of physical page frames.
/// Each bit represents one 4KB frame: 0 = free, 1 = used.
pub struct PhysicalMemoryManager {
    bitmap: *mut u8,
    bitmap_size: usize,          // in bytes
    bitmap_size_aligned: usize,  // in bytes (page-aligned)
    total_frames: usize,
    free_frames: usize,
    last_alloc_index: usize,     // optimization: start search here
    highest_address: u64,
}

impl PhysicalMemoryManager {
    /// Initialize the Physical Memory Manager
    /// 
    /// # Safety
    /// Must be called after memory map is initialized.
    /// Writes to physical memory for bitmap storage.
    pub unsafe fn init(memory_map: &MemoryMap) -> Result<Self, MemoryError> {
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
        // Kernel occupies 1MB to 9MB (0x100000 to 0x900000), so bitmap must go after
        const KERNEL_END: u64 = 0x100000 + (8 * 1024 * 1024); // 9MB
        
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

        //// Step 5: Initialize bitmap to "all used" (0xFF)
        for byte_idx in 0..bitmap_size {
            *bitmap_ptr.add(byte_idx) = 0xFF;
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

        // Kernel memory region (1MB to 9MB)
        let kernel_start_frame: usize = 0x100000 / PAGE_SIZE;
        let kernel_end_frame: usize = (0x100000 + 8 * 1024 * 1024) / PAGE_SIZE;
        Self::mark_region_used_static(kernel_start_frame, kernel_end_frame, bitmap_ptr, &mut free_frames);

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

        //// Step 9: Return initialized PMM
        Ok(PhysicalMemoryManager {
            bitmap: bitmap_ptr,
            bitmap_size,
            bitmap_size_aligned,
            total_frames,
            free_frames,
            last_alloc_index: 0,
            highest_address,
        })
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

        // Update state
        self.free_frames += 1;

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
}

// =============================================================================
// Global PMM Instance
// =============================================================================

static mut PMM_INSTANCE: Option<PhysicalMemoryManager> = None;

/// Initialize the global PMM
/// 
/// # Safety
/// Must be called exactly once during kernel initialization.
pub unsafe fn pmm_init(memory_map: &MemoryMap) -> Result<(), MemoryError> {
    match PhysicalMemoryManager::init(memory_map) {
        Ok(pmm) => {
            PMM_INSTANCE = Some(pmm);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Allocate a physical frame using the global PMM
#[no_mangle]
pub fn alloc_frame() -> Result<u64, MemoryError> {
    unsafe {
        match PMM_INSTANCE.as_mut() {
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
        match PMM_INSTANCE.as_mut() {
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
        match PMM_INSTANCE.as_ref() {
            Some(pmm) => pmm.free_memory(),
            None => 0,
        }
    }
}

/// Get used memory in bytes
#[no_mangle]
pub fn pmm_used_memory() -> u64 {
    unsafe {
        match PMM_INSTANCE.as_ref() {
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
    // Use the already-initialized global memory map instead of re-parsing
    if !MEMORY_MAP_STORAGE.initialized {
        return -9; // Memory map not initialized
    }
    
    match PhysicalMemoryManager::init(&MEMORY_MAP_STORAGE.map) {
        Ok(pmm) => {
            PMM_INSTANCE = Some(pmm);
            0 // Success
        }
        Err(e) => -(e.code as i32), // Return negative error code
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
        match PMM_INSTANCE.as_mut() {
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
        match PMM_INSTANCE.as_mut() {
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
        match PMM_INSTANCE.as_ref() {
            Some(pmm) => pmm.total_memory(),
            None => 0,
        }
    }
}

/// Get number of free frames (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_free_frame_count() -> u64 {
    unsafe {
        match PMM_INSTANCE.as_ref() {
            Some(pmm) => pmm.free_frame_count() as u64,
            None => 0,
        }
    }
}

/// Get number of used frames (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_used_frame_count() -> u64 {
    unsafe {
        match PMM_INSTANCE.as_ref() {
            Some(pmm) => (pmm.total_frame_count() - pmm.free_frame_count()) as u64,
            None => 0,
        }
    }
}

/// Check if PMM is initialized (C-compatible)
#[no_mangle]
pub extern "C" fn pmm_is_initialized() -> i32 {
    unsafe {
        match PMM_INSTANCE.as_ref() {
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
        match PMM_INSTANCE.as_ref() {
            Some(pmm) => {
                if pmm.is_frame_used(addr) { 1 } else { 0 }
            }
            None => -1,
        }
    }
}

