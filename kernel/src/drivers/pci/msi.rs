//! Nafuraito Kernel - MSI and MSI-X Support Module
//!
//! This module provides comprehensive support for Message Signaled Interrupts (MSI)
//! and Extended Message Signaled Interrupts (MSI-X) for PCI devices.
//!
//! MSI and MSI-X are interrupt mechanisms that replace traditional pin-based
//! interrupts with in-band messages, offering several advantages:
//! - More efficient interrupt delivery
//! - Per-vector masking
//! - Multiple interrupt vectors per device
//! - Better multi-core scalability
//!
//! ## MSI Architecture
//! MSI uses a message address and message data to signal interrupts:
//! - Message Address: Target CPU's APIC address
//! - Message Data: Interrupt vector and delivery mode
//!
//! ## MSI-X Architecture
//! MSI-X extends MSI with:
//! - Up to 2048 interrupt vectors per device
//! - Per-vector address and data
//! - Per-vector masking via a dedicated table
//! - Pending Bit Array (PBA) for tracking pending interrupts

#![allow(unused)]

use super::{
    pci_config_read8, pci_config_read16, pci_config_read32,
    pci_config_write8, pci_config_write16, pci_config_write32,
    pci_read_bar, read_volatile_u32, read_volatile_u64,
    write_volatile_u32, write_volatile_u64,
    PciAddress, PciDevice, Bar, BarType,
    PCI_CAP_ID_MSI, PCI_CAP_ID_MSIX, PCI_COMMAND, PCI_COMMAND_INTERRUPT_DISABLE,
};

// ============================================================================
// MSI CONSTANTS
// ============================================================================

/// MSI Capability Register Offsets (relative to capability start)
const MSI_CAP_ID: u8 = 0x00;
const MSI_CONTROL: u8 = 0x02;
const MSI_MESSAGE_ADDRESS_LOW: u8 = 0x04;
const MSI_MESSAGE_ADDRESS_HIGH: u8 = 0x08;  // Only for 64-bit capable
const MSI_MESSAGE_DATA_32: u8 = 0x08;       // For 32-bit addressing
const MSI_MESSAGE_DATA_64: u8 = 0x0C;       // For 64-bit addressing
const MSI_MASK_BITS_32: u8 = 0x0C;          // For 32-bit with per-vector masking
const MSI_MASK_BITS_64: u8 = 0x10;          // For 64-bit with per-vector masking
const MSI_PENDING_BITS_32: u8 = 0x10;       // For 32-bit with per-vector masking
const MSI_PENDING_BITS_64: u8 = 0x14;       // For 64-bit with per-vector masking

/// MSI Control Register bits
const MSI_CONTROL_ENABLE: u16 = 1 << 0;
const MSI_CONTROL_MMC_MASK: u16 = 0b111 << 1;    // Multiple Message Capable
const MSI_CONTROL_MMC_SHIFT: u16 = 1;
const MSI_CONTROL_MME_MASK: u16 = 0b111 << 4;    // Multiple Message Enable
const MSI_CONTROL_MME_SHIFT: u16 = 4;
const MSI_CONTROL_64BIT: u16 = 1 << 7;           // 64-bit Address Capable
const MSI_CONTROL_PER_VECTOR_MASK: u16 = 1 << 8; // Per-Vector Masking Capable

/// MSI-X Capability Register Offsets (relative to capability start)
const MSIX_CAP_ID: u8 = 0x00;
const MSIX_CONTROL: u8 = 0x02;
const MSIX_TABLE_OFFSET: u8 = 0x04;
const MSIX_PBA_OFFSET: u8 = 0x08;

/// MSI-X Control Register bits
const MSIX_CONTROL_TABLE_SIZE_MASK: u16 = 0x07FF;  // Bits 0-10
const MSIX_CONTROL_FUNC_MASK: u16 = 1 << 14;       // Function Mask
const MSIX_CONTROL_ENABLE: u16 = 1 << 15;          // MSI-X Enable

/// MSI-X Table Entry bits
const MSIX_ENTRY_VECTOR_CTRL_MASK: u32 = 1 << 0;   // Vector masked

/// MSI-X Table/PBA offset mask
const MSIX_TABLE_BIR_MASK: u32 = 0x07;             // BAR Indicator Register
const MSIX_TABLE_OFFSET_MASK: u32 = !0x07;         // Offset (8-byte aligned)

// ============================================================================
// LOCAL APIC MESSAGE FORMAT (for x86/x86-64)
// ============================================================================

/// Base address for MSI messages on x86 (writes to Local APIC)
const MSI_ADDRESS_BASE: u32 = 0xFEE0_0000;

/// MSI Address Register bits (x86)
const MSI_ADDRESS_DEST_ID_SHIFT: u32 = 12;         // Destination APIC ID (bits 12-19)
const MSI_ADDRESS_REDIRECT_HINT: u32 = 1 << 3;     // Redirection Hint
const MSI_ADDRESS_DEST_MODE_LOGICAL: u32 = 1 << 2; // Logical destination mode

/// MSI Data Register bits (x86)
const MSI_DATA_VECTOR_MASK: u16 = 0xFF;            // Vector (bits 0-7)
const MSI_DATA_DELIVERY_MODE_SHIFT: u16 = 8;       // Delivery mode (bits 8-10)
const MSI_DATA_DELIVERY_FIXED: u16 = 0b000 << 8;   // Fixed delivery
const MSI_DATA_DELIVERY_LOWEST: u16 = 0b001 << 8;  // Lowest priority
const MSI_DATA_DELIVERY_SMI: u16 = 0b010 << 8;     // SMI
const MSI_DATA_DELIVERY_NMI: u16 = 0b100 << 8;     // NMI
const MSI_DATA_DELIVERY_INIT: u16 = 0b101 << 8;    // INIT
const MSI_DATA_DELIVERY_EXTINT: u16 = 0b111 << 8;  // ExtINT
const MSI_DATA_LEVEL_ASSERT: u16 = 1 << 14;        // Level: Assert
const MSI_DATA_TRIGGER_EDGE: u16 = 0 << 15;        // Trigger mode: Edge
const MSI_DATA_TRIGGER_LEVEL: u16 = 1 << 15;       // Trigger mode: Level

// ============================================================================
// MSI TYPES AND STRUCTURES
// ============================================================================

/// MSI delivery mode
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum MsiDeliveryMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    Smi = 0b010,
    Nmi = 0b100,
    Init = 0b101,
    ExtInt = 0b111,
}

/// MSI destination mode
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MsiDestinationMode {
    Physical = 0,
    Logical = 1,
}

/// MSI trigger mode
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MsiTriggerMode {
    Edge = 0,
    Level = 1,
}

/// MSI configuration for a single interrupt
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct MsiMessage {
    /// Interrupt vector number (0-255 on x86)
    pub vector: u8,
    /// Destination CPU (APIC ID)
    pub dest_id: u8,
    /// Delivery mode
    pub delivery_mode: MsiDeliveryMode,
    /// Destination mode
    pub dest_mode: MsiDestinationMode,
    /// Trigger mode
    pub trigger_mode: MsiTriggerMode,
    /// Redirection hint (for lowest priority delivery)
    pub redirect_hint: bool,
}

impl MsiMessage {
    /// Create a new MSI message with default settings
    pub const fn new(vector: u8, dest_id: u8) -> Self {
        Self {
            vector,
            dest_id,
            delivery_mode: MsiDeliveryMode::Fixed,
            dest_mode: MsiDestinationMode::Physical,
            trigger_mode: MsiTriggerMode::Edge,
            redirect_hint: false,
        }
    }

    /// Create an MSI message for lowest priority delivery
    pub const fn lowest_priority(vector: u8) -> Self {
        Self {
            vector,
            dest_id: 0,  // Not used in lowest priority mode
            delivery_mode: MsiDeliveryMode::LowestPriority,
            dest_mode: MsiDestinationMode::Logical,
            trigger_mode: MsiTriggerMode::Edge,
            redirect_hint: true,
        }
    }

    /// Build the MSI message address (low 32 bits)
    pub const fn address_low(&self) -> u32 {
        let mut addr = MSI_ADDRESS_BASE;
        addr |= (self.dest_id as u32) << MSI_ADDRESS_DEST_ID_SHIFT;
        if self.redirect_hint {
            addr |= MSI_ADDRESS_REDIRECT_HINT;
        }
        if matches!(self.dest_mode, MsiDestinationMode::Logical) {
            addr |= MSI_ADDRESS_DEST_MODE_LOGICAL;
        }
        addr
    }

    /// Build the MSI message address (high 32 bits)
    /// Always 0 for x86/x86-64
    pub const fn address_high(&self) -> u32 {
        0
    }

    /// Build the full 64-bit MSI message address
    pub const fn address(&self) -> u64 {
        self.address_low() as u64
    }

    /// Build the MSI message data
    pub const fn data(&self) -> u16 {
        let mut data: u16 = self.vector as u16;
        data |= (self.delivery_mode as u16) << MSI_DATA_DELIVERY_MODE_SHIFT as u16;
        if matches!(self.trigger_mode, MsiTriggerMode::Level) {
            data |= MSI_DATA_TRIGGER_LEVEL;
            data |= MSI_DATA_LEVEL_ASSERT;
        }
        data
    }
}

/// MSI capability information
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct MsiCapability {
    /// Offset of MSI capability in configuration space
    pub offset: u8,
    /// Number of vectors the device can support (1, 2, 4, 8, 16, or 32)
    pub max_vectors: u8,
    /// 64-bit addressing capable
    pub is_64bit: bool,
    /// Per-vector masking capable
    pub per_vector_mask: bool,
}

impl MsiCapability {
    /// Read MSI capability information from a PCI device
    pub unsafe fn read(addr: &PciAddress, offset: u8) -> Self {
        let control = pci_config_read16(addr, offset + MSI_CONTROL);
        
        let mmc = ((control & MSI_CONTROL_MMC_MASK) >> MSI_CONTROL_MMC_SHIFT) as u8;
        let max_vectors = 1u8 << mmc;
        let is_64bit = (control & MSI_CONTROL_64BIT) != 0;
        let per_vector_mask = (control & MSI_CONTROL_PER_VECTOR_MASK) != 0;

        Self {
            offset,
            max_vectors,
            is_64bit,
            per_vector_mask,
        }
    }

    /// Check if MSI is currently enabled
    pub unsafe fn is_enabled(&self, addr: &PciAddress) -> bool {
        let control = pci_config_read16(addr, self.offset + MSI_CONTROL);
        (control & MSI_CONTROL_ENABLE) != 0
    }

    /// Get the number of allocated vectors
    pub unsafe fn allocated_vectors(&self, addr: &PciAddress) -> u8 {
        let control = pci_config_read16(addr, self.offset + MSI_CONTROL);
        let mme = ((control & MSI_CONTROL_MME_MASK) >> MSI_CONTROL_MME_SHIFT) as u8;
        1u8 << mme
    }
}

/// MSI-X table entry
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct MsixTableEntry {
    /// Message address (low 32 bits)
    pub address_low: u32,
    /// Message address (high 32 bits)
    pub address_high: u32,
    /// Message data
    pub data: u32,
    /// Vector control (bit 0 = masked)
    pub vector_ctrl: u32,
}

impl MsixTableEntry {
    /// Create a table entry from an MSI message
    pub const fn from_message(msg: &MsiMessage) -> Self {
        Self {
            address_low: msg.address_low(),
            address_high: msg.address_high(),
            data: msg.data() as u32,
            vector_ctrl: 0, // Unmasked
        }
    }

    /// Create a masked table entry from an MSI message
    pub const fn from_message_masked(msg: &MsiMessage) -> Self {
        Self {
            address_low: msg.address_low(),
            address_high: msg.address_high(),
            data: msg.data() as u32,
            vector_ctrl: 1, // Masked
        }
    }

    /// Check if this entry is masked
    pub const fn is_masked(&self) -> bool {
        (self.vector_ctrl & MSIX_ENTRY_VECTOR_CTRL_MASK) != 0
    }
}

/// MSI-X capability information
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct MsixCapability {
    /// Offset of MSI-X capability in configuration space
    pub offset: u8,
    /// Number of table entries (1 to 2048)
    pub table_size: u16,
    /// BAR index for MSI-X table
    pub table_bar: u8,
    /// Offset within BAR for MSI-X table
    pub table_offset: u32,
    /// BAR index for PBA
    pub pba_bar: u8,
    /// Offset within BAR for PBA
    pub pba_offset: u32,
}

impl MsixCapability {
    /// Read MSI-X capability information from a PCI device
    pub unsafe fn read(addr: &PciAddress, offset: u8) -> Self {
        let control = pci_config_read16(addr, offset + MSIX_CONTROL);
        let table_size = (control & MSIX_CONTROL_TABLE_SIZE_MASK) + 1;

        let table_bir = pci_config_read32(addr, offset + MSIX_TABLE_OFFSET);
        let table_bar = (table_bir & MSIX_TABLE_BIR_MASK) as u8;
        let table_offset = table_bir & MSIX_TABLE_OFFSET_MASK;

        let pba_bir = pci_config_read32(addr, offset + MSIX_PBA_OFFSET);
        let pba_bar = (pba_bir & MSIX_TABLE_BIR_MASK) as u8;
        let pba_offset = pba_bir & MSIX_TABLE_OFFSET_MASK;

        Self {
            offset,
            table_size,
            table_bar,
            table_offset,
            pba_bar,
            pba_offset,
        }
    }

    /// Check if MSI-X is currently enabled
    pub unsafe fn is_enabled(&self, addr: &PciAddress) -> bool {
        let control = pci_config_read16(addr, self.offset + MSIX_CONTROL);
        (control & MSIX_CONTROL_ENABLE) != 0
    }

    /// Check if the function mask is set
    pub unsafe fn is_function_masked(&self, addr: &PciAddress) -> bool {
        let control = pci_config_read16(addr, self.offset + MSIX_CONTROL);
        (control & MSIX_CONTROL_FUNC_MASK) != 0
    }
}

// ============================================================================
// MSI OPERATIONS
// ============================================================================

/// Enable MSI for a PCI device with a single vector
#[no_mangle]
pub unsafe fn msi_enable_single(device: &PciDevice, message: &MsiMessage) -> bool {
    if device.msi_cap_offset == 0 {
        return false;
    }

    let offset = device.msi_cap_offset;
    let addr = &device.address;

    // Read current control register
    let mut control = pci_config_read16(addr, offset + MSI_CONTROL);
    let is_64bit = (control & MSI_CONTROL_64BIT) != 0;

    // Disable MSI while configuring
    control &= !MSI_CONTROL_ENABLE;
    pci_config_write16(addr, offset + MSI_CONTROL, control);

    // Disable legacy interrupts
    let cmd = pci_config_read16(addr, PCI_COMMAND);
    pci_config_write16(addr, PCI_COMMAND, cmd | PCI_COMMAND_INTERRUPT_DISABLE);

    // Write message address
    pci_config_write32(addr, offset + MSI_MESSAGE_ADDRESS_LOW, message.address_low());

    // Write message data
    if is_64bit {
        pci_config_write32(addr, offset + MSI_MESSAGE_ADDRESS_HIGH, message.address_high());
        pci_config_write16(addr, offset + MSI_MESSAGE_DATA_64, message.data());
    } else {
        pci_config_write16(addr, offset + MSI_MESSAGE_DATA_32, message.data());
    }

    // Set number of allocated vectors to 1 (MME = 0)
    control &= !MSI_CONTROL_MME_MASK;

    // Enable MSI
    control |= MSI_CONTROL_ENABLE;
    pci_config_write16(addr, offset + MSI_CONTROL, control);

    true
}

/// Enable MSI for a PCI device with multiple vectors
/// Returns the actual number of vectors allocated (power of 2)
#[no_mangle]
pub unsafe fn msi_enable_multi(
    device: &PciDevice,
    base_message: &MsiMessage,
    requested_vectors: u8
) -> u8 {
    if device.msi_cap_offset == 0 {
        return 0;
    }

    let offset = device.msi_cap_offset;
    let addr = &device.address;

    // Read capability info
    let cap = MsiCapability::read(addr, offset);
    
    // Calculate vectors to allocate (must be power of 2, <= max supported)
    let vectors = core::cmp::min(requested_vectors, cap.max_vectors);
    let vectors = vectors.next_power_of_two().min(32) as u8;
    let mme = vectors.trailing_zeros() as u8;

    // Read current control register
    let mut control = pci_config_read16(addr, offset + MSI_CONTROL);

    // Disable MSI while configuring
    control &= !MSI_CONTROL_ENABLE;
    pci_config_write16(addr, offset + MSI_CONTROL, control);

    // Disable legacy interrupts
    let cmd = pci_config_read16(addr, PCI_COMMAND);
    pci_config_write16(addr, PCI_COMMAND, cmd | PCI_COMMAND_INTERRUPT_DISABLE);

    // Write message address
    pci_config_write32(addr, offset + MSI_MESSAGE_ADDRESS_LOW, base_message.address_low());

    // Write message data (base vector; device will use base..base+vectors-1)
    if cap.is_64bit {
        pci_config_write32(addr, offset + MSI_MESSAGE_ADDRESS_HIGH, base_message.address_high());
        pci_config_write16(addr, offset + MSI_MESSAGE_DATA_64, base_message.data());
    } else {
        pci_config_write16(addr, offset + MSI_MESSAGE_DATA_32, base_message.data());
    }

    // Set number of allocated vectors
    control &= !MSI_CONTROL_MME_MASK;
    control |= (mme as u16) << MSI_CONTROL_MME_SHIFT;

    // Enable MSI
    control |= MSI_CONTROL_ENABLE;
    pci_config_write16(addr, offset + MSI_CONTROL, control);

    vectors
}

/// Disable MSI for a PCI device
#[no_mangle]
pub unsafe fn msi_disable(device: &PciDevice) -> bool {
    if device.msi_cap_offset == 0 {
        return false;
    }

    let offset = device.msi_cap_offset;
    let addr = &device.address;

    let mut control = pci_config_read16(addr, offset + MSI_CONTROL);
    control &= !MSI_CONTROL_ENABLE;
    pci_config_write16(addr, offset + MSI_CONTROL, control);

    // Re-enable legacy interrupts
    let cmd = pci_config_read16(addr, PCI_COMMAND);
    pci_config_write16(addr, PCI_COMMAND, cmd & !PCI_COMMAND_INTERRUPT_DISABLE);

    true
}

/// Mask a specific MSI vector (requires per-vector masking support)
#[no_mangle]
pub unsafe fn msi_mask_vector(device: &PciDevice, vector: u8) -> bool {
    if device.msi_cap_offset == 0 {
        return false;
    }

    let offset = device.msi_cap_offset;
    let addr = &device.address;
    let cap = MsiCapability::read(addr, offset);

    if !cap.per_vector_mask {
        return false;
    }

    let mask_offset = if cap.is_64bit {
        MSI_MASK_BITS_64
    } else {
        MSI_MASK_BITS_32
    };

    let mask = pci_config_read32(addr, offset + mask_offset);
    pci_config_write32(addr, offset + mask_offset, mask | (1 << vector));

    true
}

/// Unmask a specific MSI vector (requires per-vector masking support)
#[no_mangle]
pub unsafe fn msi_unmask_vector(device: &PciDevice, vector: u8) -> bool {
    if device.msi_cap_offset == 0 {
        return false;
    }

    let offset = device.msi_cap_offset;
    let addr = &device.address;
    let cap = MsiCapability::read(addr, offset);

    if !cap.per_vector_mask {
        return false;
    }

    let mask_offset = if cap.is_64bit {
        MSI_MASK_BITS_64
    } else {
        MSI_MASK_BITS_32
    };

    let mask = pci_config_read32(addr, offset + mask_offset);
    pci_config_write32(addr, offset + mask_offset, mask & !(1 << vector));

    true
}

/// Check if a vector has a pending interrupt
#[no_mangle]
pub unsafe fn msi_is_pending(device: &PciDevice, vector: u8) -> bool {
    if device.msi_cap_offset == 0 {
        return false;
    }

    let offset = device.msi_cap_offset;
    let addr = &device.address;
    let cap = MsiCapability::read(addr, offset);

    if !cap.per_vector_mask {
        return false;
    }

    let pending_offset = if cap.is_64bit {
        MSI_PENDING_BITS_64
    } else {
        MSI_PENDING_BITS_32
    };

    let pending = pci_config_read32(addr, offset + pending_offset);
    (pending & (1 << vector)) != 0
}

/// Get MSI capability information for a device
#[no_mangle]
pub unsafe fn msi_get_capability(device: &PciDevice) -> Option<MsiCapability> {
    if device.msi_cap_offset == 0 {
        return None;
    }
    Some(MsiCapability::read(&device.address, device.msi_cap_offset))
}

// ============================================================================
// MSI-X OPERATIONS
// ============================================================================

/// MSI-X controller for managing MSI-X table access
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct MsixController {
    /// MSI-X capability information
    pub capability: MsixCapability,
    /// Table base address (physical/virtual)
    pub table_addr: u64,
    /// PBA base address (physical/virtual)
    pub pba_addr: u64,
}

impl MsixController {
    /// Calculate required addresses from device BAR
    pub unsafe fn new(device: &PciDevice) -> Option<Self> {
        if device.msix_cap_offset == 0 {
            return None;
        }

        let cap = MsixCapability::read(&device.address, device.msix_cap_offset);
        
        // Read the BAR for the table
        let table_bar = pci_read_bar(&device.address, cap.table_bar);
        if matches!(table_bar.bar_type, BarType::None | BarType::IoSpace) {
            return None;
        }

        // Read the BAR for the PBA
        let pba_bar = if cap.pba_bar == cap.table_bar {
            table_bar
        } else {
            let bar = pci_read_bar(&device.address, cap.pba_bar);
            if matches!(bar.bar_type, BarType::None | BarType::IoSpace) {
                return None;
            }
            bar
        };

        let table_addr = table_bar.base_address + cap.table_offset as u64;
        let pba_addr = pba_bar.base_address + cap.pba_offset as u64;

        Some(Self {
            capability: cap,
            table_addr,
            pba_addr,
        })
    }

    /// Get the address of a table entry
    #[inline]
    pub const fn entry_addr(&self, index: u16) -> u64 {
        self.table_addr + (index as u64 * 16) // Each entry is 16 bytes
    }

    /// Get the address of a PBA qword
    #[inline]
    pub const fn pba_qword_addr(&self, index: u16) -> u64 {
        self.pba_addr + (index as u64 / 64) * 8
    }
}

/// Enable MSI-X for a PCI device
#[no_mangle]
pub unsafe fn msix_enable(device: &PciDevice) -> bool {
    if device.msix_cap_offset == 0 {
        return false;
    }

    let offset = device.msix_cap_offset;
    let addr = &device.address;

    // Disable legacy interrupts
    let cmd = pci_config_read16(addr, PCI_COMMAND);
    pci_config_write16(addr, PCI_COMMAND, cmd | PCI_COMMAND_INTERRUPT_DISABLE);

    // Enable MSI-X with function mask set (mask all vectors)
    let mut control = pci_config_read16(addr, offset + MSIX_CONTROL);
    control |= MSIX_CONTROL_ENABLE | MSIX_CONTROL_FUNC_MASK;
    pci_config_write16(addr, offset + MSIX_CONTROL, control);

    true
}

/// Disable MSI-X for a PCI device
#[no_mangle]
pub unsafe fn msix_disable(device: &PciDevice) -> bool {
    if device.msix_cap_offset == 0 {
        return false;
    }

    let offset = device.msix_cap_offset;
    let addr = &device.address;

    let mut control = pci_config_read16(addr, offset + MSIX_CONTROL);
    control &= !MSIX_CONTROL_ENABLE;
    pci_config_write16(addr, offset + MSIX_CONTROL, control);

    // Re-enable legacy interrupts
    let cmd = pci_config_read16(addr, PCI_COMMAND);
    pci_config_write16(addr, PCI_COMMAND, cmd & !PCI_COMMAND_INTERRUPT_DISABLE);

    true
}

/// Set the global function mask (masks all vectors)
#[no_mangle]
pub unsafe fn msix_set_function_mask(device: &PciDevice, masked: bool) -> bool {
    if device.msix_cap_offset == 0 {
        return false;
    }

    let offset = device.msix_cap_offset;
    let addr = &device.address;

    let mut control = pci_config_read16(addr, offset + MSIX_CONTROL);
    if masked {
        control |= MSIX_CONTROL_FUNC_MASK;
    } else {
        control &= !MSIX_CONTROL_FUNC_MASK;
    }
    pci_config_write16(addr, offset + MSIX_CONTROL, control);

    true
}

/// Write an MSI-X table entry
#[no_mangle]
pub unsafe fn msix_write_entry(
    controller: &MsixController,
    index: u16,
    entry: &MsixTableEntry
) -> bool {
    if index >= controller.capability.table_size {
        return false;
    }

    let entry_addr = controller.entry_addr(index);
    
    // Write entry (address low, address high, data, vector control)
    write_volatile_u32(entry_addr, entry.address_low);
    write_volatile_u32(entry_addr + 4, entry.address_high);
    write_volatile_u32(entry_addr + 8, entry.data);
    write_volatile_u32(entry_addr + 12, entry.vector_ctrl);

    true
}

/// Read an MSI-X table entry
#[no_mangle]
pub unsafe fn msix_read_entry(controller: &MsixController, index: u16) -> Option<MsixTableEntry> {
    if index >= controller.capability.table_size {
        return None;
    }

    let entry_addr = controller.entry_addr(index);

    Some(MsixTableEntry {
        address_low: read_volatile_u32(entry_addr),
        address_high: read_volatile_u32(entry_addr + 4),
        data: read_volatile_u32(entry_addr + 8),
        vector_ctrl: read_volatile_u32(entry_addr + 12),
    })
}

/// Configure an MSI-X vector with an MSI message
#[no_mangle]
pub unsafe fn msix_configure_vector(
    controller: &MsixController,
    index: u16,
    message: &MsiMessage,
    masked: bool
) -> bool {
    let entry = if masked {
        MsixTableEntry::from_message_masked(message)
    } else {
        MsixTableEntry::from_message(message)
    };
    
    msix_write_entry(controller, index, &entry)
}

/// Mask a specific MSI-X vector
#[no_mangle]
pub unsafe fn msix_mask_vector(controller: &MsixController, index: u16) -> bool {
    if index >= controller.capability.table_size {
        return false;
    }

    let vector_ctrl_addr = controller.entry_addr(index) + 12;
    let ctrl = read_volatile_u32(vector_ctrl_addr);
    write_volatile_u32(vector_ctrl_addr, ctrl | MSIX_ENTRY_VECTOR_CTRL_MASK);

    true
}

/// Unmask a specific MSI-X vector
#[no_mangle]
pub unsafe fn msix_unmask_vector(controller: &MsixController, index: u16) -> bool {
    if index >= controller.capability.table_size {
        return false;
    }

    let vector_ctrl_addr = controller.entry_addr(index) + 12;
    let ctrl = read_volatile_u32(vector_ctrl_addr);
    write_volatile_u32(vector_ctrl_addr, ctrl & !MSIX_ENTRY_VECTOR_CTRL_MASK);

    true
}

/// Check if a specific MSI-X vector is masked
#[no_mangle]
pub unsafe fn msix_is_vector_masked(controller: &MsixController, index: u16) -> bool {
    if index >= controller.capability.table_size {
        return true;
    }

    let vector_ctrl_addr = controller.entry_addr(index) + 12;
    let ctrl = read_volatile_u32(vector_ctrl_addr);
    (ctrl & MSIX_ENTRY_VECTOR_CTRL_MASK) != 0
}

/// Check if a specific MSI-X vector has a pending interrupt
#[no_mangle]
pub unsafe fn msix_is_pending(controller: &MsixController, index: u16) -> bool {
    if index >= controller.capability.table_size {
        return false;
    }

    let qword_index = index / 64;
    let bit_index = index % 64;
    
    let pba_addr = controller.pba_qword_addr(qword_index);
    let pending = read_volatile_u64(pba_addr);
    
    (pending & (1u64 << bit_index)) != 0
}

/// Mask all MSI-X vectors
#[no_mangle]
pub unsafe fn msix_mask_all(controller: &MsixController) {
    for i in 0..controller.capability.table_size {
        msix_mask_vector(controller, i);
    }
}

/// Unmask all MSI-X vectors that are configured
#[no_mangle]
pub unsafe fn msix_unmask_all(controller: &MsixController) {
    for i in 0..controller.capability.table_size {
        msix_unmask_vector(controller, i);
    }
}

/// Get MSI-X capability information for a device
#[no_mangle]
pub unsafe fn msix_get_capability(device: &PciDevice) -> Option<MsixCapability> {
    if device.msix_cap_offset == 0 {
        return None;
    }
    Some(MsixCapability::read(&device.address, device.msix_cap_offset))
}

/// Get the table size for MSI-X
#[no_mangle]
pub unsafe fn msix_get_table_size(device: &PciDevice) -> u16 {
    if device.msix_cap_offset == 0 {
        return 0;
    }
    
    let control = pci_config_read16(&device.address, device.msix_cap_offset + MSIX_CONTROL);
    (control & MSIX_CONTROL_TABLE_SIZE_MASK) + 1
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Check if a device supports MSI or MSI-X
#[no_mangle]
pub unsafe fn device_supports_msi_or_msix(device: &PciDevice) -> bool {
    device.supports_msi() || device.supports_msix()
}

/// Preferred way to configure interrupts: use MSI-X if available, else MSI
/// Returns the number of vectors successfully configured
#[no_mangle]
pub unsafe fn configure_device_interrupts(
    device: &PciDevice,
    messages: &[MsiMessage],
    msix_controller: Option<&MsixController>
) -> u16 {
    // Prefer MSI-X if available
    if device.supports_msix() {
        if let Some(controller) = msix_controller {
            msix_enable(device);
            
            let count = core::cmp::min(messages.len(), controller.capability.table_size as usize);
            for (i, msg) in messages.iter().take(count).enumerate() {
                msix_configure_vector(controller, i as u16, msg, false);
            }
            
            msix_set_function_mask(device, false);
            return count as u16;
        }
    }
    
    // Fall back to MSI
    if device.supports_msi() && !messages.is_empty() {
        if messages.len() == 1 {
            if msi_enable_single(device, &messages[0]) {
                return 1;
            }
        } else {
            let allocated = msi_enable_multi(device, &messages[0], messages.len() as u8);
            if allocated > 0 {
                return allocated as u16;
            }
        }
    }

    0
}

// ============================================================================
// INTERRUPT VECTOR ALLOCATION HELPERS
// ============================================================================

/// Simple vector allocator state (for example purposes)
static mut NEXT_VECTOR: u8 = 48; // Start after exception vectors

/// Allocate interrupt vectors starting from a base
/// Returns the base vector number
#[no_mangle]
pub unsafe fn allocate_vectors(count: u8) -> Option<u8> {
    if NEXT_VECTOR.checked_add(count).is_none() || NEXT_VECTOR + count > 224 {
        return None;
    }
    
    let base = NEXT_VECTOR;
    NEXT_VECTOR = NEXT_VECTOR.saturating_add(count);
    Some(base)
}

/// Create MSI messages for a range of vectors targeting a specific CPU
#[no_mangle]
pub unsafe fn create_msi_messages(base_vector: u8, count: u8, target_cpu: u8) -> [MsiMessage; 32] {
    let mut messages = [MsiMessage::new(0, 0); 32];
    
    for i in 0..core::cmp::min(count as usize, 32) {
        messages[i] = MsiMessage::new(base_vector.wrapping_add(i as u8), target_cpu);
    }
    
    messages
}

/// Create MSI messages with lowest priority delivery mode
/// This allows the APIC to choose the CPU with lowest task priority
#[no_mangle]
pub unsafe fn create_msi_messages_lowest_priority(base_vector: u8, count: u8) -> [MsiMessage; 32] {
    let mut messages = [MsiMessage::new(0, 0); 32];
    
    for i in 0..core::cmp::min(count as usize, 32) {
        messages[i] = MsiMessage::lowest_priority(base_vector.wrapping_add(i as u8));
    }
    
    messages
}
