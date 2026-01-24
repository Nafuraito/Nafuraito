// Nafuraito Kernel Loader (C Implementation)
//
// Loads the kernel binary from disk using EXT4 filesystem
// and returns the kernel entry point address.

#include "bootloader.h"
#include "ext4_boot.h"

//=============================================================================
// Memory Management Stubs for EXT4 Driver
//=============================================================================

#define HEAP_SIZE (2 * 1024 * 1024)  // 2MB heap
static uint8_t heap[HEAP_SIZE];
static uint64_t heap_offset = 0;

void* kmalloc(uint64_t size) {
    // Align to 16 bytes
    size = (size + 15) & ~15;
    
    if (heap_offset + size > HEAP_SIZE) {
        return NULL;
    }
    
    void* ptr = &heap[heap_offset];
    heap_offset += size;
    return ptr;
}

void kfree(void* ptr) {
    // Simple allocator doesn't support free
    (void)ptr;
}

void* memset(void* s, int c, uint64_t n) {
    uint8_t* p = (uint8_t*)s;
    for (uint64_t i = 0; i < n; i++) {
        p[i] = (uint8_t)c;
    }
    return s;
}

void* memcpy(void* dest, const void* src, uint64_t n) {
    uint8_t* d = (uint8_t*)dest;
    const uint8_t* s = (const uint8_t*)src;
    for (uint64_t i = 0; i < n; i++) {
        d[i] = s[i];
    }
    return dest;
}

int memcmp(const void* s1, const void* s2, uint64_t n) {
    const uint8_t* p1 = (const uint8_t*)s1;
    const uint8_t* p2 = (const uint8_t*)s2;
    for (uint64_t i = 0; i < n; i++) {
        if (p1[i] != p2[i]) {
            return p1[i] - p2[i];
        }
    }
    return 0;
}

uint64_t strlen(const char* s) {
    uint64_t len = 0;
    while (s[len]) {
        len++;
    }
    return len;
}

int strncmp(const char* s1, const char* s2, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) {
        if (s1[i] != s2[i]) {
            return (unsigned char)s1[i] - (unsigned char)s2[i];
        }
        if (s1[i] == '\0') {
            return 0;
        }
    }
    return 0;
}

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
    ext4_boot_ctx_t ext4_ctx;
    uint64_t bytes_read = 0;
    int ret;
    
    (void)vbe;
    (void)mem;
    
    serial_str("C:LOAD\n");
    
    // Wait for drive to be ready
    if (!ata_wait_ready()) {
        serial_str("C:ATA_FAIL\n");
        return 0; // Signal failure
    }
    serial_str("C:ATA_OK\n");
    
    // Initialize EXT4 filesystem (partition starts at sector 2048 = 1MiB)
    ret = ext4_boot_init(&ext4_ctx, 2048);
    if (ret != EXT4_SUCCESS) {
        serial_str("C:MOUNT_FAIL\n");
        return 0; // Failed to mount filesystem
    }
    serial_str("C:MOUNTED\n");
    
    // Read kernel file directly into memory
    void* kernel_dest = (void*)KERNEL_LOAD_ADDR;
    uint64_t max_kernel_size = 10 * 1024 * 1024; // 10MB max kernel size
    
    ret = ext4_boot_read_file(&ext4_ctx, "/nafuraito/kernel.bin", 
                               kernel_dest, max_kernel_size, &bytes_read);
    if (ret != EXT4_SUCCESS) {
        serial_str("C:READ_FAIL\n");
        return 0; // Failed to read kernel
    }
    serial_str("C:READ_OK\n");
    
    // Verify kernel was loaded (check if first 8 bytes are non-zero)
    uint64_t* kernel_start = (uint64_t*)KERNEL_LOAD_ADDR;
    if (*kernel_start == 0 || bytes_read == 0) {
        serial_str("C:VERIFY_FAIL\n");
        return 0; // Signal failure
    }
    serial_str("C:OK\n");
    
    // Return kernel entry point address
    return KERNEL_LOAD_ADDR;
}
