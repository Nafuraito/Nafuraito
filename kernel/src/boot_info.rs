//! boot_info — Unified Boot Information Reader
//!
//! Reads the BootInfo struct written by the bootloader to physical address
//! 0x6000 (BOOT_INFO_ADDR) before the kernel entry point is called.
//!
//! Both the BIOS (bridge.asm) and UEFI (uefi/src/main.c) bootloaders fill
//! this struct so the kernel can determine:
//!   - Which boot path was used (BIOS or UEFI)
//!   - The physical address of the ACPI RSDP (UEFI provides this reliably)
//!   - How many entries are in the E820-format memory map
//!
//! The struct is validated via a 32-bit magic number; if the magic is absent
//! (e.g. during very early development with an old bootloader) the module
//! silently operates in "unknown" mode.

/// Physical address of the BootInfo struct written by the bootloader.
const BOOT_INFO_ADDR: usize = 0x6000;

/// Magic number that identifies a valid BootInfo struct — ASCII "NAFI".
const BOOT_INFO_MAGIC: u32 = 0x4E414649;

/// boot_type value indicating a BIOS / MBR boot — ASCII "BIOS".
pub const BOOT_TYPE_BIOS: u32 = 0x42494F53;

/// boot_type value indicating a UEFI boot — ASCII "UEFI".
pub const BOOT_TYPE_UEFI: u32 = 0x55454649;

// ============================================================================
// Global state
// ============================================================================

/// Boot type detected at startup (`BOOT_TYPE_BIOS`, `BOOT_TYPE_UEFI`, or 0).
static mut BOOT_TYPE: u32 = 0;

/// Physical address of the ACPI RSDP reported by the bootloader (0 = unknown).
static mut RSDP_ADDR: u64 = 0;

/// Number of E820 memory map entries passed via R9 (informational).
static mut MEM_ENTRY_COUNT: u32 = 0;

// ============================================================================
// Initialisation
// ============================================================================

/// Read the BootInfo struct from physical 0x6000 and populate the module
/// globals.
///
/// # Safety
/// Must be called exactly once, early in kernel startup before any code that
/// needs to know the boot type.  Physical address 0x6000 is in the first
/// megabyte and is always identity-mapped by the bootloader's page tables.
pub unsafe fn init() {
    // Read the magic field (offset +0).
    let magic = core::ptr::read_volatile(BOOT_INFO_ADDR as *const u32);
    if magic != BOOT_INFO_MAGIC {
        // Struct not present or written by an old bootloader — leave globals at 0.
        return;
    }

    // +0x04: boot_type
    BOOT_TYPE = core::ptr::read_volatile((BOOT_INFO_ADDR + 0x04) as *const u32);

    // +0x08: rsdp_addr (64-bit)
    RSDP_ADDR = core::ptr::read_volatile((BOOT_INFO_ADDR + 0x08) as *const u64);

    // +0x10: mem_map_entry_count
    MEM_ENTRY_COUNT = core::ptr::read_volatile((BOOT_INFO_ADDR + 0x10) as *const u32);
}

// ============================================================================
// Public accessors
// ============================================================================

/// Returns the raw boot_type field (`BOOT_TYPE_BIOS`, `BOOT_TYPE_UEFI`, or 0).
pub fn get_boot_type() -> u32 {
    unsafe { BOOT_TYPE }
}

/// Returns `true` when the system was booted via UEFI.
pub fn is_uefi_boot() -> bool {
    unsafe { BOOT_TYPE == BOOT_TYPE_UEFI }
}

/// Returns `true` when the system was booted via BIOS / MBR.
pub fn is_bios_boot() -> bool {
    unsafe { BOOT_TYPE == BOOT_TYPE_BIOS }
}

/// Returns the physical address of the ACPI RSDP, or 0 if not provided.
///
/// The UEFI bootloader locates the RSDP via `EFI_ACPI_20_TABLE_GUID` in
/// the UEFI configuration table.  The BIOS bootloader currently returns 0;
/// ACPI RSDP scanning for the BIOS path may be added in the future.
pub fn get_rsdp_addr() -> u64 {
    unsafe { RSDP_ADDR }
}

/// Returns the number of E820 entries in the memory map passed via R9.
pub fn get_mem_entry_count() -> u32 {
    unsafe { MEM_ENTRY_COUNT }
}

/// Returns a human-readable string identifying the boot path.
pub fn boot_type_str() -> &'static str {
    match get_boot_type() {
        BOOT_TYPE_UEFI => "UEFI",
        BOOT_TYPE_BIOS => "BIOS/MBR",
        _ => "Unknown",
    }
}
