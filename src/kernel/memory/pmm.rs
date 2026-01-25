pub const PAGE_SIZE: usize = 4096; // 4 KB

pub struct PhysicalMemoryManager {
    bitmap: *mut u8,
    bitmap_size: usize,      // in bytes
    bitmap_size_aligned: usize,      // in pages
    total_frames: usize,
    free_frames: usize,
    last_alloc_index: usize, // optimization: start search here
    highest_address: *mut u64,
}

impl PhysicalMemoryManager {

    pub unsafe fn init(memory_map: &MemoryMap) -> Self {
        //// Search for usable Region

        let highest_address = memory_map.highest_usable_address();

        if highest_address == 0 {
            // No memory detected
            return;
        }


        let total_frames = (highest_address / PAGE_SIZE as u64) as usize;
        let bitmap_size = (total_frames + 7) / 8; // round up to full bytes
        let bitmap_size_aligned = ((bitmap_size + 4095) / 4096) * 4096; // page-align the size

        let entry_count: usize = memory_map.valid_entry_count();

        let mut used_entry: E820Entry;
        
        for i in 0..=entry_count {
            let entry: E820Entry = memory_map.get(i);

            if memory_map.is_usable() {
                // Skip if smaller than 1 MB (we avoid first 1 MB) or too small for bitmap
                if (entry.base < 0x100000) || (entry.length < bitmap_size_aligned) {
                    continue;
                }

                used_entry = entry;
            }
        }

        // Some variables
        let mut bitmap_base_address: u64 = used_entry.base;
        
        //// Place the Bitmap

        if bitmap_base_address % 4096 != 0 {
            bitmap_base_address = ((bitmap_base_address / 4096) + 1) * 4096; // Bitmap must be page aligned
        }

        let bitmap_ptr = bitmap_base_address as *mut u8; // pointer to base address
        let bitmap_end: u64 = bitmap_base_address + bitmap_size;

        //// Init Bitmap to "all used"
        
        // Set every byte
        for byte_idx in 0..bitmap_size {
            bitmap[byte_idx] = 0xFF; // Write "used" to byte
        }

        // Set some variables
        free_frames = 0;
        let used_frames = total_frames;

        //// Mark usable regions as free
        
        for i in 0..=entry_count {
            let entry: E820Entry = memory_map.get(i);

            if entry.is_usable() {
                // Calc frame range
                let region_base = entry.base;
                let region_end = entry.base + entry.length;

                // Align to page boundaries (round inward to be safe)
                let start_frame = (region_base + 4095) / 4096; // round UP
                let end_frame = region_end / 4096;             // round DOWN

                // Clamp to valid range
                if start_frame >= total_frames { continue; } // Skip
                if end_frame > total_frames {
                    end_frame = total_frames;                // Clamp
                }

                // Clear bits (mark as free)
                for frame in start_frame..end_frame {
                    let byte_idx = frame / 8;
                    let bit_offset = frame % 8;
                    bitmap[byte_idx] &= ~(1 << bit_offset)  // Clear bit
                    free_frames += 1
                }
            }
        }

        //// Re-mark Reserved areas as used

        // these regions must NEVER be allocated, even if E820 said "uasble"

        //// First 1 MB (0x00 to 0x0100000)
        // Why: Real-mode IVT, BIOS data area, legacy hardware regions, bootloader structure

        let reserved_end_frame = 256; // 0x100000 / 4096

        // set bits (mark as used)
        Self::mark_bits_as_used(0 as u64, reserved_end_frame as u64, bitmap_ptr);

        //// Kernel memory region
        // just 8 MB, then the kernel can allocate it's memory itself

        let kernel_start_frame = 0x100000 / 4096; // 256 frames (kernel starts at 1MB)
        let kernel_end_frame = (0x100000 + 8 * 1024 * 1024) / 4096; // +8MB for kernel space
        
        // Mark kernel region as used
        Self::mark_bits_as_used(kernel_start_frame as u64, kernel_end_frame as u64, bitmap_ptr);

        //// Bitmap itself
        
        let bitmap_start_frame = bitmap_base_address / 4096;
        let bitmap_end_frame = (bitmap_base_address + bitmap_size_bytes + 4095) / 4096;

        // Mark bitmap region as used
        Self::mark_bits_as_used(bitmap_start_frame as u64, bitmap_end_frame as u64, bitmap_ptr);

        //// Memory map data structure
        // Get memory map location and size
        let (entries_ptr, entry_count) = memory_map.entries_raw();
        let memmap_base = entries_ptr as u64;
        let memmap_size = entry_count * core::mem::size_of::<E820Entry>();

        let memmap_start_frame = memmap_base / 4096;
        let memmap_end_frame = (memmap_base + memmap_size as u64 + 4095) / 4096;

        // Mark memory map region as used
        Self::mark_bits_as_used(memmap_start_frame, memmap_end_frame, bitmap_ptr, &mut free_frames);

        //// Handle edge cases
        // If total_frames isn't a multiple of 8, the last byte has "phantom" bits

        let remaining_bits = total_frames % 8;
        if remaining_bits != 0 {
            let last_byte_index = total_frames / 8;

            // Set upper bits as used (they don't respresent real memory)
            for bit in remaining_bits..=7 {
                unsafe { *bitmap_ptr.add(last_byte_index) |= (1 << bit) }; 
            }
        }
    }

    fn mark_bits_as_used(start_frame: u64, end_frame: u64, bitmap_ptr: *mut u8) {
        for frame in start_frame..end_frame {
            let byte_idx = frame / 8;
            let bit_offset = frame % 8;
            
            // If the bit was set as free: make it used
            if (unsafe { *bitmap_ptr.add(byte_idx) } & (1 << bit_offset)) == 0 {
                unsafe { *bitmap_ptr.add(byte_idx) |= 1 << bit_offset }; // Set bit
                free_frames -= 1;
            }
        }
    }

    pub fn alloc_frame(&mut self) -> Option<u64> { ... }

    pub fn free_frame(&mut self, addr: u64) { ... }

    fn set_bit(&mut self, frame: usize) { ... }

    fn clear_bit(&mut self, frame: usize) { ... }

    fn test_bit(&self, frame: usize) -> bool { ... }

}
