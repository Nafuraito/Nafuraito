//! Copy-on-Write (CoW) Implementation
//!
//! This module provides Copy-on-Write functionality for efficient memory management,
//! particularly useful for fork() implementation and shared memory optimization.
//!

#![allow(unused)]

use super::paging::{
    PageTableFlags, VirtAddr, PhysAddr, PAGE_SIZE,
    translate_address, remap_page, get_page_flags,
};
use super::pmm::{alloc_frame, free_frame};
use core::sync::atomic::{AtomicU32, Ordering};

/// Maximum number of physical frames we can track (4GB of RAM / 4KB pages = 1M frames)
/// Adjust this based on your system's maximum memory
const MAX_FRAMES: usize = 1024 * 1024; // 1M frames = 4GB RAM

/// Reference count storage for physical frames
/// Each entry represents the reference count for a 4KB physical frame
static mut REFCOUNT_TABLE: [AtomicU32; MAX_FRAMES] = {
    const ZERO: AtomicU32 = AtomicU32::new(0);
    [ZERO; MAX_FRAMES]
};

/// CoW flag bit - uses bit 9 (available for OS use in page table entries)
/// This bit is in the "available for software" range (bits 9-11)
pub const COW_BIT: u64 = 1 << 9;

/// Initialize the CoW system
pub fn init() {
    unsafe {
        // Initialize all reference counts to 0
        for i in 0..MAX_FRAMES {
            REFCOUNT_TABLE[i].store(0, Ordering::Relaxed);
        }
    }
}

/// Convert physical address to frame index
#[inline]
fn phys_to_frame_index(phys_addr: u64) -> usize {
    (phys_addr / PAGE_SIZE as u64) as usize
}

/// Get reference count for a physical frame
pub fn get_refcount(phys_addr: PhysAddr) -> u32 {
    let index = phys_to_frame_index(phys_addr.as_u64());
    if index >= MAX_FRAMES {
        return 0;
    }
    
    unsafe {
        REFCOUNT_TABLE[index].load(Ordering::Acquire)
    }
}

/// Increment reference count for a physical frame
/// Returns the new reference count
pub fn inc_refcount(phys_addr: PhysAddr) -> u32 {
    let index = phys_to_frame_index(phys_addr.as_u64());
    if index >= MAX_FRAMES {
        return 0;
    }
    
    unsafe {
        REFCOUNT_TABLE[index].fetch_add(1, Ordering::AcqRel) + 1
    }
}

/// Decrement reference count for a physical frame
/// Returns the new reference count
/// If count reaches 0, the frame should be freed
pub fn dec_refcount(phys_addr: PhysAddr) -> u32 {
    let index = phys_to_frame_index(phys_addr.as_u64());
    if index >= MAX_FRAMES {
        return 0;
    }
    
    unsafe {
        let old = REFCOUNT_TABLE[index].fetch_sub(1, Ordering::AcqRel);
        if old > 0 {
            old - 1
        } else {
            0
        }
    }
}

/// Set reference count for a physical frame
pub fn set_refcount(phys_addr: PhysAddr, count: u32) {
    let index = phys_to_frame_index(phys_addr.as_u64());
    if index >= MAX_FRAMES {
        return;
    }
    
    unsafe {
        REFCOUNT_TABLE[index].store(count, Ordering::Release);
    }
}

/// Check if a page table entry has the CoW bit set
#[inline]
pub fn is_cow_page(flags: PageTableFlags) -> bool {
    (flags.raw() & COW_BIT) != 0
}

/// Mark a virtual page as Copy-on-Write
/// This makes the page read-only and sets the CoW bit
pub unsafe fn mark_cow(virt_addr: VirtAddr) -> Result<(), &'static str> {
    // Get current page flags
    let current_flags = get_page_flags(virt_addr)?;
    
    // Get the physical address
    let phys_addr = translate_address(virt_addr)?;
    
    // Create new flags: remove writable, add CoW bit
    let mut new_flags = current_flags;
    new_flags.clear(PageTableFlags::WRITABLE);
    let new_flags_raw = new_flags.raw() | COW_BIT;
    let new_flags = PageTableFlags::from_raw(new_flags_raw);
    
    // Increment reference count for the physical page
    inc_refcount(phys_addr);
    
    // Remap with new flags
    remap_page(virt_addr, phys_addr, new_flags)?;
    
    Ok(())
}

/// Mark a range of virtual pages as Copy-on-Write
pub unsafe fn mark_cow_range(virt_start: VirtAddr, size: usize) -> Result<(), &'static str> {
    let page_count = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    
    for i in 0..page_count {
        let virt = VirtAddr::new(virt_start.as_u64() + (i * PAGE_SIZE) as u64);
        mark_cow(virt)?;
    }
    
    Ok(())
}

/// Handle a Copy-on-Write page fault
/// This is called when a write occurs to a CoW page
/// Returns Ok(()) if the fault was handled, Err otherwise
pub unsafe fn handle_cow_fault(virt_addr: VirtAddr) -> Result<(), &'static str> {
    // Get current page flags
    let current_flags = get_page_flags(virt_addr)?;
    
    // Check if this is actually a CoW page
    if !is_cow_page(current_flags) {
        return Err("Not a CoW page");
    }
    
    // Get the current physical address
    let old_phys_addr = translate_address(virt_addr)?;
    
    // Get reference count
    let refcount = get_refcount(old_phys_addr);
    
    if refcount <= 1 {
        // We're the only reference, just make it writable again
        let mut new_flags = current_flags;
        new_flags.set(PageTableFlags::WRITABLE);
        let new_flags_raw = new_flags.raw() & !COW_BIT; // Clear CoW bit
        let new_flags = PageTableFlags::from_raw(new_flags_raw);
        
        // Decrement reference count
        dec_refcount(old_phys_addr);
        
        // Remap with writable flag
        remap_page(virt_addr, old_phys_addr, new_flags)?;
    } else {
        // Multiple references, need to copy
        
        // Allocate a new physical frame
        let new_phys_frame = alloc_frame();
        if new_phys_frame.unwrap_or_default()== 0 {
            return Err("Failed to allocate frame for CoW");
        }
        let new_phys_addr = PhysAddr::new(new_phys_frame?);
        
        // Copy the page contents
        copy_page(old_phys_addr, new_phys_addr)?;
        
        // Create new flags with writable bit set and CoW bit cleared
        let mut new_flags = current_flags;
        new_flags.set(PageTableFlags::WRITABLE);
        let new_flags_raw = new_flags.raw() & !COW_BIT; // Clear CoW bit
        let new_flags = PageTableFlags::from_raw(new_flags_raw);
        
        // Remap to the new physical address
        remap_page(virt_addr, new_phys_addr, new_flags)?;
        
        // Decrement reference count for old page
        let new_refcount = dec_refcount(old_phys_addr);
        
        // If reference count reached 0, free the old page
        if new_refcount == 0 {
            free_frame(old_phys_addr.as_u64());
        }
        
        // Initialize reference count for new page to 1
        set_refcount(new_phys_addr, 1);
    }
    
    Ok(())
}

/// Copy the contents of one physical page to another
/// Both addresses must be page-aligned
unsafe fn copy_page(src_phys: PhysAddr, dst_phys: PhysAddr) -> Result<(), &'static str> {
    if !src_phys.is_aligned() || !dst_phys.is_aligned() {
        return Err("Physical addresses must be page-aligned");
    }
    
    // Convert physical addresses to virtual addresses
    // Assuming identity mapping or known offset
    let src_virt = phys_to_virt(src_phys.as_u64());
    let dst_virt = phys_to_virt(dst_phys.as_u64());
    
    // Copy page contents (4096 bytes)
    core::ptr::copy_nonoverlapping(
        src_virt as *const u8,
        dst_virt as *mut u8,
        PAGE_SIZE
    );
    
    Ok(())
}

/// Share a page between two virtual addresses (CoW)
/// Both virtual addresses will point to the same physical page with CoW semantics
pub unsafe fn share_page_cow(
    src_virt: VirtAddr,
    dst_virt: VirtAddr,
) -> Result<(), &'static str> {
    // Get the physical address from source
    let phys_addr = translate_address(src_virt)?;
    
    // Get source flags
    let src_flags = get_page_flags(src_virt)?;
    
    // Create CoW flags (read-only with CoW bit)
    let mut cow_flags = src_flags;
    cow_flags.clear(PageTableFlags::WRITABLE);
    let cow_flags_raw = cow_flags.raw() | COW_BIT;
    let cow_flags = PageTableFlags::from_raw(cow_flags_raw);
    
    // Mark source as CoW if not already
    remap_page(src_virt, phys_addr, cow_flags)?;
    
    // Map destination to same physical address with CoW flags
    use super::paging::map_page;
    map_page(dst_virt, phys_addr, cow_flags)?;
    
    // Increment reference count (once for the new mapping)
    // The source should already have a refcount of at least 1
    inc_refcount(phys_addr);
    
    Ok(())
}

/// Clone a page range with CoW semantics
/// Useful for implementing fork()
pub unsafe fn clone_range_cow(
    src_virt_start: VirtAddr,
    dst_virt_start: VirtAddr,
    size: usize,
) -> Result<(), &'static str> {
    let page_count = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    
    for i in 0..page_count {
        let src_virt = VirtAddr::new(src_virt_start.as_u64() + (i * PAGE_SIZE) as u64);
        let dst_virt = VirtAddr::new(dst_virt_start.as_u64() + (i * PAGE_SIZE) as u64);
        
        // Check if source page is mapped
        if let Ok(_) = translate_address(src_virt) {
            share_page_cow(src_virt, dst_virt)?;
        }
    }
    
    Ok(())
}

/// Unshare a CoW page (break CoW sharing)
/// Forces a copy even if there are multiple references
pub unsafe fn unshare_cow_page(virt_addr: VirtAddr) -> Result<(), &'static str> {
    // Get current flags
    let current_flags = get_page_flags(virt_addr)?;
    
    if !is_cow_page(current_flags) {
        // Not a CoW page, nothing to do
        return Ok(());
    }
    
    // Get current physical address
    let old_phys_addr = translate_address(virt_addr)?;
    let refcount = get_refcount(old_phys_addr);
    
    if refcount <= 1 {
        // Only reference, just remove CoW flag and make writable
        let mut new_flags = current_flags;
        new_flags.set(PageTableFlags::WRITABLE);
        let new_flags_raw = new_flags.raw() & !COW_BIT;
        let new_flags = PageTableFlags::from_raw(new_flags_raw);
        
        dec_refcount(old_phys_addr);
        remap_page(virt_addr, old_phys_addr, new_flags)?;
    } else {
        // Multiple references, allocate new page and copy
        let new_phys_frame = alloc_frame();
        if new_phys_frame.unwrap_or_default() == 0 {
            return Err("Failed to allocate frame");
        }
        let new_phys_addr = PhysAddr::new(new_phys_frame?);
        
        // Copy page contents
        copy_page(old_phys_addr, new_phys_addr)?;
        
        // Create writable flags without CoW bit
        let mut new_flags = current_flags;
        new_flags.set(PageTableFlags::WRITABLE);
        let new_flags_raw = new_flags.raw() & !COW_BIT;
        let new_flags = PageTableFlags::from_raw(new_flags_raw);
        
        // Remap to new physical page
        remap_page(virt_addr, new_phys_addr, new_flags)?;
        
        // Update reference counts
        let new_refcount = dec_refcount(old_phys_addr);
        if new_refcount == 0 {
            free_frame(old_phys_addr.as_u64());
        }
        
        set_refcount(new_phys_addr, 1);
    }
    
    Ok(())
}

/// Helper function to convert physical to virtual address
/// This assumes identity mapping or a known offset
/// You may need to adjust this based on your memory layout
#[inline]
unsafe fn phys_to_virt(phys: u64) -> u64 {
    // Common approaches:
    // 1. Identity mapping: phys == virt
    // 2. Offset mapping: virt = phys + KERNEL_OFFSET
    // For now, assume identity mapping in lower memory
    // or use the kernel's physical memory mapping region
    
    // If you have a higher-half kernel, adjust this:
    // phys + 0xFFFF800000000000  // Linux-style
    
    phys  // Assuming identity mapping for now
}

// C-compatible wrappers for integration with existing code

/// Initialize CoW system (C wrapper)
#[no_mangle]
pub extern "C" fn cow_init() {
    init();
}

/// Mark a page as CoW (C wrapper)
/// Returns 0 on success, -1 on error
#[no_mangle]
pub unsafe extern "C" fn cow_mark_page(virt_addr: u64) -> i32 {
    match mark_cow(VirtAddr::new(virt_addr)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Handle CoW page fault (C wrapper)
/// Returns 0 on success, -1 on error
#[no_mangle]
pub unsafe extern "C" fn cow_handle_fault(virt_addr: u64) -> i32 {
    match handle_cow_fault(VirtAddr::new(virt_addr)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get reference count for a physical address (C wrapper)
#[no_mangle]
pub extern "C" fn cow_get_refcount(phys_addr: u64) -> u32 {
    get_refcount(PhysAddr::new(phys_addr))
}

/// Share a page with CoW (C wrapper)
/// Returns 0 on success, -1 on error
#[no_mangle]
pub unsafe extern "C" fn cow_share_page(src_virt: u64, dst_virt: u64) -> i32 {
    match share_page_cow(VirtAddr::new(src_virt), VirtAddr::new(dst_virt)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Clone a range with CoW (C wrapper)
/// Returns 0 on success, -1 on error
#[no_mangle]
pub unsafe extern "C" fn cow_clone_range(
    src_virt_start: u64,
    dst_virt_start: u64,
    size: u64,
) -> i32 {
    match clone_range_cow(
        VirtAddr::new(src_virt_start),
        VirtAddr::new(dst_virt_start),
        size as usize,
    ) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would need to run in a kernel environment
    // with proper memory management initialized
    
    #[test]
    fn test_refcount_basic() {
        init();
        let phys = PhysAddr::new(0x1000);
        
        assert_eq!(get_refcount(phys), 0);
        assert_eq!(inc_refcount(phys), 1);
        assert_eq!(inc_refcount(phys), 2);
        assert_eq!(dec_refcount(phys), 1);
        assert_eq!(dec_refcount(phys), 0);
    }

    #[test]
    fn test_cow_bit() {
        let mut flags = PageTableFlags::kernel();
        assert!(!is_cow_page(flags));
        
        let flags = PageTableFlags::from_raw(flags.raw() | COW_BIT);
        assert!(is_cow_page(flags));
    }
}
