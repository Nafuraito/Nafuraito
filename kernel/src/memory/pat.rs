//! Page Attribute Table (PAT) Support
//!
//! This module provides PAT (Page Attribute Table) support for fine-grained
//! control over memory caching on a per-page basis.
//!
//! PAT extends the page table format to allow specifying memory types for
//! individual pages, complementing the MTRRs (Memory Type Range Registers).
//!
//! ## Memory Types
//! - UC (Uncacheable): No caching - used for memory-mapped I/O
//! - WC (Write-Combining): Writes are combined and buffered - used for framebuffers
//! - WT (Write-Through): Writes go to cache and memory - consistent but slower writes
//! - WP (Write-Protected): Reads can be cached, writes go directly to memory
//! - WB (Write-Back): Default caching - best performance for normal memory
//!
//! ## MSR IA32_PAT
//! The PAT is configured via the IA32_PAT MSR (0x277), which contains 8 entries
//! (PA0-PA7), each defining a memory type. The page table bits PCD, PWT, and PAT
//! are used to select one of the 8 entries.

#![allow(unused)]

use core::arch::asm;

// Serial output for debugging
#[inline(always)]
unsafe fn serial_outb(val: u8) {
    asm!("out dx, al", in("dx") 0x3F8u16, in("al") val, options(nomem, nostack));
}

#[inline(always)]
unsafe fn serial_print(s: &str) {
    for &b in s.as_bytes() {
        serial_outb(b);
    }
}

#[inline(always)]
unsafe fn serial_print_hex(mut num: u64) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    serial_print("0x");
    if num == 0 {
        serial_outb(b'0');
        return;
    }
    let mut buf = [0u8; 16];
    let mut i = 0usize;
    while num > 0 && i < 16 {
        buf[i] = HEX[(num & 0xF) as usize];
        num >>= 4;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        serial_outb(buf[i]);
    }
}

/// IA32_PAT MSR address
pub const IA32_PAT_MSR: u32 = 0x277;

/// Memory type values for PAT entries
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Uncacheable - no caching (memory-mapped I/O)
    UC = 0x00,
    /// Write-Combining - writes are buffered (framebuffer)
    WC = 0x01,
    /// Reserved (do not use)
    Reserved2 = 0x02,
    /// Reserved (do not use)
    Reserved3 = 0x03,
    /// Write-Through - writes to cache and memory
    WT = 0x04,
    /// Write-Protected - reads cached, writes to memory
    WP = 0x05,
    /// Write-Back - default caching (normal memory)
    WB = 0x06,
    /// Uncacheable with Write-Combining allowed
    UCMinus = 0x07,
}

impl MemoryType {
    /// Convert from u8
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x00 => Some(MemoryType::UC),
            0x01 => Some(MemoryType::WC),
            0x04 => Some(MemoryType::WT),
            0x05 => Some(MemoryType::WP),
            0x06 => Some(MemoryType::WB),
            0x07 => Some(MemoryType::UCMinus),
            _ => None,
        }
    }

    /// Get name string
    pub fn name(&self) -> &'static str {
        match self {
            MemoryType::UC => "UC (Uncacheable)",
            MemoryType::WC => "WC (Write-Combining)",
            MemoryType::WT => "WT (Write-Through)",
            MemoryType::WP => "WP (Write-Protected)",
            MemoryType::WB => "WB (Write-Back)",
            MemoryType::UCMinus => "UC- (Uncacheable)",
            MemoryType::Reserved2 => "Reserved",
            MemoryType::Reserved3 => "Reserved",
        }
    }
}

/// PAT configuration structure
#[derive(Debug, Clone, Copy)]
pub struct PatConfig {
    /// PA0 - Entry 0 (PCD=0, PWT=0, PAT=0)
    pub pa0: MemoryType,
    /// PA1 - Entry 1 (PCD=0, PWT=1, PAT=0)
    pub pa1: MemoryType,
    /// PA2 - Entry 2 (PCD=1, PWT=0, PAT=0)
    pub pa2: MemoryType,
    /// PA3 - Entry 3 (PCD=1, PWT=1, PAT=0)
    pub pa3: MemoryType,
    /// PA4 - Entry 4 (PCD=0, PWT=0, PAT=1)
    pub pa4: MemoryType,
    /// PA5 - Entry 5 (PCD=0, PWT=1, PAT=1)
    pub pa5: MemoryType,
    /// PA6 - Entry 6 (PCD=1, PWT=0, PAT=1)
    pub pa6: MemoryType,
    /// PA7 - Entry 7 (PCD=1, PWT=1, PAT=1)
    pub pa7: MemoryType,
}

impl PatConfig {
    /// Create default PAT configuration (Intel default)
    /// PA0 = WB, PA1 = WT, PA2 = UC-, PA3 = UC,
    /// PA4 = WB, PA5 = WT, PA6 = UC-, PA7 = UC
    pub const fn default() -> Self {
        PatConfig {
            pa0: MemoryType::WB,    // Default caching
            pa1: MemoryType::WT,    // Write-through
            pa2: MemoryType::UCMinus, // Uncacheable
            pa3: MemoryType::UC,    // Uncacheable
            pa4: MemoryType::WB,    // Default caching
            pa5: MemoryType::WT,    // Write-through
            pa6: MemoryType::UCMinus, // Uncacheable
            pa7: MemoryType::UC,    // Uncacheable
        }
    }

    /// Create optimized PAT configuration for OS use
    /// PA0 = WB (normal memory)
    /// PA1 = WT (write-through)
    /// PA2 = UC (uncacheable - MMIO)
    /// PA3 = UC (uncacheable)
    /// PA4 = WB (normal memory)
    /// PA5 = WC (write-combining - framebuffer)
    /// PA6 = WP (write-protected)
    /// PA7 = UC (uncacheable)
    pub const fn optimized() -> Self {
        PatConfig {
            pa0: MemoryType::WB,    // Normal memory (default)
            pa1: MemoryType::WT,    // Write-through
            pa2: MemoryType::UC,    // Uncacheable (MMIO)
            pa3: MemoryType::UC,    // Uncacheable
            pa4: MemoryType::WB,    // Normal memory
            pa5: MemoryType::WC,    // Write-combining (framebuffer)
            pa6: MemoryType::WP,    // Write-protected
            pa7: MemoryType::UC,    // Uncacheable
        }
    }

    /// Convert to MSR value
    pub fn to_msr_value(&self) -> u64 {
        (self.pa0 as u64)
            | ((self.pa1 as u64) << 8)
            | ((self.pa2 as u64) << 16)
            | ((self.pa3 as u64) << 24)
            | ((self.pa4 as u64) << 32)
            | ((self.pa5 as u64) << 40)
            | ((self.pa6 as u64) << 48)
            | ((self.pa7 as u64) << 56)
    }

    /// Create from MSR value
    pub fn from_msr_value(val: u64) -> Self {
        PatConfig {
            pa0: MemoryType::from_u8((val & 0xFF) as u8).unwrap_or(MemoryType::WB),
            pa1: MemoryType::from_u8(((val >> 8) & 0xFF) as u8).unwrap_or(MemoryType::WT),
            pa2: MemoryType::from_u8(((val >> 16) & 0xFF) as u8).unwrap_or(MemoryType::UCMinus),
            pa3: MemoryType::from_u8(((val >> 24) & 0xFF) as u8).unwrap_or(MemoryType::UC),
            pa4: MemoryType::from_u8(((val >> 32) & 0xFF) as u8).unwrap_or(MemoryType::WB),
            pa5: MemoryType::from_u8(((val >> 40) & 0xFF) as u8).unwrap_or(MemoryType::WT),
            pa6: MemoryType::from_u8(((val >> 48) & 0xFF) as u8).unwrap_or(MemoryType::UCMinus),
            pa7: MemoryType::from_u8(((val >> 56) & 0xFF) as u8).unwrap_or(MemoryType::UC),
        }
    }
}

/// PAT manager
pub struct PatManager {
    /// PAT support available
    supported: bool,
    /// PAT enabled
    enabled: bool,
    /// Current PAT configuration
    config: PatConfig,
}

/// Global PAT manager instance
static mut PAT_MANAGER: Option<PatManager> = None;

impl PatManager {
    /// Create new PAT manager
    pub const fn new() -> Self {
        PatManager {
            supported: false,
            enabled: false,
            config: PatConfig::default(),
        }
    }

    /// Check if PAT is supported
    pub fn is_supported(&self) -> bool {
        self.supported
    }

    /// Check if PAT is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get current configuration
    pub fn config(&self) -> PatConfig {
        self.config
    }
}

/// CPUID result structure
#[derive(Debug, Clone, Copy)]
struct CpuidResult {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
}

/// Execute CPUID instruction
fn cpuid(leaf: u32, subleaf: u32) -> CpuidResult {
    let mut eax: u32;
    let mut ebx: u32;
    let mut ecx: u32;
    let mut edx: u32;

    unsafe {
        asm!(
            "push rbx",       // Save rbx (LLVM uses it)
            "cpuid",
            "mov {0:e}, ebx", // Move ebx to output without using it as operand
            "pop rbx",        // Restore rbx
            out(reg) ebx,
            inout("eax") leaf => eax,
            inout("ecx") subleaf => ecx,
            inout("edx") 0u32 => edx,
            options(nostack, preserves_flags)
        );
    }

    CpuidResult { eax, ebx, ecx, edx }
}

/// Check if PAT is supported via CPUID
pub fn check_pat_support() -> bool {
    // CPUID.01H:EDX.PAT[bit 16] = 1
    let result = cpuid(0x01, 0);
    (result.edx & (1 << 16)) != 0
}

/// Read MSR
#[inline(always)]
unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack, preserves_flags)
    );
    ((high as u64) << 32) | (low as u64)
}

/// Write MSR
#[inline(always)]
unsafe fn wrmsr(msr: u32, value: u64) {
    let low = (value & 0xFFFF_FFFF) as u32;
    let high = (value >> 32) as u32;
    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, nostack, preserves_flags)
    );
}

/// Read current PAT configuration from MSR
pub fn read_pat() -> PatConfig {
    unsafe {
        let val = rdmsr(IA32_PAT_MSR);
        PatConfig::from_msr_value(val)
    }
}

/// Write PAT configuration to MSR
pub unsafe fn write_pat(config: &PatConfig) {
    let val = config.to_msr_value();
    wrmsr(IA32_PAT_MSR, val);
}

/// Initialize PAT with optimized configuration
pub fn pat_init() -> Result<(), &'static str> {
    unsafe {
        serial_print("\n[PAT] Initializing Page Attribute Table...\n");

        // Check if PAT is supported
        if !check_pat_support() {
            serial_print("[PAT] ERROR: PAT not supported by CPU\n");
            return Err("PAT not supported");
        }

        serial_print("[PAT] PAT support detected\n");

        // Read current PAT configuration
        let current_config = read_pat();
        serial_print("[PAT] Current PAT configuration:\n");
        serial_print("  PA0: ");
        serial_print(current_config.pa0.name());
        serial_print("\n  PA1: ");
        serial_print(current_config.pa1.name());
        serial_print("\n  PA2: ");
        serial_print(current_config.pa2.name());
        serial_print("\n  PA3: ");
        serial_print(current_config.pa3.name());
        serial_print("\n  PA4: ");
        serial_print(current_config.pa4.name());
        serial_print("\n  PA5: ");
        serial_print(current_config.pa5.name());
        serial_print("\n  PA6: ");
        serial_print(current_config.pa6.name());
        serial_print("\n  PA7: ");
        serial_print(current_config.pa7.name());
        serial_print("\n");

        // Set optimized PAT configuration
        let optimized_config = PatConfig::optimized();
        serial_print("[PAT] Setting optimized PAT configuration...\n");
        write_pat(&optimized_config);

        // Verify configuration
        let new_config = read_pat();
        serial_print("[PAT] New PAT configuration:\n");
        serial_print("  PA0: ");
        serial_print(new_config.pa0.name());
        serial_print(" (Normal memory - default)\n  PA1: ");
        serial_print(new_config.pa1.name());
        serial_print("\n  PA2: ");
        serial_print(new_config.pa2.name());
        serial_print(" (MMIO)\n  PA3: ");
        serial_print(new_config.pa3.name());
        serial_print("\n  PA4: ");
        serial_print(new_config.pa4.name());
        serial_print("\n  PA5: ");
        serial_print(new_config.pa5.name());
        serial_print(" (Framebuffer)\n  PA6: ");
        serial_print(new_config.pa6.name());
        serial_print("\n  PA7: ");
        serial_print(new_config.pa7.name());
        serial_print("\n");

        // Initialize global manager
        PAT_MANAGER = Some(PatManager {
            supported: true,
            enabled: true,
            config: new_config,
        });

        serial_print("[PAT] Initialization complete\n");
        Ok(())
    }
}

/// Get PAT manager instance
pub fn get_pat_manager() -> Option<&'static PatManager> {
    unsafe { PAT_MANAGER.as_ref() }
}

/// Check if PAT is initialized
pub fn is_pat_initialized() -> bool {
    unsafe { PAT_MANAGER.is_some() }
}

// =============================================================================
// C-compatible wrapper functions
// =============================================================================

/// C-compatible: Check PAT support
#[no_mangle]
pub extern "C" fn pat_check_support() -> u8 {
    if check_pat_support() { 1 } else { 0 }
}

/// C-compatible: Initialize PAT
#[no_mangle]
pub extern "C" fn pat_init_c() -> i32 {
    match pat_init() {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// C-compatible: Check if PAT is initialized
#[no_mangle]
pub extern "C" fn pat_is_initialized() -> u8 {
    if is_pat_initialized() { 1 } else { 0 }
}

/// C-compatible: Get PAT entry value
#[no_mangle]
pub extern "C" fn pat_get_entry(index: u8) -> u8 {
    if index > 7 {
        return 0xFF; // Invalid
    }

    unsafe {
        if let Some(manager) = PAT_MANAGER.as_ref() {
            let config = manager.config;
            match index {
                0 => config.pa0 as u8,
                1 => config.pa1 as u8,
                2 => config.pa2 as u8,
                3 => config.pa3 as u8,
                4 => config.pa4 as u8,
                5 => config.pa5 as u8,
                6 => config.pa6 as u8,
                7 => config.pa7 as u8,
                _ => 0xFF,
            }
        } else {
            0xFF
        }
    }
}

/// C-compatible: Read PAT MSR value
#[no_mangle]
pub extern "C" fn pat_read_msr() -> u64 {
    unsafe { rdmsr(IA32_PAT_MSR) }
}

// =============================================================================
// Page Table Helper Functions
// =============================================================================

/// Convert memory type to page table flags (PCD, PWT, PAT bits)
/// Based on the optimized PAT configuration:
/// - PA0 (000): WB - Normal memory
/// - PA1 (001): WT - Write-through
/// - PA2 (010): UC - Uncacheable (MMIO)
/// - PA5 (101): WC - Write-combining (Framebuffer)
pub fn memory_type_to_flags(mem_type: MemoryType) -> u64 {
    match mem_type {
        MemoryType::WB => 0, // PCD=0, PWT=0, PAT=0 -> PA0
        MemoryType::WT => (1 << 3), // PCD=0, PWT=1, PAT=0 -> PA1
        MemoryType::UC => (1 << 4), // PCD=1, PWT=0, PAT=0 -> PA2
        MemoryType::WC => (1 << 7) | (1 << 3), // PCD=0, PWT=1, PAT=1 -> PA5
        MemoryType::WP => (1 << 7) | (1 << 4), // PCD=1, PWT=0, PAT=1 -> PA6
        _ => 0, // Default to WB
    }
}

/// Get flags for normal memory (Write-Back)
#[inline(always)]
pub const fn flags_normal_memory() -> u64 {
    0 // PA0: WB
}

/// Get flags for write-through memory
#[inline(always)]
pub const fn flags_write_through() -> u64 {
    1 << 3 // PA1: WT
}

/// Get flags for uncacheable memory (MMIO)
#[inline(always)]
pub const fn flags_uncacheable() -> u64 {
    1 << 4 // PA2: UC
}

/// Get flags for write-combining memory (Framebuffer)
#[inline(always)]
pub const fn flags_write_combining() -> u64 {
    (1 << 7) | (1 << 3) // PA5: WC
}

/// Get flags for write-protected memory
#[inline(always)]
pub const fn flags_write_protected() -> u64 {
    (1 << 7) | (1 << 4) // PA6: WP
}
