//! Slab Allocation Primitives
//! =============================================================================
//!
//! This module contains the low-level logic for slab pages:
//! - size class selection
//! - per-page free-list setup
//! - object allocation and free within a slab page
//!
//! NOTE:
//! The higher-level heap manager (page provisioning, locking, and ownership
//! tracking) lives in `heap.rs`.  This split keeps slab logic reusable and easy
//! to test in isolation.
//! =============================================================================

#![allow(unused)]

use core::mem;
use core::ptr;

/// Standard kernel page size (4 KiB).
pub const PAGE_SIZE: usize = 4096;

/// Number of slab size classes.
pub const SLAB_CLASS_COUNT: usize = 8;

/// Slab size classes (bytes), intentionally power-of-two for fast alignment.
pub const SLAB_CLASS_SIZES: [usize; SLAB_CLASS_COUNT] = [16, 32, 64, 128, 256, 512, 1024, 2048];

/// End marker for the in-object free-list (`u16::MAX` means no next object).
pub const FREE_LIST_END: u16 = u16::MAX;

/// Metadata for one slab page.
///
/// One slab page only contains objects from one class size.
#[derive(Debug, Clone, Copy)]
pub struct SlabPage {
    /// Whether this metadata entry is currently active.
    pub active: bool,
    /// Which class index from `SLAB_CLASS_SIZES` this slab uses.
    pub class_index: u8,
    /// Head index of the free-list.
    pub free_head: u16,
    /// Number of objects currently allocated from this slab.
    pub in_use: u16,
    /// Total number of objects that fit in this slab.
    pub capacity: u16,
}

impl SlabPage {
    /// Construct an inactive slab metadata entry.
    pub const fn empty() -> Self {
        Self {
            active: false,
            class_index: 0,
            free_head: FREE_LIST_END,
            in_use: 0,
            capacity: 0,
        }
    }

    /// Returns true if at least one object is available.
    #[inline(always)]
    pub const fn has_free_objects(&self) -> bool {
        self.active && self.free_head != FREE_LIST_END
    }

    /// Returns true if all objects are currently free.
    #[inline(always)]
    pub const fn is_empty(&self) -> bool {
        self.active && self.in_use == 0
    }
}

/// Pick the smallest slab class that can satisfy size+alignment.
#[inline]
pub fn class_index_for(size: usize, align: usize) -> Option<usize> {
    // The free-list stores a `u16 next` inside free objects, so each object
    // needs at least two bytes even if the caller asks for less.
    let required = size.max(align).max(mem::size_of::<u16>());

    for (index, class_size) in SLAB_CLASS_SIZES.iter().enumerate() {
        if required <= *class_size {
            return Some(index);
        }
    }

    None
}

/// Get the object size for a class index.
#[inline]
pub fn class_size(class_index: usize) -> usize {
    if class_index >= SLAB_CLASS_COUNT {
        return 0;
    }
    unsafe { *SLAB_CLASS_SIZES.get_unchecked(class_index) }
}

/// Initialize a new slab page over the provided page memory.
///
/// SAFETY:
/// - `page_ptr` must point to a writable 4 KiB page.
/// - No live allocations may currently exist in that page.
pub unsafe fn init_slab_page(page_ptr: *mut u8, class_index: usize, slab: &mut SlabPage) {
    let obj_size = class_size(class_index);
    let capacity = (PAGE_SIZE / obj_size) as u16;

    slab.active = true;
    slab.class_index = class_index as u8;
    slab.in_use = 0;
    slab.capacity = capacity;
    slab.free_head = if capacity > 0 { 0 } else { FREE_LIST_END };

    if capacity == 0 {
        return;
    }

    // Build a singly-linked free-list directly inside free objects.
    // Each free object's first two bytes store the next object index.
    let mut i: u16 = 0;
    while i < capacity {
        let next = if i + 1 < capacity { i + 1 } else { FREE_LIST_END };
        let obj_ptr = page_ptr.add((i as usize) * obj_size);
        ptr::write_unaligned(obj_ptr.cast::<u16>(), next);
        i += 1;
    }
}

/// Allocate one object from an initialized slab page.
///
/// Returns null if the slab is full.
///
/// SAFETY:
/// - `page_ptr` must be the base pointer for this slab page.
/// - `slab` must correspond to that page.
pub unsafe fn alloc_from_slab(page_ptr: *mut u8, slab: &mut SlabPage) -> *mut u8 {
    if !slab.has_free_objects() {
        return core::ptr::null_mut();
    }

    let obj_size = class_size(slab.class_index as usize);
    if obj_size == 0 {
        return core::ptr::null_mut();
    }
    let index = slab.free_head;
    let obj_ptr = page_ptr.add((index as usize) * obj_size);

    // Pop from free-list: next index is stored in the freed object itself.
    let next = ptr::read_unaligned(obj_ptr.cast::<u16>());
    slab.free_head = next;
    slab.in_use = slab.in_use.saturating_add(1);

    obj_ptr
}

/// Free one object back into a slab page.
///
/// Returns false if the pointer is invalid for this slab.
///
/// SAFETY:
/// - `page_ptr` must be the base pointer for this slab page.
/// - `slab` must correspond to that page.
/// - `ptr` must either be null or a pointer previously returned by this slab.
pub unsafe fn free_to_slab(page_ptr: *mut u8, slab: &mut SlabPage, ptr: *mut u8) -> bool {
    if ptr.is_null() {
        return true;
    }

    if !slab.active {
        return false;
    }

    let obj_size = class_size(slab.class_index as usize);
    if obj_size == 0 {
        return false;
    }
    let page_start = page_ptr as usize;
    let page_end = page_start + PAGE_SIZE;
    let p = ptr as usize;

    // Validate pointer range.
    if p < page_start || p >= page_end {
        return false;
    }

    let offset = p - page_start;
    if offset % obj_size != 0 {
        return false;
    }

    let obj_index = (offset / obj_size) as u16;
    if obj_index >= slab.capacity {
        return false;
    }

    // Push object index back to free-list.
    let obj_ptr = page_ptr.add(offset);
    ptr::write_unaligned(obj_ptr.cast::<u16>(), slab.free_head);
    slab.free_head = obj_index;
    slab.in_use = slab.in_use.saturating_sub(1);

    true
}
