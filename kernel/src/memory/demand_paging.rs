//! Demand Paging (lazy allocation)
//!
//! This module implements demand paging for anonymous memory. It keeps a small
//! registry of valid virtual ranges and allocates/maps pages the first time
//! they are accessed (page fault on not-present PTE).

#![allow(unused)]

use core::ptr;

use super::paging::{map_page, PageTableFlags, PhysAddr, VirtAddr, PAGE_SIZE};
use super::pmm::alloc_frame;

/// Maximum number of demand-paged regions we can track without a heap.
const MAX_REGIONS: usize = 64;

/// Backing source for a demand-paged region.
///
/// TODO: Only zero-fill is implemented right now; file-backed is a placeholder for
/// future mmap support.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackingKind {
    /// Allocate a zeroed page on first access.
    ZeroFill,
    /// TODO: File-backed pages (future).
    FileBacked,
}

/// A demand-paged virtual memory region.
#[derive(Clone, Copy, Debug)]
pub struct VmRegion {
    pub start: u64,
    pub end: u64,
    pub flags: PageTableFlags,
    pub backing: BackingKind,
}

/// TODO: Global region table (simple fixed-size registry).
static mut REGIONS: [Option<VmRegion>; MAX_REGIONS] = [None; MAX_REGIONS];

/// Align an address down to a page boundary.
#[inline]
fn align_down(addr: u64) -> u64 {
    addr & !(PAGE_SIZE as u64 - 1)
}

/// Align an address up to a page boundary.
#[inline]
fn align_up(addr: u64) -> u64 {
    (addr + PAGE_SIZE as u64 - 1) & !(PAGE_SIZE as u64 - 1)
}

/// Convert a physical address to a virtual address using the identity map.
///
/// This matches the paging module assumption that the first 128 MiB are
/// identity-mapped for kernel use.
#[inline]
unsafe fn phys_to_virt(phys_addr: u64) -> *mut u8 {
    if phys_addr >= 0x8_000_000 {
        ptr::null_mut()
    } else {
        phys_addr as *mut u8
    }
}

/// Register a demand-paged region.
///
/// The region is reserved logically but not mapped until accessed.
pub fn register_region(
    start: VirtAddr,
    size: usize,
    flags: PageTableFlags,
    backing: BackingKind,
) -> Result<(), &'static str> {
    let start_addr = align_down(start.as_u64());
    let end_addr = align_up(
        start
            .as_u64()
            .checked_add(size as u64)
            .ok_or("Size overflow")?,
    );

    if end_addr <= start_addr {
        return Err("Invalid region size");
    }

    unsafe {
        for slot in REGIONS.iter_mut() {
            if slot.is_none() {
                *slot = Some(VmRegion {
                    start: start_addr,
                    end: end_addr,
                    flags,
                    backing,
                });
                return Ok(());
            }
        }
    }

    Err("No free region slots")
}

/// Check whether a faulting address is within a registered region.
fn find_region(addr: u64) -> Option<VmRegion> {
    unsafe {
        for slot in &REGIONS {
            if let Some(region) = slot {
                if addr >= region.start && addr < region.end {
                    return Some(*region);
                }
            }
        }
    }
    None
}

/// Handle a not-present page fault for a demand-paged region.
///
/// Returns Ok(()) if the fault was handled and the page mapped.
pub fn handle_demand_page_fault(fault_addr: VirtAddr, error_code: u64) -> Result<(), &'static str> {
    // Bit 0 set means the page was present (protection fault), not a demand fault.
    if (error_code & 0x1) != 0 {
        return Err("Page was present");
    }

    let region = find_region(fault_addr.as_u64()).ok_or("Address not in demand region")?;
    let page_base = align_down(fault_addr.as_u64());

    // Allocate a physical frame for the new page.
    let frame = alloc_frame().map_err(|_| "Failed to allocate frame")?;
    let phys = PhysAddr::new(frame);

    // Prepare the page content based on the backing kind.
    unsafe {
        match region.backing {
            BackingKind::ZeroFill => {
                let virt = phys_to_virt(phys.as_u64());
                if virt.is_null() {
                    return Err("No identity mapping for frame");
                }
                ptr::write_bytes(virt, 0, PAGE_SIZE);
            }
            BackingKind::FileBacked => {
                return Err("File-backed paging not implemented");
            }
        }
    }

    // Ensure the present bit is set in the final mapping.
    let mut map_flags = region.flags;
    map_flags.set(PageTableFlags::PRESENT);

    unsafe {
        map_page(VirtAddr::new(page_base), phys, map_flags)?;
    }

    Ok(())
}
