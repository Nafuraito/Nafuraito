//! Kernel Heap Manager (Slab + Page Fallback)
//! =============================================================================
//!
//! This module provides the runtime heap used by:
//! - Rust `alloc` global allocator hooks
//! - C-compatible `kmalloc` / `kfree` wrappers
//!
//! Design:
//! - Small allocations (<= 2048 bytes) use slab caches.
//! - Large allocations use contiguous page runs from the same arena.
//! - A spinlock protects allocator global state.
//!
//! NOTE:
//! This is an in-kernel heap arena implementation designed to replace the
//! previous bump-only allocator. It is intentionally conservative and fully
//! metadata-driven so `kfree(ptr)` works without a size parameter.
//! =============================================================================

#![allow(unused)]

use core::alloc::Layout;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

use super::slab::{
    alloc_from_slab,
    class_index_for,
    free_to_slab,
    init_slab_page,
    SlabPage,
    PAGE_SIZE,
};

/// Size of the managed kernel heap arena.
///
/// This arena is used by both slab pages and large page-backed allocations.
const HEAP_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

/// Total number of 4 KiB pages in the heap arena.
const HEAP_PAGE_COUNT: usize = HEAP_SIZE / PAGE_SIZE;

/// Page kind markers for ownership tracking.
const PAGE_KIND_FREE: u8 = 0;
const PAGE_KIND_SLAB: u8 = 1;
const PAGE_KIND_LARGE: u8 = 2;

/// Magic constant used to validate large-allocation headers.
const LARGE_MAGIC: u32 = 0x4E41_4655; // "NAFU"

/// Arena storage aligned to page size so page indexing is trivial.
#[repr(align(4096))]
struct HeapArena([u8; HEAP_SIZE]);

/// Header written to the first page of each large allocation run.
#[repr(C)]
#[derive(Clone, Copy)]
struct LargeAllocHeader {
    magic: u32,
    pages: u16,
    _reserved: u16,
}

/// Global heap allocator state.
struct HeapState {
    initialized: bool,

    // One bit per page: 1 means allocated.
    page_bitmap: [u8; HEAP_PAGE_COUNT / 8],

    // Per-page ownership classification and owner mapping.
    page_kind: [u8; HEAP_PAGE_COUNT],
    page_head: [u16; HEAP_PAGE_COUNT],

    // Slab metadata indexed by page index when `page_kind == PAGE_KIND_SLAB`.
    slabs: [SlabPage; HEAP_PAGE_COUNT],
}

impl HeapState {
    /// Build a clean, empty heap state.
    const fn new() -> Self {
        Self {
            initialized: false,
            page_bitmap: [0; HEAP_PAGE_COUNT / 8],
            page_kind: [PAGE_KIND_FREE; HEAP_PAGE_COUNT],
            page_head: [u16::MAX; HEAP_PAGE_COUNT],
            slabs: [SlabPage::empty(); HEAP_PAGE_COUNT],
        }
    }

    /// Reset all bookkeeping structures to an empty heap.
    fn reset(&mut self) {
        // NOTE:
        // We intentionally clear arrays with explicit loops instead of bulk
        // assignments like `[0; N]`. In this bare-metal binary format build,
        // those bulk ops can compile to helper-call sequences that are brittle
        // during very early runtime. Straight loops keep the generated machine
        // code simple and predictable.

        let mut i = 0usize;
        while i < (HEAP_PAGE_COUNT / 8) {
            unsafe {
                ptr::write_volatile(self.page_bitmap.get_unchecked_mut(i), 0);
            }
            i += 1;
        }

        i = 0;
        while i < HEAP_PAGE_COUNT {
            unsafe {
                ptr::write_volatile(self.page_kind.get_unchecked_mut(i), PAGE_KIND_FREE);
                ptr::write_volatile(self.page_head.get_unchecked_mut(i), u16::MAX);
                ptr::write_volatile(self.slabs.get_unchecked_mut(i), SlabPage::empty());
            }
            i += 1;
        }

        self.initialized = true;
    }
}

/// Minimal spinlock for allocator state.
struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    const fn new() -> Self {
        Self {
            locked: AtomicBool::new(false),
        }
    }

    fn lock(&self) -> SpinLockGuard<'_> {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        SpinLockGuard { lock: self }
    }
}

/// RAII lock guard.
struct SpinLockGuard<'a> {
    lock: &'a SpinLock,
}

impl Drop for SpinLockGuard<'_> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

static HEAP_LOCK: SpinLock = SpinLock::new();
static HEAP_READY: AtomicBool = AtomicBool::new(false);

static mut HEAP_ARENA: HeapArena = HeapArena([0; HEAP_SIZE]);
static mut HEAP_STATE: HeapState = HeapState::new();

#[inline(always)]
fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[inline(always)]
fn arena_base() -> usize {
    unsafe { HEAP_ARENA.0.as_ptr() as usize }
}

#[inline(always)]
fn is_page_used(state: &HeapState, page: usize) -> bool {
    let byte = page / 8;
    let bit = page % 8;
    unsafe { (*state.page_bitmap.get_unchecked(byte) & (1u8 << bit)) != 0 }
}

#[inline(always)]
fn set_page_used(state: &mut HeapState, page: usize, used: bool) {
    let byte = page / 8;
    let bit = page % 8;
    if used {
        unsafe {
            *state.page_bitmap.get_unchecked_mut(byte) |= 1u8 << bit;
        }
    } else {
        unsafe {
            *state.page_bitmap.get_unchecked_mut(byte) &= !(1u8 << bit);
        }
    }
}

#[inline(always)]
unsafe fn page_kind_get(state: &HeapState, page: usize) -> u8 {
    *state.page_kind.get_unchecked(page)
}

#[inline(always)]
unsafe fn page_kind_set(state: &mut HeapState, page: usize, kind: u8) {
    *state.page_kind.get_unchecked_mut(page) = kind;
}

#[inline(always)]
unsafe fn page_head_get(state: &HeapState, page: usize) -> u16 {
    *state.page_head.get_unchecked(page)
}

#[inline(always)]
unsafe fn page_head_set(state: &mut HeapState, page: usize, head: u16) {
    *state.page_head.get_unchecked_mut(page) = head;
}

#[inline(always)]
unsafe fn slab_get_mut(state: &mut HeapState, page: usize) -> &mut SlabPage {
    state.slabs.get_unchecked_mut(page)
}

/// Find `count` contiguous free pages.
fn alloc_contiguous_pages(state: &mut HeapState, count: usize) -> Option<usize> {
    if count == 0 || count > HEAP_PAGE_COUNT {
        return None;
    }

    let mut run_start = 0usize;
    let mut run_len = 0usize;

    let mut page = 0usize;
    while page < HEAP_PAGE_COUNT {
        if !is_page_used(state, page) {
            if run_len == 0 {
                run_start = page;
            }
            run_len += 1;
            if run_len == count {
                let mut i = 0usize;
                while i < count {
                    set_page_used(state, run_start + i, true);
                    i += 1;
                }
                return Some(run_start);
            }
        } else {
            run_len = 0;
        }
        page += 1;
    }

    None
}

/// Release `count` pages starting at `start`.
fn free_contiguous_pages(state: &mut HeapState, start: usize, count: usize) {
    let mut i = 0usize;
    while i < count && (start + i) < HEAP_PAGE_COUNT {
        let page = start + i;
        set_page_used(state, page, false);
        unsafe {
            page_kind_set(state, page, PAGE_KIND_FREE);
            page_head_set(state, page, u16::MAX);
            *slab_get_mut(state, page) = SlabPage::empty();
        }
        i += 1;
    }
}

#[inline(always)]
fn page_ptr(page_index: usize) -> *mut u8 {
    unsafe { HEAP_ARENA.0.as_mut_ptr().add(page_index * PAGE_SIZE) }
}

/// Allocate from slab classes.
fn alloc_small(state: &mut HeapState, size: usize, align: usize) -> *mut u8 {
    let class_index = match class_index_for(size, align) {
        Some(idx) => idx,
        None => return core::ptr::null_mut(),
    };

    // First pass: try existing slabs for this class that still have free objects.
    let mut page = 0usize;
    while page < HEAP_PAGE_COUNT {
        if unsafe { page_kind_get(state, page) } == PAGE_KIND_SLAB {
            let slab = unsafe { slab_get_mut(state, page) };
            if slab.active && slab.class_index as usize == class_index && slab.has_free_objects() {
                let ptr = unsafe { alloc_from_slab(page_ptr(page), slab) };
                if !ptr.is_null() {
                    return ptr;
                }
            }
        }
        page += 1;
    }

    // No existing slab had space; allocate one fresh page and initialize it.
    let new_page = match alloc_contiguous_pages(state, 1) {
        Some(p) => p,
        None => return core::ptr::null_mut(),
    };

    unsafe {
        page_kind_set(state, new_page, PAGE_KIND_SLAB);
        page_head_set(state, new_page, new_page as u16);
    }

    let slab = unsafe { slab_get_mut(state, new_page) };
    unsafe {
        init_slab_page(page_ptr(new_page), class_index, slab);
        alloc_from_slab(page_ptr(new_page), slab)
    }
}

/// Allocate large objects via page runs + header metadata.
fn alloc_large(state: &mut HeapState, size: usize, align: usize) -> *mut u8 {
    let header_size = core::mem::size_of::<LargeAllocHeader>();
    let bytes_needed = size.saturating_add(align).saturating_add(header_size);
    let pages = (bytes_needed + PAGE_SIZE - 1) / PAGE_SIZE;

    if pages == 0 || pages > u16::MAX as usize {
        return core::ptr::null_mut();
    }

    let head_page = match alloc_contiguous_pages(state, pages) {
        Some(p) => p,
        None => return core::ptr::null_mut(),
    };

    let base_ptr = page_ptr(head_page);

    // Mark all pages in this run as belonging to one large allocation.
    let mut i = 0usize;
    while i < pages {
        let page = head_page + i;
        unsafe {
            page_kind_set(state, page, PAGE_KIND_LARGE);
            page_head_set(state, page, head_page as u16);
        }
        i += 1;
    }

    unsafe {
        // Header sits at the start of the run so free(ptr) can recover size.
        let header = base_ptr.cast::<LargeAllocHeader>();
        (*header).magic = LARGE_MAGIC;
        (*header).pages = pages as u16;
        (*header)._reserved = 0;

        // User pointer is aligned after the header.
        let raw = base_ptr as usize + header_size;
        let aligned = align_up(raw, align.max(1));
        aligned as *mut u8
    }
}

/// Shared allocation implementation.
fn alloc_inner(state: &mut HeapState, size: usize, align: usize) -> *mut u8 {
    if size == 0 {
        return core::ptr::null_mut();
    }

    // Slab classes handle small + modest alignment requests.
    if class_index_for(size, align).is_some() {
        let ptr = alloc_small(state, size, align);
        if !ptr.is_null() {
            return ptr;
        }
    }

    // Fallback for larger or unusual alignments.
    alloc_large(state, size, align)
}

/// Shared free implementation.
fn free_inner(state: &mut HeapState, ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }

    let base = arena_base();
    let end = base + HEAP_SIZE;
    let p = ptr as usize;

    // Pointer outside this heap arena: ignore safely.
    if p < base || p >= end {
        return;
    }

    let page = (p - base) / PAGE_SIZE;
    if page >= HEAP_PAGE_COUNT {
        return;
    }
    let kind = unsafe { page_kind_get(state, page) };

    if kind == PAGE_KIND_SLAB {
        let head_page = unsafe { page_head_get(state, page) } as usize;
        if head_page >= HEAP_PAGE_COUNT {
            return;
        }

        let slab = unsafe { slab_get_mut(state, head_page) };
        let ok = unsafe { free_to_slab(page_ptr(head_page), slab, ptr) };
        if !ok {
            return;
        }

        // Optional reclaim policy: if a slab becomes empty, release its page.
        if slab.is_empty() {
            free_contiguous_pages(state, head_page, 1);
        }
        return;
    }

    if kind == PAGE_KIND_LARGE {
        let head_page = unsafe { page_head_get(state, page) } as usize;
        if head_page >= HEAP_PAGE_COUNT {
            return;
        }

        unsafe {
            let header = page_ptr(head_page).cast::<LargeAllocHeader>();
            if (*header).magic != LARGE_MAGIC {
                return;
            }
            let pages = (*header).pages as usize;
            free_contiguous_pages(state, head_page, pages);
        }
    }
}

/// Initialize heap subsystem.
pub fn heap_init() -> Result<(), &'static str> {
    let _guard = HEAP_LOCK.lock();
    unsafe {
        HEAP_STATE.reset();
    }
    HEAP_READY.store(true, Ordering::Release);
    Ok(())
}

/// Query whether heap initialization has completed.
#[inline(always)]
pub fn heap_is_initialized() -> bool {
    HEAP_READY.load(Ordering::Acquire)
}

/// Allocate memory from the kernel heap.
pub fn heap_alloc(layout: Layout) -> *mut u8 {
    if !heap_is_initialized() {
        return core::ptr::null_mut();
    }

    let _guard = HEAP_LOCK.lock();
    unsafe { alloc_inner(&mut HEAP_STATE, layout.size(), layout.align()) }
}

/// Free memory previously allocated by `heap_alloc`.
pub fn heap_dealloc(ptr: *mut u8) {
    if !heap_is_initialized() {
        return;
    }

    let _guard = HEAP_LOCK.lock();
    unsafe {
        free_inner(&mut HEAP_STATE, ptr);
    }
}

// =============================================================================
// C-compatible wrappers (used by kernel entry crate)
// =============================================================================

/// Initialize heap allocator. Returns 0 on success, negative error code on failure.
#[no_mangle]
pub extern "C" fn heap_init_c() -> i32 {
    match heap_init() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Returns 1 if heap is initialized, 0 otherwise.
#[no_mangle]
pub extern "C" fn heap_is_initialized_c() -> u8 {
    if heap_is_initialized() {
        1
    } else {
        0
    }
}

/// Allocate `size` bytes with `align` alignment. Returns null on failure.
#[no_mangle]
pub extern "C" fn heap_alloc_c(size: usize, align: usize) -> *mut u8 {
    let safe_align = align.max(1);
    let layout = match Layout::from_size_align(size, safe_align) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };
    heap_alloc(layout)
}

/// Free pointer allocated by `heap_alloc_c` / Rust global allocator bridge.
#[no_mangle]
pub extern "C" fn heap_dealloc_c(ptr: *mut u8) {
    heap_dealloc(ptr);
}
