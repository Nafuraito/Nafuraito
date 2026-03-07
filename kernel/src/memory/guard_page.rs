//! Guard Page subsystem for stack overflow detection
//!
//! # How it works
//!
//! A **guard page** is a single 4 KiB virtual page placed at the *bottom*
//! (lowest address) of every kernel stack.  It is deliberately **not mapped**
//! in the page tables (its PTE is zero / not-present).
//!
//! When the stack grows down past the usable region and touches the guard page
//! the CPU finds `PRESENT = 0` in the PTE, raises a `#PF` (vector 14), and
//! saves the faulting address in `CR2`.  The page-fault handler reads `CR2`,
//! looks it up in the guard-page registry, and prints a "STACK OVERFLOW" panic
//! rather than treating it as an ordinary demand-paging miss.
//!
//! # Stack layout produced by `allocate_kernel_stack`
//!
//! ```text
//! High address  ┌────────────────────────────────┐  ← returned RSP (initial stack top)
//!               │  stack_pages × 4 KiB           │  PRESENT | WRITABLE | NO_EXECUTE
//!               │  (grows downward  ↓ )           │
//! Low address   ├────────────────────────────────┤
//!               │  1 guard page  (NOT MAPPED)     │  PTE = 0 → triggers #PF on touch
//!               └────────────────────────────────┘  ← base of allocation
//! ```
//!
//! # Safety notes
//!
//! - The guard page registry is protected only by the single-threaded
//!   early-boot invariant.  Before SMP is enabled this module must be
//!   extended with a spinlock.
//! - `allocate_kernel_stack` uses a bump allocator anchored at
//!   `KERNEL_STACK_AREA_BASE`.  There is no free-list; stacks are never
//!   reclaimed.  This is intentional until a proper task-teardown path exists.
//! - All virtual addresses used by this module must lie within the identity
//!   map established by the bootloader (below 4 GiB for DMA32 frames).

#![allow(unused)]

use core::sync::atomic::{AtomicUsize, Ordering};

use super::paging::{map_page, PageTableFlags, PhysAddr, VirtAddr, PAGE_SIZE};

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of guard pages tracked simultaneously.
///
/// Fixed at compile time because there is no heap allocator yet.
/// Each kernel task requires one slot; raise this constant as needed.
pub const MAX_GUARD_PAGES: usize = 64;

/// Default number of usable 4 KiB pages per kernel stack (excludes guard page).
///
/// 4 pages = 16 KiB of usable stack; sufficient for early kernel threads.
/// Increase when deeper call chains are needed.
pub const DEFAULT_STACK_PAGES: usize = 4;

/// Virtual base address of the kernel stack area.
///
/// Layout (grows upward, one slot = guard page + DEFAULT_STACK_PAGES):
///   0x0200_0000 … 0x02FF_FFFF  (16 MiB region)
///
/// NOTE: Must not overlap kernel code/data (loaded at 0x100000) or the
/// hardcoded IST emergency stacks (0x8A0000–0x900000) in `gdt/lib.rs`.
pub const KERNEL_STACK_AREA_BASE: u64 = 0x0200_0000; // 32 MiB mark

// =============================================================================
// Guard page registry
// =============================================================================

/// Table of registered guard-page virtual addresses (page-aligned base).
///
/// A zero entry means the slot is empty.  Slots are never freed.
///
/// NOTE: Unprotected; safe only under the single-CPU early-boot assumption.
static mut GUARD_REGISTRY: [u64; MAX_GUARD_PAGES] = [0u64; MAX_GUARD_PAGES];

/// Number of filled entries in `GUARD_REGISTRY`.
static GUARD_REGISTRY_COUNT: AtomicUsize = AtomicUsize::new(0);

// =============================================================================
// Bump pointer for stack virtual-address allocation
// =============================================================================

/// Byte offset into `KERNEL_STACK_AREA_BASE` for the next stack allocation.
///
/// Incremented by `(stack_pages + 1) * PAGE_SIZE` on every call.
static NEXT_STACK_OFFSET: AtomicUsize = AtomicUsize::new(0);

// =============================================================================
// Public Rust API
// =============================================================================

/// Register `virt_addr` as a guard page.
///
/// The caller is responsible for ensuring the page is **not** present in the
/// active page table – this function only updates the software registry; it
/// does NOT touch the page tables.
///
/// Returns `true` on success, `false` if the registry is full.
///
/// # Safety
/// `virt_addr` should be page-aligned.  Misalignment is silently rounded down.
pub unsafe fn register_guard_page(virt_addr: u64) -> bool {
    let page_base = virt_addr & !(PAGE_SIZE as u64 - 1); // round down to page boundary

    let idx = GUARD_REGISTRY_COUNT.load(Ordering::Relaxed);
    if idx >= MAX_GUARD_PAGES {
        // Registry is full – this stack will have no overflow detection.
        // NOTE: This should never happen under normal conditions; increase
        //       MAX_GUARD_PAGES if more than 64 kernel stacks are needed.
        return false;
    }

    // SAFETY: We write slot `idx` before incrementing the count, so concurrent
    //         readers (via `is_guard_page`) will never observe an uninitialised slot.
    GUARD_REGISTRY[idx] = page_base;
    GUARD_REGISTRY_COUNT.store(idx + 1, Ordering::Release);
    true
}

/// Return `true` if `addr` falls within any registered guard page.
///
/// Called from the `#PF` handler to distinguish stack-overflow faults from
/// ordinary not-present faults (demand paging, CoW, etc.).
///
/// # Performance
/// Linear scan over at most `MAX_GUARD_PAGES` entries.  Acceptable for the
/// small number of kernel stacks present during early boot.
pub fn is_guard_page(addr: u64) -> bool {
    let page_base = addr & !(PAGE_SIZE as u64 - 1); // round down to page boundary

    // SAFETY: We use Acquire ordering so we see all writes that happened before
    //         the corresponding Release store in `register_guard_page`.
    let count = GUARD_REGISTRY_COUNT.load(Ordering::Acquire);

    // SAFETY: We only read slots in [0, count), each of which was fully written
    //         before `count` was incremented.  `count` is always <= MAX_GUARD_PAGES
    //         (enforced in `register_guard_page`), so `get_unchecked` is safe.
    unsafe {
        for i in 0..count {
            if *GUARD_REGISTRY.get_unchecked(i) == page_base {
                return true;
            }
        }
    }
    false
}

/// Allocate a kernel stack backed by physical frames, with a guard page at its
/// bottom (lowest virtual address).
///
/// Each call bumps the virtual-address pointer forward by
/// `(stack_pages + 1) * PAGE_SIZE` bytes, so successive stacks never overlap.
///
/// # Returns
/// The **top** of the usable stack (initial RSP value) on success, or `0` on
/// failure (PMM exhausted or registry full without a critical error).
///
/// # Safety
/// Requires that both the PMM and the paging subsystem are fully initialised.
/// Must not be called from interrupt context.
pub unsafe fn allocate_kernel_stack(stack_pages: usize) -> u64 {
    // One slot = 1 guard page (unmapped) + stack_pages usable pages.
    let slot_size = (stack_pages + 1) * PAGE_SIZE;

    // Bump-allocate a virtual address range for this slot.
    let slot_offset = NEXT_STACK_OFFSET.fetch_add(slot_size, Ordering::Relaxed);
    let base_virt: u64 = KERNEL_STACK_AREA_BASE + slot_offset as u64;

    // -------------------------------------------------------------------------
    // Guard page: the page at `base_virt` is intentionally left unmapped.
    // The page-fault handler will recognise accesses to it (via CR2) as stack
    // overflows by looking it up in the software registry.
    // -------------------------------------------------------------------------
    let guard_virt = base_virt;
    if !register_guard_page(guard_virt) {
        // Registry full.  We still allocate the stack but overflow detection
        // will not work for this particular stack.
        // NOTE: Consider panicking here once the registry size is known-good.
    }

    // -------------------------------------------------------------------------
    // Usable stack pages: mapped PRESENT | WRITABLE | NO_EXECUTE.
    // Physical frames are allocated from the PMM one at a time.
    // -------------------------------------------------------------------------
    for i in 0..stack_pages {
        // Usable pages start one page above the guard page.
        let virt = VirtAddr::new(base_virt + PAGE_SIZE as u64 + (i * PAGE_SIZE) as u64);

        let phys = alloc_stack_frame();
        if phys == 0 {
            // PMM exhausted – return 0 to signal failure.
            // NOTE: Partially mapped stacks are intentionally left in place;
            //       unmapping them is safe but not implemented here.
            return 0;
        }

        // Kernel-only page: present, writable, global, non-executable.
        let mut flags = PageTableFlags::kernel(); // PRESENT | WRITABLE | GLOBAL
        flags.set(PageTableFlags::NO_EXECUTE); // stack pages must never be executable

        if let Err(_) = map_page(virt, PhysAddr::new(phys), flags) {
            // Mapping failed (e.g. page already mapped, or OOM for page tables).
            return 0;
        }
    }

    // Return the top of the usable stack (initial RSP).
    // RSP points one byte past the last stack page; the first `push` instruction
    // will decrement it by 8 before writing, landing inside the last page.
    base_virt + (stack_pages as u64 + 1) * PAGE_SIZE as u64
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Allocate one 4 KiB physical frame from the PMM.
///
/// Prefers DMA32-zone frames (<4 GiB) so they are always reachable via the
/// bootloader's identity map.
///
/// Returns `0` if the PMM is exhausted.
unsafe fn alloc_stack_frame() -> u64 {
    extern "C" {
        fn pmm_alloc_dma32_frame() -> u64;
        fn pmm_alloc_frame() -> u64;
    }

    // Try DMA32 first (avoids needing high-memory mappings for page tables).
    let frame = pmm_alloc_dma32_frame();
    if frame != 0 {
        return frame;
    }

    // Fall back to any available frame.
    pmm_alloc_frame()
}

// =============================================================================
// C-compatible exports
// =============================================================================
//
// These allow `main.rs` (compiled as a separate staticlib) to call into this
// subsystem through the standard C ABI without Rust-to-Rust cross-crate links.

/// C export: initialise the guard page subsystem.
///
/// Must be called once after both the PMM and the paging subsystem are online.
/// Currently a no-op because all state is statically initialised, but reserved
/// for future setup work (e.g. allocating the registry from a slab allocator).
#[no_mangle]
pub unsafe extern "C" fn guard_page_init() {
    // Nothing to do yet – static initialisers handle all setup.
}

/// C export: register `virt_addr` as a guard page.
///
/// Returns `1` on success, `0` if the registry is full.
#[no_mangle]
pub unsafe extern "C" fn guard_page_register(virt_addr: u64) -> i32 {
    if register_guard_page(virt_addr) {
        1
    } else {
        0
    }
}

/// C export: return `1` if `addr` falls within a registered guard page, `0` otherwise.
///
/// Called from `exception_handler` in `main.rs` on every not-present page fault.
#[no_mangle]
pub extern "C" fn guard_page_is_guard_page(addr: u64) -> i32 {
    if is_guard_page(addr) {
        1
    } else {
        0
    }
}

/// C export: allocate a kernel stack with a guard page using `DEFAULT_STACK_PAGES`.
///
/// Returns the initial RSP value (stack top address), or `0` on failure.
#[no_mangle]
pub unsafe extern "C" fn guard_page_alloc_kernel_stack() -> u64 {
    allocate_kernel_stack(DEFAULT_STACK_PAGES)
}
