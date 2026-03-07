//! Paging Module with LA57 (5-Level Paging) Support
//! =============================================================================

#![allow(unused)]

use core::arch::asm;
use core::ptr;

// Simple serial output for debugging
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

/// Page size constants
pub const PAGE_SIZE: usize = 4096;  // 4 KiB
pub const PAGE_SIZE_2MB: usize = 2 * 1024 * 1024;  // 2 MiB
pub const PAGE_SIZE_1GB: usize = 1024 * 1024 * 1024;  // 1 GiB

/// Number of entries in each page table level
pub const PAGE_TABLE_ENTRIES: usize = 512;

/// Virtual address space sizes
pub const LA48_VIRT_ADDR_BITS: usize = 48;
pub const LA57_VIRT_ADDR_BITS: usize = 57;
pub const LA48_VIRT_ADDR_SIZE: u64 = 1u64 << LA48_VIRT_ADDR_BITS;  // 256 TiB
pub const LA57_VIRT_ADDR_SIZE: u64 = 1u64 << LA57_VIRT_ADDR_BITS;  // 128 PiB

/// Page table entry flags
/// 
/// Each page table entry is 64 bits:
/// - Bits 0-11: Flags (present, writable, user-accessible, etc.)
/// - Bits 12-51: Physical address of page or next-level table
/// - Bits 52-62: More flags and reserved bits
/// - Bit 63: NX (No-Execute) bit
///
/// These flags control how the page can be accessed
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageTableFlags(u64);

impl PageTableFlags {
    pub const PRESENT: u64 = 1 << 0;          // P - Present: Must be 1 for page to be accessible
    pub const WRITABLE: u64 = 1 << 1;         // R/W - If 0, page is read-only
    pub const USER: u64 = 1 << 2;             // U/S - If 0, only kernel can access
    pub const WRITE_THROUGH: u64 = 1 << 3;    // PWT - Write-through caching (PAT bit 0)
    pub const CACHE_DISABLE: u64 = 1 << 4;    // PCD - Disable caching for this page (PAT bit 1)
    pub const ACCESSED: u64 = 1 << 5;         // A - CPU sets this when page is accessed
    pub const DIRTY: u64 = 1 << 6;            // D - CPU sets this when page is written to
    pub const HUGE_PAGE: u64 = 1 << 7;        // PS - 2MB/1GB page instead of 4KB
    pub const GLOBAL: u64 = 1 << 8;           // G - Don't flush from TLB on CR3 change
    pub const PAT: u64 = 1 << 7;              // PAT - Memory type (WB, WT, UC, WC, etc.)
    pub const NO_EXECUTE: u64 = 1 << 63;      // NX - CPU will fault if we try to execute code here

    /// Create new flags
    pub const fn new() -> Self {
        PageTableFlags(0)
    }

    /// Create flags from raw value
    pub const fn from_raw(flags: u64) -> Self {
        PageTableFlags(flags)
    }

    /// Get raw flags value
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Check if flag is set
    pub const fn contains(&self, flag: u64) -> bool {
        (self.0 & flag) != 0
    }

    /// Set flag
    pub fn set(&mut self, flag: u64) {
        self.0 |= flag;
    }

    /// Clear flag
    pub fn clear(&mut self, flag: u64) {
        self.0 &= !flag;
    }

    /// Standard kernel page flags
    pub const fn kernel() -> Self {
        PageTableFlags(Self::PRESENT | Self::WRITABLE | Self::GLOBAL)
    }

    /// Standard user page flags
    pub const fn user() -> Self {
        PageTableFlags(Self::PRESENT | Self::WRITABLE | Self::USER)
    }

    /// Read-only kernel page
    pub const fn kernel_readonly() -> Self {
        PageTableFlags(Self::PRESENT | Self::GLOBAL)
    }

    /// Read-only user page
    pub const fn user_readonly() -> Self {
        PageTableFlags(Self::PRESENT | Self::USER)
    }

    /// Kernel page with memory type (PAT-aware)
    /// Uses PAT bits (PCD, PWT, PAT) to set memory type
    pub const fn kernel_with_memory_type(pat_flags: u64) -> Self {
        PageTableFlags(Self::PRESENT | Self::WRITABLE | Self::GLOBAL | pat_flags)
    }

    /// User page with memory type (PAT-aware)
    pub const fn user_with_memory_type(pat_flags: u64) -> Self {
        PageTableFlags(Self::PRESENT | Self::WRITABLE | Self::USER | pat_flags)
    }

    /// Uncacheable kernel page (for MMIO)
    pub const fn kernel_uncacheable() -> Self {
        // Using PA2: UC (PCD=1, PWT=0, PAT=0)
        PageTableFlags(Self::PRESENT | Self::WRITABLE | Self::GLOBAL | Self::CACHE_DISABLE)
    }

    /// Write-combining kernel page (for framebuffer)
    pub const fn kernel_write_combining() -> Self {
        // Using PA5: WC (PCD=0, PWT=1, PAT=1)
        PageTableFlags(Self::PRESENT | Self::WRITABLE | Self::GLOBAL | Self::WRITE_THROUGH | Self::PAT)
    }
}

/// Page table entry (shared for all levels)
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Create a new empty entry
    pub const fn new() -> Self {
        PageTableEntry(0)
    }

    /// Check if entry is present
    pub const fn is_present(&self) -> bool {
        (self.0 & PageTableFlags::PRESENT) != 0
    }

    /// Check if entry is a huge page (2MB or 1GB)
    pub const fn is_huge(&self) -> bool {
        (self.0 & PageTableFlags::HUGE_PAGE) != 0
    }

    /// Get physical address from entry
    pub const fn address(&self) -> u64 {
        self.0 & 0x000F_FFFF_FFFF_F000
    }

    /// Get flags from entry
    pub const fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_raw(self.0 & 0xFFF)
    }

    /// Set entry with address and flags
    pub fn set(&mut self, addr: u64, flags: PageTableFlags) {
        // Ensure address is page-aligned
        debug_assert!(addr & 0xFFF == 0, "Address must be page-aligned");
        self.0 = (addr & 0x000F_FFFF_FFFF_F000) | flags.raw();
    }

    /// Clear entry
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Get raw entry value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Page table structure (512 entries)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    /// Create a new zeroed page table
    pub const fn new() -> Self {
        PageTable {
            entries: [PageTableEntry::new(); PAGE_TABLE_ENTRIES],
        }
    }

    /// Get entry at index
    pub fn get(&self, index: usize) -> Option<&PageTableEntry> {
        self.entries.get(index)
    }

    /// Get mutable entry at index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PageTableEntry> {
        self.entries.get_mut(index)
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.clear();
        }
    }

    /// Get physical address of this table
    pub fn phys_addr(&self) -> u64 {
        self as *const _ as u64
    }
}

/// Paging mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingMode {
    /// 4-level paging (LA48) - 48-bit virtual addresses
    FourLevel,
    /// 5-level paging (LA57) - 57-bit virtual addresses
    FiveLevel,
}

/// Paging manager state
pub struct PagingManager {
    /// Current paging mode
    mode: PagingMode,
    /// Root page table physical address (PML4 or PML5)
    root_table_phys: u64,
    /// LA57 support available
    la57_supported: bool,
    /// NX (No-Execute) support available
    nx_supported: bool,
}

/// Global paging manager instance
static mut PAGING_MANAGER: Option<PagingManager> = None;

impl PagingManager {
    /// Create a new paging manager
    pub const fn new() -> Self {
        PagingManager {
            mode: PagingMode::FourLevel,
            root_table_phys: 0,
            la57_supported: false,
            nx_supported: false,
        }
    }

    /// Get current paging mode
    pub fn mode(&self) -> PagingMode {
        self.mode
    }

    /// Check if LA57 is supported
    pub fn is_la57_supported(&self) -> bool {
        self.la57_supported
    }

    /// Check if NX is supported
    pub fn is_nx_supported(&self) -> bool {
        self.nx_supported
    }

    /// Get root table physical address
    pub fn root_table_phys(&self) -> u64 {
        self.root_table_phys
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
            "mov {ebx_tmp:e}, ebx",  // Save rbx (LLVM uses it)
            "cpuid",
            "xchg {ebx_tmp:e}, ebx",  // Restore rbx
            inout("eax") leaf => eax,
            ebx_tmp = out(reg) ebx,
            inout("ecx") subleaf => ecx,
            inout("edx") 0u32 => edx,
            options(nostack, preserves_flags)
        );
    }

    CpuidResult { eax, ebx, ecx, edx }
}

/// Check if LA57 (5-level paging) is supported
pub fn check_la57_support() -> bool {
    // CPUID.07H:ECX.LA57[bit 16] indicates LA57 support
    let result = cpuid(0x07, 0);
    (result.ecx & (1 << 16)) != 0
}

/// Check if NX (No-Execute) is supported
pub fn check_nx_support() -> bool {
    // CPUID.80000001H:EDX.NX[bit 20] indicates NX support
    let result = cpuid(0x8000_0001, 0);
    (result.edx & (1 << 20)) != 0
}

/// Read CR3 register (current page table root)
pub fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nostack, preserves_flags));
    }
    cr3
}

/// Write CR3 register (set page table root)
pub unsafe fn write_cr3(value: u64) {
    asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
}

/// Read CR4 register
pub fn read_cr4() -> u64 {
    let cr4: u64;
    unsafe {
        asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
    }
    cr4
}

/// Write CR4 register
pub unsafe fn write_cr4(value: u64) {
    asm!("mov cr4, {}", in(reg) value, options(nostack, preserves_flags));
}

/// Enable LA57 (5-level paging)
/// Must be called before enabling paging!
pub unsafe fn enable_la57() -> Result<(), &'static str> {
    if !check_la57_support() {
        return Err("LA57 not supported by CPU");
    }

    // CR4.LA57[bit 12] enables 5-level paging
    let mut cr4 = read_cr4();
    cr4 |= 1 << 12;  // Set LA57 bit
    write_cr4(cr4);

    Ok(())
}

/// Disable LA57 (return to 4-level paging)
pub unsafe fn disable_la57() {
    let mut cr4 = read_cr4();
    cr4 &= !(1 << 12);  // Clear LA57 bit
    write_cr4(cr4);
}

/// Check if LA57 is currently enabled
pub fn is_la57_enabled() -> bool {
    let cr4 = read_cr4();
    (cr4 & (1 << 12)) != 0
}

/// Invalidate TLB entry for a specific virtual address
pub unsafe fn invlpg(virt_addr: u64) {
    asm!("invlpg [{}]", in(reg) virt_addr, options(nostack, preserves_flags));
}

/// Flush entire TLB by reloading CR3
pub unsafe fn flush_tlb() {
    let cr3 = read_cr3();
    write_cr3(cr3);
}

/// Virtual address structure for easy decomposition
#[derive(Debug, Clone, Copy)]
pub struct VirtAddr {
    addr: u64,
}

impl VirtAddr {
    /// Create from raw address
    pub const fn new(addr: u64) -> Self {
        VirtAddr { addr }
    }

    /// Get raw address
    pub const fn as_u64(&self) -> u64 {
        self.addr
    }

    /// Get PML5 index (bits 56-48) for LA57
    pub const fn pml5_index(&self) -> usize {
        ((self.addr >> 48) & 0x1FF) as usize
    }

    /// Get PML4 index (bits 47-39)
    pub const fn pml4_index(&self) -> usize {
        ((self.addr >> 39) & 0x1FF) as usize
    }

    /// Get PDPT index (bits 38-30)
    pub const fn pdpt_index(&self) -> usize {
        ((self.addr >> 30) & 0x1FF) as usize
    }

    /// Get PD index (bits 29-21)
    pub const fn pd_index(&self) -> usize {
        ((self.addr >> 21) & 0x1FF) as usize
    }

    /// Get PT index (bits 20-12)
    pub const fn pt_index(&self) -> usize {
        ((self.addr >> 12) & 0x1FF) as usize
    }

    /// Get page offset (bits 11-0)
    pub const fn page_offset(&self) -> usize {
        (self.addr & 0xFFF) as usize
    }

    /// Check if address is canonical for LA48
    pub const fn is_canonical_la48(&self) -> bool {
        let sign_bit = (self.addr >> 47) & 1;
        let upper_bits = self.addr >> 48;
        // Upper bits must be all 0s or all 1s
        upper_bits == 0 || upper_bits == 0xFFFF
    }

    /// Check if address is canonical for LA57
    pub const fn is_canonical_la57(&self) -> bool {
        let sign_bit = (self.addr >> 56) & 1;
        let upper_bits = self.addr >> 57;
        // Upper bits must be all 0s or all 1s
        upper_bits == 0 || upper_bits == 0x7F
    }
}

/// Physical address type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysAddr(pub u64);

impl PhysAddr {
    /// Create from raw address
    pub const fn new(addr: u64) -> Self {
        PhysAddr(addr)
    }

    /// Get raw address
    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// Check if address is page-aligned
    pub const fn is_aligned(&self) -> bool {
        (self.0 & 0xFFF) == 0
    }

    /// Align down to page boundary
    pub const fn align_down(&self) -> Self {
        PhysAddr(self.0 & !0xFFF)
    }

    /// Align up to page boundary
    pub const fn align_up(&self) -> Self {
        PhysAddr((self.0 + 0xFFF) & !0xFFF)
    }
}

/// Initialize paging system
pub fn paging_init(root_table_phys: u64) -> Result<(), &'static str> {
    let la57_supported = check_la57_support();
    let nx_supported = check_nx_support();
    let mode = if is_la57_enabled() {
        PagingMode::FiveLevel
    } else {
        PagingMode::FourLevel
    };

    // Mask out CR3 control bits (PCID/PWT/PCD) to leave only the table base.
    let root_aligned = root_table_phys & !0xFFFu64;

    let manager = PagingManager {
        mode,
        root_table_phys: root_aligned,
        la57_supported,
        nx_supported,
    };

    unsafe {
        PAGING_MANAGER = Some(manager);
    }

    Ok(())
}

/// Get the global paging manager
pub fn get_paging_manager() -> Option<&'static PagingManager> {
    unsafe { PAGING_MANAGER.as_ref() }
}

// =============================================================================
// Page Table Manipulation Functions
// =============================================================================

/// Convert a physical address to a virtual address for accessing page tables
/// In identity-mapped kernel, physical == virtual for low memory (first 4GB)
/// The bootloader identity-maps the first 4GB for kernel use
#[inline]
unsafe fn phys_to_virt(phys_addr: u64) -> *mut u8 {
    // Ensure we're in the identity-mapped region (first 4GB)
    if phys_addr >= 0x1_0000_0000 {
        // Address is beyond identity mapping
        // This would be a bug - all page tables should be in low memory
        0 as *mut u8  // This will cause immediate crash, which is better than silent corruption
    } else {
        phys_addr as *mut u8
    }
}

/// Allocate a physical frame for a page table
unsafe fn alloc_page_table() -> Result<u64, &'static str> {
    extern "C" {
        fn pmm_alloc_frame() -> u64;
        fn pmm_alloc_dma32_frame() -> u64;
    }

    // Prefer DMA32 (<4 GiB) frames so the identity map always covers the table.
    let mut addr = pmm_alloc_dma32_frame();
    if addr == 0 {
        addr = pmm_alloc_frame();
    }
    if addr == 0 {
        return Err("Failed to allocate page table frame");
    }

    let table_ptr = phys_to_virt(addr) as *mut u64;
    if table_ptr.is_null() {
        return Err("Page table frame not identity-mapped (<4 GiB required)");
    }

    ptr::write_bytes(table_ptr, 0, PAGE_TABLE_ENTRIES);
    Ok(addr)
}

/// Free a physical frame used by a page table
unsafe fn free_page_table(addr: u64) -> Result<(), &'static str> {
    use super::pmm::free_frame;
    
    match free_frame(addr) {
        Ok(_) => Ok(()),
        Err(_) => Err("Failed to free page table frame"),
    }
}

/// Get or create a page table entry at the next level
/// Returns the physical address of the next level page table
unsafe fn get_or_create_next_level(
    entry: &mut PageTableEntry,
    flags: PageTableFlags,
) -> Result<u64, &'static str> {
    if entry.is_present() {
        // Entry exists, return its address
        Ok(entry.address())
    } else {
        // Allocate new page table
        let new_table_addr = alloc_page_table()?;
        
        // Set up entry to point to new table
        // Use present + writable + user flags for intermediate entries
        let mut table_flags = PageTableFlags::new();
        table_flags.set(PageTableFlags::PRESENT);
        table_flags.set(PageTableFlags::WRITABLE);
        if flags.contains(PageTableFlags::USER) {
            table_flags.set(PageTableFlags::USER);
        }
        
        entry.set(new_table_addr, table_flags);
        Ok(new_table_addr)
    }
}

/// Check if a page table is empty (all entries are zero)
unsafe fn is_page_table_empty(table_phys: u64) -> bool {
    let table = phys_to_virt(table_phys) as *const PageTable;
    for i in 0..PAGE_TABLE_ENTRIES {
        if (*table).entries[i].raw() != 0 {
            return false;
        }
    }
    true
}

/// Map a virtual address to a physical address with given flags
/// Handles both 4-level and 5-level paging
#[inline(never)]
pub unsafe fn map_page(
    virt_addr: VirtAddr,
    phys_addr: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    let manager = get_paging_manager().ok_or("Paging not initialized")?;
    
    // Validate addresses
    if !phys_addr.is_aligned() {
        return Err("Physical address not page-aligned");
    }
    
    // Check canonical address
    match manager.mode() {
        PagingMode::FourLevel => {
            if !virt_addr.is_canonical_la48() {
                return Err("Virtual address not canonical for LA48");
            }
        }
        PagingMode::FiveLevel => {
            if !virt_addr.is_canonical_la57() {
                return Err("Virtual address not canonical for LA57");
            }
        }
    }
    
    let root_table_phys = manager.root_table_phys();
    let mut current_table_phys = root_table_phys;
    
    // Walk through page table levels based on paging mode
    match manager.mode() {
        PagingMode::FiveLevel => {
            // PML5 -> PML4 -> PDPT -> PD -> PT
            let pml5_index = virt_addr.pml5_index();
            let pml4_index = virt_addr.pml4_index();
            let pdpt_index = virt_addr.pdpt_index();
            let pd_index = virt_addr.pd_index();
            let pt_index = virt_addr.pt_index();
            
            // Get PML5 entry
            let pml5 = phys_to_virt(current_table_phys) as *mut PageTable;
            let pml5_entry = &mut (*pml5).entries[pml5_index];
            current_table_phys = get_or_create_next_level(pml5_entry, flags)?;
            
            // Get PML4 entry
            let pml4 = phys_to_virt(current_table_phys) as *mut PageTable;
            let pml4_entry = &mut (*pml4).entries[pml4_index];
            current_table_phys = get_or_create_next_level(pml4_entry, flags)?;
            
            // Get PDPT entry
            let pdpt = phys_to_virt(current_table_phys) as *mut PageTable;
            let pdpt_entry = &mut (*pdpt).entries[pdpt_index];
            current_table_phys = get_or_create_next_level(pdpt_entry, flags)?;
            
            // Get PD entry
            let pd = phys_to_virt(current_table_phys) as *mut PageTable;
            let pd_entry = &mut (*pd).entries[pd_index];
            current_table_phys = get_or_create_next_level(pd_entry, flags)?;
            
            // Set PT entry (final level)
            let pt = phys_to_virt(current_table_phys) as *mut PageTable;
            let pt_entry = &mut (*pt).entries[pt_index];
            pt_entry.set(phys_addr.as_u64(), flags);
        }
        PagingMode::FourLevel => {
            // PML4 -> PDPT -> PD -> PT
            let pml4_index = virt_addr.pml4_index();
            let pdpt_index = virt_addr.pdpt_index();
            let pd_index = virt_addr.pd_index();
            let pt_index = virt_addr.pt_index();
            
            // Get PML4 entry
            let pml4 = phys_to_virt(current_table_phys) as *mut PageTable;
            let pml4_entry = &mut (*pml4).entries[pml4_index];
            current_table_phys = get_or_create_next_level(pml4_entry, flags)?;
            
            // Get PDPT entry
            let pdpt = phys_to_virt(current_table_phys) as *mut PageTable;
            let pdpt_entry = &mut (*pdpt).entries[pdpt_index];
            current_table_phys = get_or_create_next_level(pdpt_entry, flags)?;
            
            // Get PD entry
            let pd = phys_to_virt(current_table_phys) as *mut PageTable;
            let pd_entry = &mut (*pd).entries[pd_index];
            current_table_phys = get_or_create_next_level(pd_entry, flags)?;
            
            // Set PT entry (final level)
            let pt = phys_to_virt(current_table_phys) as *mut PageTable;
            let pt_entry = &mut (*pt).entries[pt_index];
            pt_entry.set(phys_addr.as_u64(), flags);
        }
    }
    
    // Flush TLB for this address
    invlpg(virt_addr.as_u64());
    
    Ok(())
}

/// Unmap a virtual address
/// Optionally frees empty page tables (free_empty parameter)
pub unsafe fn unmap_page(virt_addr: VirtAddr) -> Result<(), &'static str> {
    unmap_page_internal(virt_addr, true)
}

/// Internal unmap implementation with option to free empty tables
unsafe fn unmap_page_internal(virt_addr: VirtAddr, free_empty: bool) -> Result<(), &'static str> {
    let manager = get_paging_manager().ok_or("Paging not initialized")?;
    
    // Check canonical address
    match manager.mode() {
        PagingMode::FourLevel => {
            if !virt_addr.is_canonical_la48() {
                return Err("Virtual address not canonical for LA48");
            }
        }
        PagingMode::FiveLevel => {
            if !virt_addr.is_canonical_la57() {
                return Err("Virtual address not canonical for LA57");
            }
        }
    }
    
    let root_table_phys = manager.root_table_phys();
    
    match manager.mode() {
        PagingMode::FiveLevel => {
            let pml5_index = virt_addr.pml5_index();
            let pml4_index = virt_addr.pml4_index();
            let pdpt_index = virt_addr.pdpt_index();
            let pd_index = virt_addr.pd_index();
            let pt_index = virt_addr.pt_index();
            
            // Walk to PML5
            let pml5 = phys_to_virt(root_table_phys) as *mut PageTable;
            let pml5_entry = &mut (*pml5).entries[pml5_index];
            if !pml5_entry.is_present() {
                return Err("Page not mapped (PML5)");
            }
            let pml4_phys = pml5_entry.address();
            
            // Walk to PML4
            let pml4 = phys_to_virt(pml4_phys) as *mut PageTable;
            let pml4_entry = &mut (*pml4).entries[pml4_index];
            if !pml4_entry.is_present() {
                return Err("Page not mapped (PML4)");
            }
            let pdpt_phys = pml4_entry.address();
            
            // Walk to PDPT
            let pdpt = phys_to_virt(pdpt_phys) as *mut PageTable;
            let pdpt_entry = &mut (*pdpt).entries[pdpt_index];
            if !pdpt_entry.is_present() {
                return Err("Page not mapped (PDPT)");
            }
            let pd_phys = pdpt_entry.address();
            
            // Walk to PD
            let pd = phys_to_virt(pd_phys) as *mut PageTable;
            let pd_entry = &mut (*pd).entries[pd_index];
            if !pd_entry.is_present() {
                return Err("Page not mapped (PD)");
            }
            let pt_phys = pd_entry.address();
            
            // Clear PT entry
            let pt = phys_to_virt(pt_phys) as *mut PageTable;
            let pt_entry = &mut (*pt).entries[pt_index];
            if !pt_entry.is_present() {
                return Err("Page not mapped (PT)");
            }
            pt_entry.clear();
            
            // Optionally free empty page tables (bottom-up)
            if free_empty {
                if is_page_table_empty(pt_phys) {
                    pd_entry.clear();
                    free_page_table(pt_phys)?;
                    
                    if is_page_table_empty(pd_phys) {
                        pdpt_entry.clear();
                        free_page_table(pd_phys)?;
                        
                        if is_page_table_empty(pdpt_phys) {
                            pml4_entry.clear();
                            free_page_table(pdpt_phys)?;
                            
                            if is_page_table_empty(pml4_phys) {
                                pml5_entry.clear();
                                free_page_table(pml4_phys)?;
                            }
                        }
                    }
                }
            }
        }
        PagingMode::FourLevel => {
            let pml4_index = virt_addr.pml4_index();
            let pdpt_index = virt_addr.pdpt_index();
            let pd_index = virt_addr.pd_index();
            let pt_index = virt_addr.pt_index();
            
            // Walk to PML4
            let pml4 = phys_to_virt(root_table_phys) as *mut PageTable;
            let pml4_entry = &mut (*pml4).entries[pml4_index];
            if !pml4_entry.is_present() {
                return Err("Page not mapped (PML4)");
            }
            let pdpt_phys = pml4_entry.address();
            
            // Walk to PDPT
            let pdpt = phys_to_virt(pdpt_phys) as *mut PageTable;
            let pdpt_entry = &mut (*pdpt).entries[pdpt_index];
            if !pdpt_entry.is_present() {
                return Err("Page not mapped (PDPT)");
            }
            let pd_phys = pdpt_entry.address();
            
            // Walk to PD
            let pd = phys_to_virt(pd_phys) as *mut PageTable;
            let pd_entry = &mut (*pd).entries[pd_index];
            if !pd_entry.is_present() {
                return Err("Page not mapped (PD)");
            }
            let pt_phys = pd_entry.address();
            
            // Clear PT entry
            let pt = phys_to_virt(pt_phys) as *mut PageTable;
            let pt_entry = &mut (*pt).entries[pt_index];
            if !pt_entry.is_present() {
                return Err("Page not mapped (PT)");
            }
            pt_entry.clear();
            
            // Optionally free empty page tables (bottom-up)
            if free_empty {
                if is_page_table_empty(pt_phys) {
                    pd_entry.clear();
                    free_page_table(pt_phys)?;
                    
                    if is_page_table_empty(pd_phys) {
                        pdpt_entry.clear();
                        free_page_table(pd_phys)?;
                        
                        if is_page_table_empty(pdpt_phys) {
                            pml4_entry.clear();
                            free_page_table(pdpt_phys)?;
                        }
                    }
                }
            }
        }
    }
    
    // Flush TLB for this address
    invlpg(virt_addr.as_u64());
    
    Ok(())
}

/// Get physical address mapped to a virtual address
/// Handles both 4-level and 5-level paging, including huge pages
pub fn translate_address(virt_addr: VirtAddr) -> Result<PhysAddr, &'static str> {
    let manager = get_paging_manager().ok_or("Paging not initialized")?;
    
    // Check canonical address
    match manager.mode() {
        PagingMode::FourLevel => {
            if !virt_addr.is_canonical_la48() {
                return Err("Virtual address not canonical for LA48");
            }
        }
        PagingMode::FiveLevel => {
            if !virt_addr.is_canonical_la57() {
                return Err("Virtual address not canonical for LA57");
            }
        }
    }
    
    let root_table_phys = manager.root_table_phys();
    
    unsafe {
        match manager.mode() {
            PagingMode::FiveLevel => {
                let pml5_index = virt_addr.pml5_index();
                let pml4_index = virt_addr.pml4_index();
                let pdpt_index = virt_addr.pdpt_index();
                let pd_index = virt_addr.pd_index();
                let pt_index = virt_addr.pt_index();
                
                // Walk to PML5
                let pml5 = phys_to_virt(root_table_phys) as *const PageTable;
                let pml5_entry = &(*pml5).entries[pml5_index];
                if !pml5_entry.is_present() {
                    return Err("Page not mapped (PML5)");
                }
                
                // Walk to PML4
                let pml4 = phys_to_virt(pml5_entry.address()) as *const PageTable;
                let pml4_entry = &(*pml4).entries[pml4_index];
                if !pml4_entry.is_present() {
                    return Err("Page not mapped (PML4)");
                }
                
                // Walk to PDPT
                let pdpt = phys_to_virt(pml4_entry.address()) as *const PageTable;
                let pdpt_entry = &(*pdpt).entries[pdpt_index];
                if !pdpt_entry.is_present() {
                    return Err("Page not mapped (PDPT)");
                }
                
                // Check for 1GB huge page
                if pdpt_entry.is_huge() {
                    let base = pdpt_entry.address() & !0x3FFF_FFFF; // Clear lower 30 bits
                    let offset = virt_addr.as_u64() & 0x3FFF_FFFF;  // Get lower 30 bits
                    return Ok(PhysAddr::new(base | offset));
                }
                
                // Walk to PD
                let pd = phys_to_virt(pdpt_entry.address()) as *const PageTable;
                let pd_entry = &(*pd).entries[pd_index];
                if !pd_entry.is_present() {
                    return Err("Page not mapped (PD)");
                }
                
                // Check for 2MB huge page
                if pd_entry.is_huge() {
                    let base = pd_entry.address() & !0x1F_FFFF; // Clear lower 21 bits
                    let offset = virt_addr.as_u64() & 0x1F_FFFF;  // Get lower 21 bits
                    return Ok(PhysAddr::new(base | offset));
                }
                
                // Walk to PT
                let pt = phys_to_virt(pd_entry.address()) as *const PageTable;
                let pt_entry = &(*pt).entries[pt_index];
                if !pt_entry.is_present() {
                    return Err("Page not mapped (PT)");
                }
                
                // Regular 4KB page
                let base = pt_entry.address();
                let offset = virt_addr.page_offset() as u64;
                Ok(PhysAddr::new(base | offset))
            }
            PagingMode::FourLevel => {
                let pml4_index = virt_addr.pml4_index();
                let pdpt_index = virt_addr.pdpt_index();
                let pd_index = virt_addr.pd_index();
                let pt_index = virt_addr.pt_index();
                
                // Walk to PML4
                let pml4 = phys_to_virt(root_table_phys) as *const PageTable;
                let pml4_entry = &(*pml4).entries[pml4_index];
                if !pml4_entry.is_present() {
                    return Err("Page not mapped (PML4)");
                }
                
                // Walk to PDPT
                let pdpt = phys_to_virt(pml4_entry.address()) as *const PageTable;
                let pdpt_entry = &(*pdpt).entries[pdpt_index];
                if !pdpt_entry.is_present() {
                    return Err("Page not mapped (PDPT)");
                }
                
                // Check for 1GB huge page
                if pdpt_entry.is_huge() {
                    let base = pdpt_entry.address() & !0x3FFF_FFFF;
                    let offset = virt_addr.as_u64() & 0x3FFF_FFFF;
                    return Ok(PhysAddr::new(base | offset));
                }
                
                // Walk to PD
                let pd = phys_to_virt(pdpt_entry.address()) as *const PageTable;
                let pd_entry = &(*pd).entries[pd_index];
                if !pd_entry.is_present() {
                    return Err("Page not mapped (PD)");
                }
                
                // Check for 2MB huge page
                if pd_entry.is_huge() {
                    let base = pd_entry.address() & !0x1F_FFFF;
                    let offset = virt_addr.as_u64() & 0x1F_FFFF;
                    return Ok(PhysAddr::new(base | offset));
                }
                
                // Walk to PT
                let pt = phys_to_virt(pd_entry.address()) as *const PageTable;
                let pt_entry = &(*pt).entries[pt_index];
                if !pt_entry.is_present() {
                    return Err("Page not mapped (PT)");
                }
                
                // Regular 4KB page
                let base = pt_entry.address();
                let offset = virt_addr.page_offset() as u64;
                Ok(PhysAddr::new(base | offset))
            }
        }
    }
}

/// Get the page table flags for a virtual address
/// Returns the flags from the page table entry
pub fn get_page_flags(virt_addr: VirtAddr) -> Result<PageTableFlags, &'static str> {
    let manager = get_paging_manager().ok_or("Paging not initialized")?;
    
    // Check canonical address
    match manager.mode() {
        PagingMode::FourLevel => {
            if !virt_addr.is_canonical_la48() {
                return Err("Virtual address not canonical for LA48");
            }
        }
        PagingMode::FiveLevel => {
            if !virt_addr.is_canonical_la57() {
                return Err("Virtual address not canonical for LA57");
            }
        }
    }
    
    let root_table_phys = manager.root_table_phys();
    
    unsafe {
        match manager.mode() {
            PagingMode::FiveLevel => {
                let pml5_index = virt_addr.pml5_index();
                let pml4_index = virt_addr.pml4_index();
                let pdpt_index = virt_addr.pdpt_index();
                let pd_index = virt_addr.pd_index();
                let pt_index = virt_addr.pt_index();
                
                // Walk page tables to PT entry
                let pml5 = phys_to_virt(root_table_phys) as *const PageTable;
                let pml5_entry = &(*pml5).entries[pml5_index];
                if !pml5_entry.is_present() {
                    return Err("Page not mapped (PML5)");
                }
                
                let pml4 = phys_to_virt(pml5_entry.address()) as *const PageTable;
                let pml4_entry = &(*pml4).entries[pml4_index];
                if !pml4_entry.is_present() {
                    return Err("Page not mapped (PML4)");
                }
                
                let pdpt = phys_to_virt(pml4_entry.address()) as *const PageTable;
                let pdpt_entry = &(*pdpt).entries[pdpt_index];
                if !pdpt_entry.is_present() {
                    return Err("Page not mapped (PDPT)");
                }
                
                // Check for 1GB huge page
                if pdpt_entry.is_huge() {
                    return Ok(pdpt_entry.flags());
                }
                
                let pd = phys_to_virt(pdpt_entry.address()) as *const PageTable;
                let pd_entry = &(*pd).entries[pd_index];
                if !pd_entry.is_present() {
                    return Err("Page not mapped (PD)");
                }
                
                // Check for 2MB huge page
                if pd_entry.is_huge() {
                    return Ok(pd_entry.flags());
                }
                
                let pt = phys_to_virt(pd_entry.address()) as *const PageTable;
                let pt_entry = &(*pt).entries[pt_index];
                if !pt_entry.is_present() {
                    return Err("Page not mapped (PT)");
                }
                
                Ok(pt_entry.flags())
            }
            PagingMode::FourLevel => {
                let pml4_index = virt_addr.pml4_index();
                let pdpt_index = virt_addr.pdpt_index();
                let pd_index = virt_addr.pd_index();
                let pt_index = virt_addr.pt_index();
                
                // Walk page tables to PT entry
                let pml4 = phys_to_virt(root_table_phys) as *const PageTable;
                let pml4_entry = &(*pml4).entries[pml4_index];
                if !pml4_entry.is_present() {
                    return Err("Page not mapped (PML4)");
                }
                
                let pdpt = phys_to_virt(pml4_entry.address()) as *const PageTable;
                let pdpt_entry = &(*pdpt).entries[pdpt_index];
                if !pdpt_entry.is_present() {
                    return Err("Page not mapped (PDPT)");
                }
                
                // Check for 1GB huge page
                if pdpt_entry.is_huge() {
                    return Ok(pdpt_entry.flags());
                }
                
                let pd = phys_to_virt(pdpt_entry.address()) as *const PageTable;
                let pd_entry = &(*pd).entries[pd_index];
                if !pd_entry.is_present() {
                    return Err("Page not mapped (PD)");
                }
                
                // Check for 2MB huge page
                if pd_entry.is_huge() {
                    return Ok(pd_entry.flags());
                }
                
                let pt = phys_to_virt(pd_entry.address()) as *const PageTable;
                let pt_entry = &(*pt).entries[pt_index];
                if !pt_entry.is_present() {
                    return Err("Page not mapped (PT)");
                }
                
                Ok(pt_entry.flags())
            }
        }
    }
}

/// Identity map a range of physical memory
/// Maps physical addresses to the same virtual addresses
pub unsafe fn identity_map_range(
    start_phys: PhysAddr,
    end_phys: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    let manager = get_paging_manager().ok_or("Paging not initialized")?;
    
    let start = start_phys.align_down().as_u64();
    let end = end_phys.align_up().as_u64();
    
    let mut addr = start;
    while addr < end {
        let virt = VirtAddr::new(addr);
        let phys = PhysAddr::new(addr);
        map_page(virt, phys, flags)?;
        addr += PAGE_SIZE as u64;
    }
    
    Ok(())
}

/// Get the current active page table root address from CR3
pub fn get_current_page_table() -> u64 {
    read_cr3() & 0x000F_FFFF_FFFF_F000  // Mask off flags
}

/// Switch to a different page table
pub unsafe fn switch_page_table(new_root: u64) {
    write_cr3(new_root);
}

/// Create a new empty page table
pub fn create_page_table() -> PageTable {
    PageTable::new()
}

/// Map a range of virtual addresses to physical addresses
/// Useful for mapping large memory regions
pub unsafe fn map_range(
    virt_start: VirtAddr,
    phys_start: PhysAddr,
    size: usize,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    let page_count = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    
    for i in 0..page_count {
        let virt = VirtAddr::new(virt_start.as_u64() + (i * PAGE_SIZE) as u64);
        let phys = PhysAddr::new(phys_start.as_u64() + (i * PAGE_SIZE) as u64);
        map_page(virt, phys, flags)?;
    }
    
    Ok(())
}

/// Unmap a range of virtual addresses
pub unsafe fn unmap_range(virt_start: VirtAddr, size: usize) -> Result<(), &'static str> {
    let page_count = (size + PAGE_SIZE - 1) / PAGE_SIZE;
    
    for i in 0..page_count {
        let virt = VirtAddr::new(virt_start.as_u64() + (i * PAGE_SIZE) as u64);
        unmap_page(virt)?;
    }
    
    Ok(())
}

/// Remap a virtual address to a different physical address
/// This is more efficient than unmap + map as it doesn't free page tables
pub unsafe fn remap_page(
    virt_addr: VirtAddr,
    new_phys_addr: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    let manager = get_paging_manager().ok_or("Paging not initialized")?;
    
    // Validate addresses
    if !new_phys_addr.is_aligned() {
        return Err("Physical address not page-aligned");
    }
    
    let root_table_phys = manager.root_table_phys();
    
    unsafe {
        match manager.mode() {
            PagingMode::FiveLevel => {
                let pml5_index = virt_addr.pml5_index();
                let pml4_index = virt_addr.pml4_index();
                let pdpt_index = virt_addr.pdpt_index();
                let pd_index = virt_addr.pd_index();
                let pt_index = virt_addr.pt_index();
                
                // Walk to PT
                let pml5 = phys_to_virt(root_table_phys) as *const PageTable;
                let pml5_entry = &(*pml5).entries[pml5_index];
                if !pml5_entry.is_present() {
                    return Err("Page not mapped (PML5)");
                }
                
                let pml4 = phys_to_virt(pml5_entry.address()) as *const PageTable;
                let pml4_entry = &(*pml4).entries[pml4_index];
                if !pml4_entry.is_present() {
                    return Err("Page not mapped (PML4)");
                }
                
                let pdpt = phys_to_virt(pml4_entry.address()) as *const PageTable;
                let pdpt_entry = &(*pdpt).entries[pdpt_index];
                if !pdpt_entry.is_present() {
                    return Err("Page not mapped (PDPT)");
                }
                
                let pd = phys_to_virt(pdpt_entry.address()) as *const PageTable;
                let pd_entry = &(*pd).entries[pd_index];
                if !pd_entry.is_present() {
                    return Err("Page not mapped (PD)");
                }
                
                let pt = phys_to_virt(pd_entry.address()) as *mut PageTable;
                let pt_entry = &mut (*pt).entries[pt_index];
                if !pt_entry.is_present() {
                    return Err("Page not mapped (PT)");
                }
                
                // Update the entry
                pt_entry.set(new_phys_addr.as_u64(), flags);
            }
            PagingMode::FourLevel => {
                let pml4_index = virt_addr.pml4_index();
                let pdpt_index = virt_addr.pdpt_index();
                let pd_index = virt_addr.pd_index();
                let pt_index = virt_addr.pt_index();
                
                // Walk to PT
                let pml4 = phys_to_virt(root_table_phys) as *const PageTable;
                let pml4_entry = &(*pml4).entries[pml4_index];
                if !pml4_entry.is_present() {
                    return Err("Page not mapped (PML4)");
                }
                
                let pdpt = phys_to_virt(pml4_entry.address()) as *const PageTable;
                let pdpt_entry = &(*pdpt).entries[pdpt_index];
                if !pdpt_entry.is_present() {
                    return Err("Page not mapped (PDPT)");
                }
                
                let pd = phys_to_virt(pdpt_entry.address()) as *const PageTable;
                let pd_entry = &(*pd).entries[pd_index];
                if !pd_entry.is_present() {
                    return Err("Page not mapped (PD)");
                }
                
                let pt = phys_to_virt(pd_entry.address()) as *mut PageTable;
                let pt_entry = &mut (*pt).entries[pt_index];
                if !pt_entry.is_present() {
                    return Err("Page not mapped (PT)");
                }
                
                // Update the entry
                pt_entry.set(new_phys_addr.as_u64(), flags);
            }
        }
    }
    
    // Flush TLB for this address
    invlpg(virt_addr.as_u64());
    
    Ok(())
}

/// Change page protection flags without remapping
pub unsafe fn change_page_flags(
    virt_addr: VirtAddr,
    new_flags: PageTableFlags,
) -> Result<(), &'static str> {
    // Get current physical address
    let phys_addr = translate_address(virt_addr)?;
    
    // Remap with new flags
    remap_page(virt_addr, phys_addr, new_flags)
}


// =============================================================================
// C-Compatible Interface
// =============================================================================

/// Check LA57 support (C wrapper)
#[no_mangle]
pub extern "C" fn paging_check_la57_support() -> u8 {
    if check_la57_support() { 1 } else { 0 }
}

/// Check NX support (C wrapper)
#[no_mangle]
pub extern "C" fn paging_check_nx_support() -> u8 {
    if check_nx_support() { 1 } else { 0 }
}

/// Check if LA57 is enabled (C wrapper)
#[no_mangle]
pub extern "C" fn paging_is_la57_enabled() -> u8 {
    if is_la57_enabled() { 1 } else { 0 }
}

/// Test function to verify C calling works
#[no_mangle]
pub extern "C" fn paging_test_simple(a: u64, b: u64) -> u64 {
    a + b
}

/// Get current CR3 value (C wrapper)
#[no_mangle]
pub extern "C" fn paging_read_cr3() -> u64 {
    read_cr3()
}

/// Initialize paging (C wrapper)
#[no_mangle]
pub extern "C" fn paging_init_c(root_table_phys: u64) -> i32 {
    match paging_init(root_table_phys) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Get paging mode (C wrapper): 0 = 4-level, 1 = 5-level, -1 = not initialized
#[no_mangle]
pub extern "C" fn paging_get_mode() -> i32 {
    match get_paging_manager() {
        Some(mgr) => match mgr.mode() {
            PagingMode::FourLevel => 0,
            PagingMode::FiveLevel => 1,
        },
        None => -1,
    }
}

/// Invalidate TLB entry (C wrapper)
#[no_mangle]
pub unsafe extern "C" fn paging_invlpg(virt_addr: u64) {
    invlpg(virt_addr);
}

/// Flush entire TLB (C wrapper)
#[no_mangle]
pub unsafe extern "C" fn paging_flush_tlb() {
    flush_tlb();
}

/// Map a page (C wrapper) - delegates to the Rust helper to avoid duplicate logic
#[no_mangle]
pub unsafe extern "C" fn paging_map_page(virt_addr: u64, phys_addr: u64, flags: u64) -> i32 {
    let virt = VirtAddr::new(virt_addr);
    let phys = PhysAddr::new(phys_addr);

    let mut page_flags = PageTableFlags::from_raw(flags);
    page_flags.set(PageTableFlags::PRESENT);

    match map_page(virt, phys, page_flags) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Unmap a page (C wrapper) - delegates to the Rust helper
/// Returns 0 on success, negative error code on failure
#[no_mangle]
pub unsafe extern "C" fn paging_unmap_page(virt_addr: u64) -> i32 {
    let virt = VirtAddr::new(virt_addr);
    match unmap_page(virt) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Translate virtual to physical address (C wrapper)
/// Returns physical address on success, 0 on error
#[no_mangle]
pub extern "C" fn paging_translate_address(virt_addr: u64) -> u64 {
    let virt = VirtAddr::new(virt_addr);
    
    match translate_address(virt) {
        Ok(phys) => phys.as_u64(),
        Err(_) => 0,
    }
}

/// Check if a page is mapped (C wrapper)
/// Returns 1 if mapped, 0 if not mapped
#[no_mangle]
pub extern "C" fn paging_is_mapped(virt_addr: u64) -> u8 {
    let virt = VirtAddr::new(virt_addr);
    match translate_address(virt) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// Identity map a range (C wrapper)
/// Returns 0 on success, -1 on error
#[no_mangle]
pub unsafe extern "C" fn paging_identity_map_range(
    start_phys: u64,
    end_phys: u64,
    flags: u64
) -> i32 {
    let start = PhysAddr::new(start_phys);
    let end = PhysAddr::new(end_phys);
    let page_flags = PageTableFlags::from_raw(flags);
    
    match identity_map_range(start, end, page_flags) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}
