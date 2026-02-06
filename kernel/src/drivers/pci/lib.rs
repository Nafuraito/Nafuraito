//! Nafuraito Kernel - PCI Driver Library
//! =============================================================================
//!
//! **PCI** (Peripheral Component Interconnect) is the standard bus for
//! connecting devices to the computer. 
//!
//! What this driver does:
//! 
//! 1. **Device Discovery**: Scan the PCI bus to find all connected devices
//!    - Walk through all possible bus/device/function combinations
//!    - Read vendor ID - if it's 0xFFFF, slot is empty
//!    - For each device, read its capabilities and resources
//!
//! 2. **Configuration Space Access**: Read/write device configuration
//!    - Every PCI device has a 256-byte (or 4KB for PCIe) config space
//!    - Contains vendor/device ID, class code, BARs, capabilities, etc.
//!    - It accesses it via I/O ports 0xCF8 (address) and 0xCFC (data)
//!
//! 3. **Resource Management**: Handle memory and I/O resources
//!    - BARs (Base Address Registers) tell us what memory/IO the device needs
//!    - It reads BARs to find device registers, framebuffers, etc.
//!
//! 4. **Interrupt Handling**: Set up MSI/MSI-X for modern interrupt delivery
//!    - Old way: Shared IRQ lines (slow, messy)
//!    - New way: MSI - device writes to memory to trigger interrupt (fast!)
//!    - MSI-X: Like MSI but supports thousands of interrupts per device
//!
//! 5. **Capability Parsing**: Read extended device features
//!    - PCI capabilities are a linked list of feature descriptors
//!    - E.g., Power Management, MSI, PCIe, Virtualization
//!
//! =============================================================================

#![crate_type = "rlib"]
#![crate_name = "pci"]
#![no_std]
#![allow(unused)]

// Include MSI/MSI-X module
pub mod msi;

// ============================================================================
// PCI CONFIGURATION SPACE CONSTANTS 
//
// Every PCI device has a configuration space - 256 bytes of registers.
// These constants define the byte offsets of important fields.
// Think of it like a struct, but accessed via I/O ports instead of pointers.
// ============================================================================

/// PCI Configuration Address Port (I/O)
/// We write an address here to select which config register we want to access
const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
/// PCI Configuration Data Port (I/O)  
/// After selecting an address, we read/write the data here
const PCI_CONFIG_DATA: u16 = 0xCFC;

/// PCI Configuration Space Register Offsets
pub const PCI_VENDOR_ID: u8 = 0x00;
pub const PCI_DEVICE_ID: u8 = 0x02;
pub const PCI_COMMAND: u8 = 0x04;
pub const PCI_STATUS: u8 = 0x06;
pub const PCI_REVISION_ID: u8 = 0x08;
pub const PCI_PROG_IF: u8 = 0x09;
pub const PCI_SUBCLASS: u8 = 0x0A;
pub const PCI_CLASS: u8 = 0x0B;
pub const PCI_CACHE_LINE_SIZE: u8 = 0x0C;
pub const PCI_LATENCY_TIMER: u8 = 0x0D;
pub const PCI_HEADER_TYPE: u8 = 0x0E;
pub const PCI_BIST: u8 = 0x0F;
pub const PCI_BAR0: u8 = 0x10;
pub const PCI_BAR1: u8 = 0x14;
pub const PCI_BAR2: u8 = 0x18;
pub const PCI_BAR3: u8 = 0x1C;
pub const PCI_BAR4: u8 = 0x20;
pub const PCI_BAR5: u8 = 0x24;
pub const PCI_CARDBUS_CIS: u8 = 0x28;
pub const PCI_SUBSYSTEM_VENDOR_ID: u8 = 0x2C;
pub const PCI_SUBSYSTEM_ID: u8 = 0x2E;
pub const PCI_EXPANSION_ROM_BASE: u8 = 0x30;
pub const PCI_CAPABILITIES_PTR: u8 = 0x34;
pub const PCI_INTERRUPT_LINE: u8 = 0x3C;
pub const PCI_INTERRUPT_PIN: u8 = 0x3D;
pub const PCI_MIN_GRANT: u8 = 0x3E;
pub const PCI_MAX_LATENCY: u8 = 0x3F;

/// PCI Command Register bits
pub const PCI_COMMAND_IO_SPACE: u16 = 1 << 0;
pub const PCI_COMMAND_MEMORY_SPACE: u16 = 1 << 1;
pub const PCI_COMMAND_BUS_MASTER: u16 = 1 << 2;
pub const PCI_COMMAND_SPECIAL_CYCLES: u16 = 1 << 3;
pub const PCI_COMMAND_MWI_ENABLE: u16 = 1 << 4;
pub const PCI_COMMAND_VGA_PALETTE_SNOOP: u16 = 1 << 5;
pub const PCI_COMMAND_PARITY_ERROR_RESPONSE: u16 = 1 << 6;
pub const PCI_COMMAND_SERR_ENABLE: u16 = 1 << 8;
pub const PCI_COMMAND_FAST_B2B_ENABLE: u16 = 1 << 9;
pub const PCI_COMMAND_INTERRUPT_DISABLE: u16 = 1 << 10;

/// PCI Status Register bits
pub const PCI_STATUS_INTERRUPT: u16 = 1 << 3;
pub const PCI_STATUS_CAPABILITIES_LIST: u16 = 1 << 4;
pub const PCI_STATUS_66MHZ_CAPABLE: u16 = 1 << 5;
pub const PCI_STATUS_FAST_B2B_CAPABLE: u16 = 1 << 7;
pub const PCI_STATUS_MASTER_DATA_PARITY_ERROR: u16 = 1 << 8;
pub const PCI_STATUS_DEVSEL_TIMING: u16 = 0x600;
pub const PCI_STATUS_SIGNALED_TARGET_ABORT: u16 = 1 << 11;
pub const PCI_STATUS_RECEIVED_TARGET_ABORT: u16 = 1 << 12;
pub const PCI_STATUS_RECEIVED_MASTER_ABORT: u16 = 1 << 13;
pub const PCI_STATUS_SIGNALED_SYSTEM_ERROR: u16 = 1 << 14;
pub const PCI_STATUS_DETECTED_PARITY_ERROR: u16 = 1 << 15;

/// PCI Capability IDs
pub const PCI_CAP_ID_PM: u8 = 0x01;        // Power Management
pub const PCI_CAP_ID_AGP: u8 = 0x02;       // AGP
pub const PCI_CAP_ID_VPD: u8 = 0x03;       // Vital Product Data
pub const PCI_CAP_ID_SLOTID: u8 = 0x04;    // Slot Identification
pub const PCI_CAP_ID_MSI: u8 = 0x05;       // Message Signaled Interrupts
pub const PCI_CAP_ID_CHSWP: u8 = 0x06;     // CompactPCI HotSwap
pub const PCI_CAP_ID_PCIX: u8 = 0x07;      // PCI-X
pub const PCI_CAP_ID_HT: u8 = 0x08;        // HyperTransport
pub const PCI_CAP_ID_VNDR: u8 = 0x09;      // Vendor Specific
pub const PCI_CAP_ID_DBG: u8 = 0x0A;       // Debug port
pub const PCI_CAP_ID_CCRC: u8 = 0x0B;      // CompactPCI Central Resource Control
pub const PCI_CAP_ID_SHPC: u8 = 0x0C;      // PCI Standard Hot-Plug Controller
pub const PCI_CAP_ID_SSVID: u8 = 0x0D;     // Bridge subsystem vendor/device ID
pub const PCI_CAP_ID_AGP3: u8 = 0x0E;      // AGP Target PCI-PCI bridge
pub const PCI_CAP_ID_EXP: u8 = 0x10;       // PCI Express
pub const PCI_CAP_ID_MSIX: u8 = 0x11;      // MSI-X
pub const PCI_CAP_ID_AF: u8 = 0x13;        // Advanced Features

/// Header type values
pub const PCI_HEADER_TYPE_NORMAL: u8 = 0x00;
pub const PCI_HEADER_TYPE_BRIDGE: u8 = 0x01;
pub const PCI_HEADER_TYPE_CARDBUS: u8 = 0x02;
pub const PCI_HEADER_TYPE_MULTI_FUNCTION: u8 = 0x80;

/// Invalid vendor ID (device not present)
pub const PCI_VENDOR_ID_INVALID: u16 = 0xFFFF;

// ============================================================================
// LOW-LEVEL I/O HELPERS
// ============================================================================

#[inline]
pub unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}

#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    core::arch::asm!("in al, dx", out("al") ret, in("dx") port, options(nomem, nostack));
    ret
}

#[inline]
pub unsafe fn outw(port: u16, val: u16) {
    core::arch::asm!("out dx, ax", in("dx") port, in("ax") val, options(nomem, nostack));
}

#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let ret: u16;
    core::arch::asm!("in ax, dx", out("ax") ret, in("dx") port, options(nomem, nostack));
    ret
}

#[inline]
pub unsafe fn outl(port: u16, val: u32) {
    core::arch::asm!("out dx, eax", in("dx") port, in("eax") val, options(nomem, nostack));
}

#[inline]
pub unsafe fn inl(port: u16) -> u32 {
    let ret: u32;
    core::arch::asm!("in eax, dx", out("eax") ret, in("dx") port, options(nomem, nostack));
    ret
}

#[inline]
pub unsafe fn read_volatile_u32(addr: u64) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}

#[inline]
pub unsafe fn write_volatile_u32(addr: u64, value: u32) {
    core::ptr::write_volatile(addr as *mut u32, value);
}

#[inline]
pub unsafe fn read_volatile_u64(addr: u64) -> u64 {
    core::ptr::read_volatile(addr as *const u64)
}

#[inline]
pub unsafe fn write_volatile_u64(addr: u64, value: u64) {
    core::ptr::write_volatile(addr as *mut u64, value);
}

// ============================================================================
// PCI ADDRESS REPRESENTATION
// ============================================================================

/// Represents a PCI device address (bus, device, function)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct PciAddress {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
}

impl PciAddress {
    /// Create a new PCI address
    pub const fn new(bus: u8, device: u8, function: u8) -> Self {
        Self {
            bus,
            device: device & 0x1F,      // 5 bits for device
            function: function & 0x07,   // 3 bits for function
        }
    }

    /// Build the PCI configuration address for I/O port access
    pub const fn config_address(&self, offset: u8) -> u32 {
        let offset = offset & 0xFC; // Align to 4-byte boundary
        0x8000_0000
            | ((self.bus as u32) << 16)
            | ((self.device as u32) << 11)
            | ((self.function as u32) << 8)
            | (offset as u32)
    }

    /// Build the ECAM (Enhanced Configuration Access Mechanism) address
    /// for memory-mapped configuration space access
    pub const fn ecam_address(&self, ecam_base: u64, offset: u16) -> u64 {
        ecam_base
            + ((self.bus as u64) << 20)
            + ((self.device as u64) << 15)
            + ((self.function as u64) << 12)
            + (offset as u64)
    }
}

// ============================================================================
// PCI CONFIGURATION SPACE ACCESS (I/O BASED)
// ============================================================================

/// Read 32-bit value from PCI configuration space using I/O ports
#[no_mangle]
pub unsafe fn pci_config_read32(addr: &PciAddress, offset: u8) -> u32 {
    outl(PCI_CONFIG_ADDRESS, addr.config_address(offset));
    inl(PCI_CONFIG_DATA)
}

/// Read 16-bit value from PCI configuration space using I/O ports
#[no_mangle]
pub unsafe fn pci_config_read16(addr: &PciAddress, offset: u8) -> u16 {
    outl(PCI_CONFIG_ADDRESS, addr.config_address(offset));
    ((inl(PCI_CONFIG_DATA) >> ((offset & 2) * 8)) & 0xFFFF) as u16
}

/// Read 8-bit value from PCI configuration space using I/O ports
#[no_mangle]
pub unsafe fn pci_config_read8(addr: &PciAddress, offset: u8) -> u8 {
    outl(PCI_CONFIG_ADDRESS, addr.config_address(offset));
    ((inl(PCI_CONFIG_DATA) >> ((offset & 3) * 8)) & 0xFF) as u8
}

/// Write 32-bit value to PCI configuration space using I/O ports
#[no_mangle]
pub unsafe fn pci_config_write32(addr: &PciAddress, offset: u8, value: u32) {
    outl(PCI_CONFIG_ADDRESS, addr.config_address(offset));
    outl(PCI_CONFIG_DATA, value);
}

/// Write 16-bit value to PCI configuration space using I/O ports
#[no_mangle]
pub unsafe fn pci_config_write16(addr: &PciAddress, offset: u8, value: u16) {
    outl(PCI_CONFIG_ADDRESS, addr.config_address(offset));
    let shift = (offset & 2) * 8;
    let data = inl(PCI_CONFIG_DATA);
    let mask = !(0xFFFF << shift);
    outl(PCI_CONFIG_DATA, (data & mask) | ((value as u32) << shift));
}

/// Write 8-bit value to PCI configuration space using I/O ports
#[no_mangle]
pub unsafe fn pci_config_write8(addr: &PciAddress, offset: u8, value: u8) {
    outl(PCI_CONFIG_ADDRESS, addr.config_address(offset));
    let shift = (offset & 3) * 8;
    let data = inl(PCI_CONFIG_DATA);
    let mask = !(0xFF << shift);
    outl(PCI_CONFIG_DATA, (data & mask) | ((value as u32) << shift));
}

// ============================================================================
// PCI CONFIGURATION SPACE ACCESS (MEMORY-MAPPED / ECAM)
// ============================================================================

/// Read 32-bit value from PCI configuration space using ECAM
#[no_mangle]
pub unsafe fn pci_ecam_read32(ecam_base: u64, addr: &PciAddress, offset: u16) -> u32 {
    read_volatile_u32(addr.ecam_address(ecam_base, offset & 0xFFC))
}

/// Read 16-bit value from PCI configuration space using ECAM
#[no_mangle]
pub unsafe fn pci_ecam_read16(ecam_base: u64, addr: &PciAddress, offset: u16) -> u16 {
    let aligned_offset = offset & 0xFFE;
    let base_addr = addr.ecam_address(ecam_base, aligned_offset);
    core::ptr::read_volatile(base_addr as *const u16)
}

/// Read 8-bit value from PCI configuration space using ECAM
#[no_mangle]
pub unsafe fn pci_ecam_read8(ecam_base: u64, addr: &PciAddress, offset: u16) -> u8 {
    let base_addr = addr.ecam_address(ecam_base, offset);
    core::ptr::read_volatile(base_addr as *const u8)
}

/// Write 32-bit value to PCI configuration space using ECAM
#[no_mangle]
pub unsafe fn pci_ecam_write32(ecam_base: u64, addr: &PciAddress, offset: u16, value: u32) {
    write_volatile_u32(addr.ecam_address(ecam_base, offset & 0xFFC), value);
}

/// Write 16-bit value to PCI configuration space using ECAM
#[no_mangle]
pub unsafe fn pci_ecam_write16(ecam_base: u64, addr: &PciAddress, offset: u16, value: u16) {
    let aligned_offset = offset & 0xFFE;
    let base_addr = addr.ecam_address(ecam_base, aligned_offset);
    core::ptr::write_volatile(base_addr as *mut u16, value);
}

/// Write 8-bit value to PCI configuration space using ECAM
#[no_mangle]
pub unsafe fn pci_ecam_write8(ecam_base: u64, addr: &PciAddress, offset: u16, value: u8) {
    let base_addr = addr.ecam_address(ecam_base, offset);
    core::ptr::write_volatile(base_addr as *mut u8, value);
}

// ============================================================================
// BAR (BASE ADDRESS REGISTER) HANDLING
// ============================================================================

/// BAR type enumeration
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum BarType {
    /// I/O space BAR
    IoSpace,
    /// 32-bit memory-mapped BAR
    Memory32,
    /// 64-bit memory-mapped BAR
    Memory64,
    /// Invalid or not present
    None,
}

/// BAR information structure
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Bar {
    /// BAR type
    pub bar_type: BarType,
    /// Base address
    pub base_address: u64,
    /// Size of the region
    pub size: u64,
    /// Is prefetchable (for memory BARs)
    pub prefetchable: bool,
}

impl Bar {
    /// Create an empty/invalid BAR
    pub const fn none() -> Self {
        Self {
            bar_type: BarType::None,
            base_address: 0,
            size: 0,
            prefetchable: false,
        }
    }
}

/// Read and decode a BAR from a PCI device
#[no_mangle]
pub unsafe fn pci_read_bar(addr: &PciAddress, bar_index: u8) -> Bar {
    if bar_index >= 6 {
        return Bar::none();
    }

    let bar_offset = PCI_BAR0 + (bar_index * 4);
    let bar_value = pci_config_read32(addr, bar_offset);

    if bar_value == 0 {
        return Bar::none();
    }

    // Check if I/O space or memory space
    if (bar_value & 0x01) != 0 {
        // I/O space BAR
        let base_address = (bar_value & 0xFFFF_FFFC) as u64;
        
        // Determine size by writing all 1s and reading back
        pci_config_write32(addr, bar_offset, 0xFFFF_FFFF);
        let size_mask = pci_config_read32(addr, bar_offset);
        pci_config_write32(addr, bar_offset, bar_value); // Restore original
        
        let size = !((size_mask & 0xFFFF_FFFC) as u64) + 1;

        Bar {
            bar_type: BarType::IoSpace,
            base_address,
            size: size & 0xFFFF, // I/O space is limited to 64KB
            prefetchable: false,
        }
    } else {
        // Memory space BAR
        let bar_type_bits = (bar_value >> 1) & 0x03;
        let prefetchable = (bar_value & 0x08) != 0;

        match bar_type_bits {
            0b00 => {
                // 32-bit memory BAR
                let base_address = (bar_value & 0xFFFF_FFF0) as u64;
                
                // Determine size
                pci_config_write32(addr, bar_offset, 0xFFFF_FFFF);
                let size_mask = pci_config_read32(addr, bar_offset);
                pci_config_write32(addr, bar_offset, bar_value);
                
                let size = !((size_mask & 0xFFFF_FFF0) as u64) + 1;

                Bar {
                    bar_type: BarType::Memory32,
                    base_address,
                    size,
                    prefetchable,
                }
            }
            0b10 => {
                // 64-bit memory BAR
                if bar_index >= 5 {
                    return Bar::none(); // Can't have 64-bit BAR at index 5
                }

                let bar_high_offset = bar_offset + 4;
                let bar_high = pci_config_read32(addr, bar_high_offset);
                let base_address = ((bar_high as u64) << 32) | ((bar_value & 0xFFFF_FFF0) as u64);

                // Determine size
                pci_config_write32(addr, bar_offset, 0xFFFF_FFFF);
                pci_config_write32(addr, bar_high_offset, 0xFFFF_FFFF);
                let size_mask_low = pci_config_read32(addr, bar_offset);
                let size_mask_high = pci_config_read32(addr, bar_high_offset);
                pci_config_write32(addr, bar_offset, bar_value);
                pci_config_write32(addr, bar_high_offset, bar_high);

                let size_mask = ((size_mask_high as u64) << 32) | ((size_mask_low & 0xFFFF_FFF0) as u64);
                let size = !size_mask + 1;

                Bar {
                    bar_type: BarType::Memory64,
                    base_address,
                    size,
                    prefetchable,
                }
            }
            _ => Bar::none(),
        }
    }
}

// ============================================================================
// PCI CAPABILITY LIST HANDLING
// ============================================================================

/// Find a capability in the PCI capabilities list
#[no_mangle]
pub unsafe fn pci_find_capability(addr: &PciAddress, cap_id: u8) -> Option<u8> {
    // Check if device supports capabilities list
    let status = pci_config_read16(addr, PCI_STATUS);
    if (status & PCI_STATUS_CAPABILITIES_LIST) == 0 {
        return None;
    }

    // Get capabilities pointer
    let mut cap_offset = pci_config_read8(addr, PCI_CAPABILITIES_PTR) & 0xFC;

    // Walk the capabilities list
    let mut iterations = 0;
    while cap_offset != 0 && iterations < 48 {
        let cap_header = pci_config_read16(addr, cap_offset);
        let current_id = (cap_header & 0xFF) as u8;
        
        if current_id == cap_id {
            return Some(cap_offset);
        }

        cap_offset = ((cap_header >> 8) & 0xFC) as u8;
        iterations += 1;
    }

    None
}

/// Iterate through all capabilities
pub struct CapabilityIterator {
    addr: PciAddress,
    next_offset: u8,
    iterations: u8,
}

impl CapabilityIterator {
    /// Create a new capability iterator
    pub unsafe fn new(addr: &PciAddress) -> Option<Self> {
        let status = pci_config_read16(addr, PCI_STATUS);
        if (status & PCI_STATUS_CAPABILITIES_LIST) == 0 {
            return None;
        }

        let first_offset = pci_config_read8(addr, PCI_CAPABILITIES_PTR) & 0xFC;
        Some(Self {
            addr: *addr,
            next_offset: first_offset,
            iterations: 0,
        })
    }
}

/// Capability information
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Capability {
    pub id: u8,
    pub offset: u8,
}

impl Iterator for CapabilityIterator {
    type Item = Capability;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_offset == 0 || self.iterations >= 48 {
            return None;
        }

        self.iterations += 1;

        unsafe {
            let cap_header = pci_config_read16(&self.addr, self.next_offset);
            let cap_id = (cap_header & 0xFF) as u8;
            let cap_offset = self.next_offset;

            self.next_offset = ((cap_header >> 8) & 0xFC) as u8;

            Some(Capability {
                id: cap_id,
                offset: cap_offset,
            })
        }
    }
}

// ============================================================================
// PCI DEVICE STRUCTURE
// ============================================================================

/// PCI device information structure
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PciDevice {
    /// Device address
    pub address: PciAddress,
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id: u16,
    /// Class code
    pub class_code: u8,
    /// Subclass code
    pub subclass: u8,
    /// Programming interface
    pub prog_if: u8,
    /// Revision ID
    pub revision_id: u8,
    /// Header type
    pub header_type: u8,
    /// MSI capability offset (0 if not present)
    pub msi_cap_offset: u8,
    /// MSI-X capability offset (0 if not present)
    pub msix_cap_offset: u8,
}

impl PciDevice {
    /// Check if device exists
    pub unsafe fn exists(addr: &PciAddress) -> bool {
        pci_config_read16(addr, PCI_VENDOR_ID) != PCI_VENDOR_ID_INVALID
    }

    /// Read device information from PCI configuration space
    pub unsafe fn read(addr: &PciAddress) -> Option<Self> {
        let vendor_id = pci_config_read16(addr, PCI_VENDOR_ID);
        if vendor_id == PCI_VENDOR_ID_INVALID {
            return None;
        }

        let device_id = pci_config_read16(addr, PCI_DEVICE_ID);
        let class_code = pci_config_read8(addr, PCI_CLASS);
        let subclass = pci_config_read8(addr, PCI_SUBCLASS);
        let prog_if = pci_config_read8(addr, PCI_PROG_IF);
        let revision_id = pci_config_read8(addr, PCI_REVISION_ID);
        let header_type = pci_config_read8(addr, PCI_HEADER_TYPE);

        // Find MSI and MSI-X capabilities
        let msi_cap_offset = pci_find_capability(addr, PCI_CAP_ID_MSI).unwrap_or(0);
        let msix_cap_offset = pci_find_capability(addr, PCI_CAP_ID_MSIX).unwrap_or(0);

        Some(Self {
            address: *addr,
            vendor_id,
            device_id,
            class_code,
            subclass,
            prog_if,
            revision_id,
            header_type,
            msi_cap_offset,
            msix_cap_offset,
        })
    }

    /// Check if this is a multi-function device
    pub fn is_multi_function(&self) -> bool {
        (self.header_type & PCI_HEADER_TYPE_MULTI_FUNCTION) != 0
    }

    /// Check if MSI is supported
    pub fn supports_msi(&self) -> bool {
        self.msi_cap_offset != 0
    }

    /// Check if MSI-X is supported
    pub fn supports_msix(&self) -> bool {
        self.msix_cap_offset != 0
    }

    /// Enable bus mastering for DMA
    pub unsafe fn enable_bus_master(&self) {
        let cmd = pci_config_read16(&self.address, PCI_COMMAND);
        pci_config_write16(&self.address, PCI_COMMAND, cmd | PCI_COMMAND_BUS_MASTER);
    }

    /// Enable memory space access
    pub unsafe fn enable_memory_space(&self) {
        let cmd = pci_config_read16(&self.address, PCI_COMMAND);
        pci_config_write16(&self.address, PCI_COMMAND, cmd | PCI_COMMAND_MEMORY_SPACE);
    }

    /// Enable I/O space access
    pub unsafe fn enable_io_space(&self) {
        let cmd = pci_config_read16(&self.address, PCI_COMMAND);
        pci_config_write16(&self.address, PCI_COMMAND, cmd | PCI_COMMAND_IO_SPACE);
    }

    /// Disable legacy interrupts
    pub unsafe fn disable_legacy_interrupts(&self) {
        let cmd = pci_config_read16(&self.address, PCI_COMMAND);
        pci_config_write16(&self.address, PCI_COMMAND, cmd | PCI_COMMAND_INTERRUPT_DISABLE);
    }

    /// Enable legacy interrupts
    pub unsafe fn enable_legacy_interrupts(&self) {
        let cmd = pci_config_read16(&self.address, PCI_COMMAND);
        pci_config_write16(&self.address, PCI_COMMAND, cmd & !PCI_COMMAND_INTERRUPT_DISABLE);
    }

    /// Read a BAR
    pub unsafe fn read_bar(&self, bar_index: u8) -> Bar {
        pci_read_bar(&self.address, bar_index)
    }
}

// ============================================================================
// PCI BUS ENUMERATION
// ============================================================================

/// Maximum number of devices we can track
const MAX_PCI_DEVICES: usize = 256;

/// Static storage for discovered PCI devices
static mut PCI_DEVICES: [Option<PciDevice>; MAX_PCI_DEVICES] = [None; MAX_PCI_DEVICES];
static mut PCI_DEVICE_COUNT: usize = 0;

/// Enumerate all PCI devices on the system
#[no_mangle]
pub unsafe fn pci_enumerate() -> usize {
    PCI_DEVICE_COUNT = 0;

    // Check if the host bridge is multi-function
    let host_addr = PciAddress::new(0, 0, 0);
    let host_header = pci_config_read8(&host_addr, PCI_HEADER_TYPE);
    
    if (host_header & PCI_HEADER_TYPE_MULTI_FUNCTION) != 0 {
        // Multiple PCI host controllers
        for function in 0..8 {
            let addr = PciAddress::new(0, 0, function);
            if PciDevice::exists(&addr) {
                pci_enumerate_bus(function);
            }
        }
    } else {
        // Single PCI bus
        pci_enumerate_bus(0);
    }

    PCI_DEVICE_COUNT
}

/// Enumerate a specific PCI bus
unsafe fn pci_enumerate_bus(bus: u8) {
    for device in 0..32 {
        pci_enumerate_device(bus, device);
    }
}

/// Enumerate a specific PCI device
unsafe fn pci_enumerate_device(bus: u8, device: u8) {
    let addr = PciAddress::new(bus, device, 0);
    
    if !PciDevice::exists(&addr) {
        return;
    }

    pci_enumerate_function(bus, device, 0);

    // Check if multi-function device
    let header_type = pci_config_read8(&addr, PCI_HEADER_TYPE);
    if (header_type & PCI_HEADER_TYPE_MULTI_FUNCTION) != 0 {
        for function in 1..8 {
            let func_addr = PciAddress::new(bus, device, function);
            if PciDevice::exists(&func_addr) {
                pci_enumerate_function(bus, device, function);
            }
        }
    }
}

/// Enumerate a specific PCI function
unsafe fn pci_enumerate_function(bus: u8, device: u8, function: u8) {
    let addr = PciAddress::new(bus, device, function);

    if let Some(pci_dev) = PciDevice::read(&addr) {
        if PCI_DEVICE_COUNT < MAX_PCI_DEVICES {
            PCI_DEVICES[PCI_DEVICE_COUNT] = Some(pci_dev);
            PCI_DEVICE_COUNT += 1;
        }

        // Check if this is a PCI-to-PCI bridge
        if pci_dev.class_code == 0x06 && pci_dev.subclass == 0x04 {
            // Read secondary bus number
            let secondary_bus = pci_config_read8(&addr, 0x19);
            pci_enumerate_bus(secondary_bus);
        }
    }
}

/// Get the number of discovered PCI devices
#[no_mangle]
pub unsafe fn pci_get_device_count() -> usize {
    PCI_DEVICE_COUNT
}

/// Get a discovered PCI device by index
#[no_mangle]
pub unsafe fn pci_get_device(index: usize) -> Option<&'static PciDevice> {
    if index < PCI_DEVICE_COUNT && index < MAX_PCI_DEVICES {
        (*PCI_DEVICES.as_ptr().add(index)).as_ref()
    } else {
        None
    }
}

/// Find a PCI device by vendor and device ID
#[no_mangle]
pub unsafe fn pci_find_device(vendor_id: u16, device_id: u16) -> Option<&'static PciDevice> {
    let mut i = 0usize;
    while i < PCI_DEVICE_COUNT && i < MAX_PCI_DEVICES {
        if let Some(ref dev) = *PCI_DEVICES.as_ptr().add(i) {
            if dev.vendor_id == vendor_id && dev.device_id == device_id {
                return Some(dev);
            }
        }
        i += 1;
    }
    None
}

/// Find a PCI device by class code
#[no_mangle]
pub unsafe fn pci_find_device_by_class(class_code: u8, subclass: u8) -> Option<&'static PciDevice> {
    let mut i = 0usize;
    while i < PCI_DEVICE_COUNT && i < MAX_PCI_DEVICES {
        if let Some(ref dev) = *PCI_DEVICES.as_ptr().add(i) {
            if dev.class_code == class_code && dev.subclass == subclass {
                return Some(dev);
            }
        }
        i += 1;
    }
    None
}

// ============================================================================
// INITIALIZATION
// ============================================================================

/// Initialize the PCI subsystem
#[no_mangle]
pub unsafe fn pci_init() -> usize {
    pci_enumerate()
}

// ============================================================================
// RE-EXPORTS FROM MSI MODULE
// ============================================================================

pub use msi::*;
