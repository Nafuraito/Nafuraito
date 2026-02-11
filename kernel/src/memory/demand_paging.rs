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

/// File-backed read callback used by memory-mapped regions.
///
/// The callback must read up to `len` bytes at `offset` into `dst` and return
/// the number of bytes actually read.
pub type MmapReadAtFn = extern "C" fn(ctx: *mut u8, offset: u64, dst: *mut u8, len: usize) -> usize;

/// File-backed metadata for a demand-paged region.
#[derive(Clone, Copy, Debug)]
pub struct FileBacking {
    /// Opaque context pointer for the filesystem or file object.
    pub ctx: *mut u8,
    /// Read-at callback for the file backing.
    pub read_at: MmapReadAtFn,
    /// Starting offset in the file for this mapping (must be page-aligned).
    pub file_offset: u64,
    /// Total file size in bytes.
    pub file_size: u64,
}

/// Backing source for a demand-paged region.
#[derive(Clone, Copy, Debug)]
pub enum BackingKind {
    /// Allocate a zeroed page on first access.
    ZeroFill,
    /// File-backed mapping with demand paging.
    FileBacked(FileBacking),
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
        // TODO: Detect overlaps with existing regions instead of allowing
        // multiple registrations of the same address range.
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

/// Register a file-backed demand-paged region.
///
/// The mapping is registered but pages are loaded lazily on first access.
///
/// Notes:
/// - `file_offset` must be page-aligned.
/// - `file_size` is used to zero-fill beyond EOF.
pub fn register_file_region(
    start: VirtAddr,
    size: usize,
    flags: PageTableFlags,
    file_backing: FileBacking,
) -> Result<(), &'static str> {
    if (file_backing.file_offset % PAGE_SIZE as u64) != 0 {
        return Err("File offset must be page-aligned");
    }

    register_region(start, size, flags, BackingKind::FileBacked(file_backing))
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
            BackingKind::FileBacked(file) => {
                let virt = phys_to_virt(phys.as_u64());
                if virt.is_null() {
                    // TODO: Provide a temporary kernel mapping for high frames.
                    return Err("No identity mapping for frame");
                }

                // Always start with a zeroed page to handle short reads and EOF.
                ptr::write_bytes(virt, 0, PAGE_SIZE);

                let page_offset = page_base
                    .checked_sub(region.start)
                    .ok_or("Fault outside region")?;
                let file_offset = file
                    .file_offset
                    .checked_add(page_offset)
                    .ok_or("File offset overflow")?;

                if file_offset < file.file_size {
                    let remaining = (file.file_size - file_offset) as usize;
                    let to_read = core::cmp::min(PAGE_SIZE, remaining);
                    let read = (file.read_at)(file.ctx, file_offset, virt, to_read);
                    if read != to_read {
                        // TODO: Distinguish short reads from I/O errors once the
                        // filesystem read API returns error codes.
                    }
                }

                // TODO: Track dirty pages for MAP_SHARED-style writeback.
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
