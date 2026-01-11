// Nafuraito Kernel Loader (C Implementation)
//
// Loads the kernel binary from disk using ATA PIO mode
// and returns the kernel entry point address.

#include "bootloader.h"

//=============================================================================
// ATA PIO Driver Implementation
//=============================================================================

// Wait for ATA drive to be ready
// Returns: 1 if ready, 0 on error/timeout
int ata_wait_ready(void) {
    uint32_t timeout = 100000;
    uint8_t status;
    
    while (timeout--) {
        status = inb(ATA_PRIMARY_STATUS);
        
        // Check for error
        if (status & ATA_STATUS_ERR) {
            return 0;
        }
        
        // Check if busy
        if (status & ATA_STATUS_BSY) {
            continue;
        }
        
        // Check if ready or data request
        if (status & ATA_STATUS_RDY) {
            return 1;
        }
    }
    
    // Timeout
    return 0;
}

// Wait for data request
// Returns: 1 if ready, 0 on error/timeout
int ata_wait_drq(void) {
    uint32_t timeout = 100000;
    uint8_t status;
    
    while (timeout--) {
        status = inb(ATA_PRIMARY_STATUS);
        
        // Check if busy
        if (status & ATA_STATUS_BSY) {
            continue;
        }
        
        // Check for error
        if (status & ATA_STATUS_ERR) {
            return 0;
        }
        
        // Check if data request ready
        if (status & ATA_STATUS_DRQ) {
            return 1;
        }
    }
    
    // Timeout
    return 0;
}

// Read sectors from ATA drive
// dest  = destination address
// lba   = logical block address
// count = number of sectors to read
// Returns: 1 on success, 0 on failure
int ata_read_sectors(void* dest, uint64_t lba, uint32_t count) {
    uint8_t* buffer = (uint8_t*)dest;
    
    for (uint32_t i = 0; i < count; i++) {
        // Wait for drive ready
        if (!ata_wait_ready()) {
            return 0;
        }
        
        // Select drive (LBA mode, master)
        uint8_t drive_select = 0xE0 | ((lba >> 24) & 0x0F);
        outb(ATA_PRIMARY_DRIVE, drive_select);
        
        // Set sector count (1 sector at a time)
        outb(ATA_PRIMARY_SECCOUNT, 1);
        
        // Set LBA low byte (bits 0-7)
        outb(ATA_PRIMARY_LBA_LO, (uint8_t)(lba & 0xFF));
        
        // Set LBA mid byte (bits 8-15)
        outb(ATA_PRIMARY_LBA_MID, (uint8_t)((lba >> 8) & 0xFF));
        
        // Set LBA high byte (bits 16-23)
        outb(ATA_PRIMARY_LBA_HI, (uint8_t)((lba >> 16) & 0xFF));
        
        // Send read command
        outb(ATA_PRIMARY_CMD, ATA_CMD_READ_SECTORS);
        
        // Wait for data
        if (!ata_wait_drq()) {
            return 0;
        }
        
        // Read 256 words (512 bytes) from data port
        insw(ATA_PRIMARY_DATA, buffer, 256);
        
        // Move to next sector
        buffer += 512;
        lba++;
    }
    
    return 1;
}

//=============================================================================
// Main Kernel Loader Function
//=============================================================================

// Load kernel and return entry point address
// This function is called from assembly with boot information
uint64_t load_kernel_c(vbe_info_t* vbe, memory_info_t* mem) {
    // Graphics mode is already set by VESA - no text clearing needed
    
    // Wait for drive to be ready
    if (!ata_wait_ready()) {
        return 0; // Signal failure
    }
    
    // Read kernel from disk into memory
    void* kernel_dest = (void*)KERNEL_LOAD_ADDR;
    if (!ata_read_sectors(kernel_dest, KERNEL_START_SECTOR, KERNEL_SECTORS)) {
        return 0; // Signal failure
    }
    
    // Verify kernel was loaded (check if first 8 bytes are non-zero)
    uint64_t* kernel_start = (uint64_t*)KERNEL_LOAD_ADDR;
    if (*kernel_start == 0) {
        return 0; // Signal failure
    }
    
    // Return kernel entry point address
    // The caller (assembly bridge) will:
    // 1. Set up the stack
    // 2. Prepare boot info in registers
    // 3. Jump to this address
    return KERNEL_LOAD_ADDR;
}
