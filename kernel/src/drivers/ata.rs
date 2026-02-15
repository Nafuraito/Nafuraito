// ATA PIO (LBA28) helper module for ext4 block I/O.
//
// This mirrors the bootloader's ATA PIO path and keeps disk access in one place.
// It is intentionally simple until AHCI/NVMe and a full block layer exist.

#![allow(dead_code)]

/// Primary ATA I/O port base addresses.
pub const ATA_PRIMARY_DATA: u16 = 0x1F0;
/// Primary ATA error register (read) or features register (write).
pub const ATA_PRIMARY_ERROR: u16 = 0x1F1;
/// Sector count register for ATA PIO operations.
pub const ATA_PRIMARY_SECCOUNT: u16 = 0x1F2;
/// LBA low byte register.
pub const ATA_PRIMARY_LBA_LO: u16 = 0x1F3;
/// LBA mid byte register.
pub const ATA_PRIMARY_LBA_MID: u16 = 0x1F4;
/// LBA high byte register.
pub const ATA_PRIMARY_LBA_HI: u16 = 0x1F5;
/// Drive/head register (includes LBA mode and high LBA bits).
pub const ATA_PRIMARY_DRIVE: u16 = 0x1F6;
/// Status register (read) or command register (write).
pub const ATA_PRIMARY_STATUS: u16 = 0x1F7;
/// Command register (write).
pub const ATA_PRIMARY_CMD: u16 = 0x1F7;

/// ATA status register bits.
pub const ATA_STATUS_ERR: u8 = 0x01;
/// ATA DRQ (data request) bit.
pub const ATA_STATUS_DRQ: u8 = 0x08;
/// ATA RDY bit.
pub const ATA_STATUS_RDY: u8 = 0x40;
/// ATA BSY bit.
pub const ATA_STATUS_BSY: u8 = 0x80;

/// ATA read sectors command (PIO).
pub const ATA_CMD_READ_SECTORS: u8 = 0x20;
/// ATA read sectors command for LBA48 (PIO).
pub const ATA_CMD_READ_SECTORS_EXT: u8 = 0x24;
/// ATA write sectors command (PIO).
pub const ATA_CMD_WRITE_SECTORS: u8 = 0x30;
/// ATA write sectors command for LBA48 (PIO).
pub const ATA_CMD_WRITE_SECTORS_EXT: u8 = 0x34;
/// ATA flush cache command (legacy).
pub const ATA_CMD_FLUSH_CACHE: u8 = 0xE7;
/// ATA flush cache command for LBA48.
pub const ATA_CMD_FLUSH_CACHE_EXT: u8 = 0xEA;

/// Default ext4 partition offset in 512-byte LBAs.
///
/// TODO: Derive this from the bootloader or a partition table parser.
pub const EXT4_PARTITION_LBA: u64 = 2048;

/// Simple device descriptor for ext4 block callbacks.
#[repr(C)]
pub struct Ext4AtaDevice {
    /// Partition offset in 512-byte sectors.
    pub partition_lba: u64,
}

/// Global ext4 ATA device descriptor.
pub static mut EXT4_ATA_DEVICE: Ext4AtaDevice = Ext4AtaDevice {
    partition_lba: EXT4_PARTITION_LBA,
};

/// Read an 8-bit value from an I/O port.
#[inline(always)]
unsafe fn inb(port: u16) -> u8 {
    // Read the I/O port into AL.
    let value: u8;
    // SAFETY: Port I/O is a required kernel primitive.
    core::arch::asm!("in al, dx", in("dx") port, out("al") value, options(nomem, nostack));
    value
}

/// Write an 8-bit value to an I/O port.
#[inline(always)]
unsafe fn outb(port: u16, value: u8) {
    // Write AL to the I/O port.
    // SAFETY: Port I/O is a required kernel primitive.
    core::arch::asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
}

/// Read `count` 16-bit words from an I/O port into memory.
#[inline(always)]
unsafe fn insw(port: u16, addr: *mut u16, count: usize) {
    // Use REP INSW to transfer words into memory.
    // SAFETY: Caller ensures `addr` is valid for `count` words.
    core::arch::asm!(
        "rep insw",
        in("dx") port,
        inout("rdi") addr => _,
        inout("rcx") count => _,
        options(nostack, preserves_flags)
    );
}

/// Write `count` 16-bit words from memory to an I/O port.
#[inline(always)]
unsafe fn outsw(port: u16, addr: *const u16, count: usize) {
    // Use REP OUTSW to transfer words from memory.
    // SAFETY: Caller ensures `addr` is valid for `count` words.
    core::arch::asm!(
        "rep outsw",
        in("dx") port,
        inout("rsi") addr => _,
        inout("rcx") count => _,
        options(nostack, preserves_flags)
    );
}

/// Wait for the ATA drive to become ready for commands.
fn ata_wait_ready() -> bool {
    // Use a small timeout to avoid hanging the kernel if the drive is absent.
    let mut timeout = 100000u32;

    while timeout > 0 {
        timeout -= 1;

        // Poll the status register until RDY is set.
        let status = unsafe { inb(ATA_PRIMARY_STATUS) };
        // ERR indicates the command failed.
        if (status & ATA_STATUS_ERR) != 0 {
            return false;
        }
        // BSY means the device is still processing.
        if (status & ATA_STATUS_BSY) != 0 {
            continue;
        }
        // RDY means we can issue new commands.
        if (status & ATA_STATUS_RDY) != 0 {
            return true;
        }
    }

    false
}

/// Wait for the ATA drive to request data transfer (DRQ).
fn ata_wait_drq() -> bool {
    // Use a small timeout so missing drives do not hang the kernel.
    let mut timeout = 100000u32;

    while timeout > 0 {
        timeout -= 1;

        // Poll until DRQ is set and BSY is clear.
        let status = unsafe { inb(ATA_PRIMARY_STATUS) };
        // BSY must clear before DRQ can be trusted.
        if (status & ATA_STATUS_BSY) != 0 {
            continue;
        }
        // ERR indicates the request failed.
        if (status & ATA_STATUS_ERR) != 0 {
            return false;
        }
        // DRQ indicates the data register is ready.
        if (status & ATA_STATUS_DRQ) != 0 {
            return true;
        }
    }

    false
}

/// Read a sequence of 512-byte sectors using LBA28 PIO.
fn ata_read_sectors_lba28(dest: *mut u8, lba: u64, count: u32) -> bool {
    // A zero-length request is trivially successful.
    if count == 0 {
        return true;
    }

    // LBA28 only supports 28-bit addresses.
    if lba > 0x0FFF_FFFF {
        return false;
    }

    // Track the current destination pointer and LBA.
    let mut buffer = dest;
    let mut current_lba = lba;

    for _ in 0..count {
        // Wait until the device is ready to accept a command.
        if !ata_wait_ready() {
            return false;
        }

        // Select drive (LBA mode, master) and set high LBA bits.
        let drive_select = 0xE0 | (((current_lba >> 24) & 0x0F) as u8);
        unsafe { outb(ATA_PRIMARY_DRIVE, drive_select) };

        // Program the sector count and LBA registers.
        unsafe {
            outb(ATA_PRIMARY_SECCOUNT, 1);
            outb(ATA_PRIMARY_LBA_LO, (current_lba & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_MID, ((current_lba >> 8) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_HI, ((current_lba >> 16) & 0xFF) as u8);
            outb(ATA_PRIMARY_CMD, ATA_CMD_READ_SECTORS);
        }

        // Wait until the drive presents data.
        if !ata_wait_drq() {
            return false;
        }

        // Read one sector (256 words = 512 bytes) into the caller buffer.
        unsafe {
            insw(ATA_PRIMARY_DATA, buffer as *mut u16, 256);
        }

        // Advance to the next sector and buffer location.
        buffer = unsafe { buffer.add(512) };
        current_lba += 1;
    }

    true
}

/// Read a sequence of 512-byte sectors using LBA48 PIO.
fn ata_read_sectors_lba48(dest: *mut u8, lba: u64, count: u32) -> bool {
    // A zero-length request is trivially successful.
    if count == 0 {
        return true;
    }

    // LBA48 supports up to 48-bit addresses.
    if lba > 0x0000_FFFF_FFFF_FFFF {
        return false;
    }

    // LBA48 sector count is 16-bit; limit the request size.
    if count > 0xFFFF {
        return false;
    }

    // Track the current destination pointer and LBA.
    let mut buffer = dest;
    let mut current_lba = lba;

    for _ in 0..count {
        // Wait until the device is ready to accept a command.
        if !ata_wait_ready() {
            return false;
        }

        // Select drive (LBA mode, master) without high LBA bits in LBA48.
        let drive_select = 0xE0;
        unsafe { outb(ATA_PRIMARY_DRIVE, drive_select) };

        // Program the high-order bytes first (HOB), then the low bytes.
        unsafe {
            // Write the upper sector count and LBA bytes.
            outb(ATA_PRIMARY_SECCOUNT, 0);
            outb(ATA_PRIMARY_LBA_LO, ((current_lba >> 24) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_MID, ((current_lba >> 32) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_HI, ((current_lba >> 40) & 0xFF) as u8);
            // Write the lower sector count and LBA bytes.
            outb(ATA_PRIMARY_SECCOUNT, 1);
            outb(ATA_PRIMARY_LBA_LO, (current_lba & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_MID, ((current_lba >> 8) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_HI, ((current_lba >> 16) & 0xFF) as u8);
            outb(ATA_PRIMARY_CMD, ATA_CMD_READ_SECTORS_EXT);
        }

        // Wait until the drive presents data.
        if !ata_wait_drq() {
            return false;
        }

        // Read one sector (256 words = 512 bytes) into the caller buffer.
        unsafe {
            insw(ATA_PRIMARY_DATA, buffer as *mut u16, 256);
        }

        // Advance to the next sector and buffer location.
        buffer = unsafe { buffer.add(512) };
        current_lba += 1;
    }

    true
}

/// Write a sequence of 512-byte sectors using LBA28 PIO.
fn ata_write_sectors_lba28(src: *const u8, lba: u64, count: u32) -> bool {
    // A zero-length request is trivially successful.
    if count == 0 {
        return true;
    }

    // LBA28 only supports 28-bit addresses.
    if lba > 0x0FFF_FFFF {
        return false;
    }

    // Track the current source pointer and LBA.
    let mut buffer = src;
    let mut current_lba = lba;

    for _ in 0..count {
        // Wait until the device is ready to accept a command.
        if !ata_wait_ready() {
            return false;
        }

        // Select drive (LBA mode, master) and set high LBA bits.
        let drive_select = 0xE0 | (((current_lba >> 24) & 0x0F) as u8);
        unsafe { outb(ATA_PRIMARY_DRIVE, drive_select) };

        // Program the sector count and LBA registers.
        unsafe {
            outb(ATA_PRIMARY_SECCOUNT, 1);
            outb(ATA_PRIMARY_LBA_LO, (current_lba & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_MID, ((current_lba >> 8) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_HI, ((current_lba >> 16) & 0xFF) as u8);
            outb(ATA_PRIMARY_CMD, ATA_CMD_WRITE_SECTORS);
        }

        // Wait until the drive is ready to accept data.
        if !ata_wait_drq() {
            return false;
        }

        // Write one sector (256 words = 512 bytes) from the caller buffer.
        unsafe {
            outsw(ATA_PRIMARY_DATA, buffer as *const u16, 256);
        }

        // Advance to the next sector and buffer location.
        buffer = unsafe { buffer.add(512) };
        current_lba += 1;
    }

    true
}

/// Write a sequence of 512-byte sectors using LBA48 PIO.
fn ata_write_sectors_lba48(src: *const u8, lba: u64, count: u32) -> bool {
    // A zero-length request is trivially successful.
    if count == 0 {
        return true;
    }

    // LBA48 supports up to 48-bit addresses.
    if lba > 0x0000_FFFF_FFFF_FFFF {
        return false;
    }

    // LBA48 sector count is 16-bit; limit the request size.
    if count > 0xFFFF {
        return false;
    }

    // Track the current source pointer and LBA.
    let mut buffer = src;
    let mut current_lba = lba;

    for _ in 0..count {
        // Wait until the device is ready to accept a command.
        if !ata_wait_ready() {
            return false;
        }

        // Select drive (LBA mode, master) without high LBA bits in LBA48.
        let drive_select = 0xE0;
        unsafe { outb(ATA_PRIMARY_DRIVE, drive_select) };

        // Program the high-order bytes first (HOB), then the low bytes.
        unsafe {
            // Write the upper sector count and LBA bytes.
            outb(ATA_PRIMARY_SECCOUNT, 0);
            outb(ATA_PRIMARY_LBA_LO, ((current_lba >> 24) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_MID, ((current_lba >> 32) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_HI, ((current_lba >> 40) & 0xFF) as u8);
            // Write the lower sector count and LBA bytes.
            outb(ATA_PRIMARY_SECCOUNT, 1);
            outb(ATA_PRIMARY_LBA_LO, (current_lba & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_MID, ((current_lba >> 8) & 0xFF) as u8);
            outb(ATA_PRIMARY_LBA_HI, ((current_lba >> 16) & 0xFF) as u8);
            outb(ATA_PRIMARY_CMD, ATA_CMD_WRITE_SECTORS_EXT);
        }

        // Wait until the drive is ready to accept data.
        if !ata_wait_drq() {
            return false;
        }

        // Write one sector (256 words = 512 bytes) from the caller buffer.
        unsafe {
            outsw(ATA_PRIMARY_DATA, buffer as *const u16, 256);
        }

        // Advance to the next sector and buffer location.
        buffer = unsafe { buffer.add(512) };
        current_lba += 1;
    }

    true
}

/// Write a sequence of 512-byte sectors using LBA28 or LBA48 PIO.
fn ata_write_sectors(src: *const u8, lba: u64, count: u32) -> bool {
    // Use LBA48 when the address exceeds 28-bit range.
    if lba > 0x0FFF_FFFF {
        return ata_write_sectors_lba48(src, lba, count);
    }

    // Use LBA28 for smaller addresses.
    ata_write_sectors_lba28(src, lba, count)
}

/// Flush the drive cache using a legacy or LBA48 command.
fn ata_flush_cache(use_lba48: bool) -> bool {
    // Ensure the device is ready before issuing a flush.
    if !ata_wait_ready() {
        return false;
    }

    // Issue the correct flush command based on the LBA mode.
    unsafe {
        if use_lba48 {
            outb(ATA_PRIMARY_CMD, ATA_CMD_FLUSH_CACHE_EXT);
        } else {
            outb(ATA_PRIMARY_CMD, ATA_CMD_FLUSH_CACHE);
        }
    }

    // Wait until the device completes the flush.
    ata_wait_ready()
}

/// Read a sequence of 512-byte sectors using LBA28 or LBA48 PIO.
fn ata_read_sectors(dest: *mut u8, lba: u64, count: u32) -> bool {
    // Use LBA48 when the address exceeds 28-bit range.
    if lba > 0x0FFF_FFFF {
        return ata_read_sectors_lba48(dest, lba, count);
    }

    // Use LBA28 for smaller addresses.
    ata_read_sectors_lba28(dest, lba, count)
}

/// EXT4 read block callback using ATA PIO.
pub extern "C" fn ext4_read_block_ata(
    dev: *mut u8,
    block: u64,
    buffer: *mut u8,
    len: usize,
) -> i32 {
    // Validate inputs early to avoid undefined behavior.
    if dev.is_null() || buffer.is_null() || len == 0 {
        return -1;
    }

    // ext4 passes a block-sized buffer; require 512-byte alignment.
    if (len % 512) != 0 {
        return -1;
    }

    // Interpret the device pointer as our ATA descriptor.
    let device = unsafe { &*(dev as *const Ext4AtaDevice) };
    let sectors = (len / 512) as u32;

    // Compute LBA from partition offset + block index.
    let lba = device.partition_lba + (block * sectors as u64);

    if ata_read_sectors(buffer, lba, sectors) {
        0
    } else {
        -1
    }
}

/// EXT4 write block callback placeholder (PIO writes not wired yet).
///
/// TODO: Implement ATA PIO write support or route through a DMA-capable path.
pub extern "C" fn ext4_write_block_ata(
    _dev: *mut u8,
    _block: u64,
    _buffer: *const u8,
    _len: usize,
) -> i32 {
    // Validate inputs early to avoid undefined behavior.
    if _dev.is_null() || _buffer.is_null() || _len == 0 {
        return -1;
    }

    // ext4 passes a block-sized buffer; require 512-byte alignment.
    if (_len % 512) != 0 {
        return -1;
    }

    // Interpret the device pointer as our ATA descriptor.
    let device = unsafe { &*(_dev as *const Ext4AtaDevice) };
    let sectors = (_len / 512) as u32;

    // Compute LBA from partition offset + block index.
    let lba = device.partition_lba + (_block * sectors as u64);

    if ata_write_sectors(_buffer, lba, sectors) {
        0
    } else {
        -1
    }
}

/// EXT4 sync callback placeholder (no writeback support yet).
///
/// TODO: Implement drive flush (ATA FLUSH CACHE) once writes are supported.
pub extern "C" fn ext4_sync_ata(_dev: *mut u8) -> i32 {
    // Validate inputs early to avoid undefined behavior.
    if _dev.is_null() {
        return -1;
    }

    // TODO: Detect LBA48 capability via IDENTIFY data.
    let use_lba48 = true;

    if ata_flush_cache(use_lba48) {
        0
    } else {
        -1
    }
}
