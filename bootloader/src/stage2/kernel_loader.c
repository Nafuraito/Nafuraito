//=============================================================================
// Nafuraito Kernel Loader
//=============================================================================
//
// What it does:
// 1. Provide basic memory management (a simple heap allocator)
// 2. Implement ATA disk driver to read from the hard drive
// 3. Mount the EXT4 filesystem
// 4. Find and load the kernel binary into memory
// 5. Return the kernel's entry point so it can jump to it
//
//=============================================================================

#include "bootloader.h"
#include "ext4_boot.h"

//=============================================================================
// Memory Management Stubs for EXT4 Driver
//=============================================================================
//
// The EXT4 driver needs malloc, free, memcpy, etc. 
//
// - A simple bump allocator (just hands out memory from a big static buffer)
// - No real "free" - we just leak memory (it's fine, we'll be done soon)
// - Basic string/memory manipulation functions
//
//=============================================================================

#define HEAP_SIZE (2 * 1024 * 1024)  // 2MB heap (plenty for our needs)
static uint8_t heap[HEAP_SIZE];      // Static buffer for allocations
static uint64_t heap_offset = 0;     // Current position in heap (bump pointer)

// Simple malloc: just return the next chunk of memory from our heap
// No fancy bookkeeping, no fragmentation handling - just bump the pointer
void* kmalloc(uint64_t size) {
    // Align to 16 bytes for performance and compatibility
    // (Some CPUs don't like unaligned accesses)
    size = (size + 15) & ~15;
    
    // Check if we have enough space left
    if (heap_offset + size > HEAP_SIZE) {
        return NULL;  // Out of memory - not much we can do about it
    }
    
    // Hand out the next chunk of memory
    void* ptr = &heap[heap_offset];
    heap_offset += size;  // Bump the pointer forward
    return ptr;
}

// "Free" memory - we don't actually free anything
// This is a bootloader; we'll be done in a few seconds anyway
void kfree(void* ptr) {
    // Intentionally do nothing
    (void)ptr;  // Silence "unused parameter" warning
}

// Fill memory with a specific byte value
// Like C's memset() - we need this for zeroing out structures
void* memset(void* s, int c, uint64_t n) {
    uint8_t* p = (uint8_t*)s;
    for (uint64_t i = 0; i < n; i++) {
        p[i] = (uint8_t)c;  // Set each byte to the specified value
    }
    return s;
}

// Copy memory from one place to another
// Like C's memcpy() - essential for moving data around
void* memcpy(void* dest, const void* src, uint64_t n) {
    uint8_t* d = (uint8_t*)dest;
    const uint8_t* s = (const uint8_t*)src;
    for (uint64_t i = 0; i < n; i++) {
        d[i] = s[i];  // Copy byte by byte
    }
    return dest;
}

// Compare two memory regions
// Returns 0 if they're the same, non-zero if different
int memcmp(const void* s1, const void* s2, uint64_t n) {
    const uint8_t* p1 = (const uint8_t*)s1;
    const uint8_t* p2 = (const uint8_t*)s2;
    for (uint64_t i = 0; i < n; i++) {
        if (p1[i] != p2[i]) {
            return p1[i] - p2[i];  // Return the difference
        }
    }
    return 0;  // All bytes match
}

// Get the length of a null-terminated string
uint64_t strlen(const char* s) {
    uint64_t len = 0;
    while (s[len]) {  // Count until we hit the '\0' terminator
        len++;
    }
    return len;
}

// Compare two strings up to n characters
// Returns 0 if they match, non-zero if different
int strncmp(const char* s1, const char* s2, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) {
        if (s1[i] != s2[i]) {
            return (unsigned char)s1[i] - (unsigned char)s2[i];
        }
        if (s1[i] == '\0') {  // Reached end of string early
            return 0;
        }
    }
    return 0;  // Strings match for the first n characters
}

//=============================================================================
// ATA PIO Driver Implementation
//=============================================================================
//
// ATA (Advanced Technology Attachment) is the interface for talking to IDE/SATA
// hard drives. PIO (Programmed I/O) is the simplest way to transfer data:
// we just read/write one word at a time using CPU I/O instructions.
//
// It's slow compared to DMA (which we don't have set up yet), but it works
// on every x86 machine since the dawn of time. Perfect for a bootloader!
//
//=============================================================================

// Wait for the ATA drive to be ready for commands
// The drive has status flags that tell us when it's busy, ready, or errored out
// Returns: 1 if ready, 0 on error/timeout
int ata_wait_ready(void) {
    uint32_t timeout = 100000;  // Don't wait forever (about 100ms on old hardware)
    uint8_t status;
    
    while (timeout--) {
        status = inb(ATA_PRIMARY_STATUS);  // Read the status register
        
        // Check for error - if the ERR bit is set, something went wrong
        if (status & ATA_STATUS_ERR) {
            return 0;  // Error - give up
        }
        
        // Check if busy - if BSY is set, the drive is still working
        if (status & ATA_STATUS_BSY) {
            continue;  // Keep waiting
        }
        
        // Check if ready - if RDY is set, we can send commands
        if (status & ATA_STATUS_RDY) {
            return 1;  // Ready to go!
        }
    }
    
    // We waited too long - timeout
    return 0;
}

// Wait for the drive to be ready to transfer data
// After sending a read command, we need to wait for the DRQ (Data Request) flag
// Returns: 1 if ready, 0 on error/timeout
int ata_wait_drq(void) {
    uint32_t timeout = 100000;
    uint8_t status;
    
    while (timeout--) {
        status = inb(ATA_PRIMARY_STATUS);
        
        // Check if busy - need to wait for BSY to clear first
        if (status & ATA_STATUS_BSY) {
            continue;
        }
        
        // Check for error
        if (status & ATA_STATUS_ERR) {
            return 0;
        }
        
        // Check if data request ready - DRQ means data is ready to read/write
        if (status & ATA_STATUS_DRQ) {
            return 1;  // Data ready!
        }
    }
    
    // Timeout
    return 0;
}

// Read sectors from the ATA drive
// This is the main function for reading data from the disk
//
// Parameters:
//   dest  - Where to put the data we read
//   lba   - Logical Block Address (which sector to start reading from)
//   count - How many 512-byte sectors to read
//
// Returns: 1 on success, 0 on failure
int ata_read_sectors(void* dest, uint64_t lba, uint32_t count) {
    uint8_t* buffer = (uint8_t*)dest;
    
    // Read one sector at a time
    // (We could read multiple sectors per command, but this is simpler and more reliable)
    for (uint32_t i = 0; i < count; i++) {
        // Wait for drive to be ready before sending a command
        if (!ata_wait_ready()) {
            return 0;  // Drive not ready - abort
        }
        
        // Select drive and set up LBA addressing
        // 0xE0 = master drive, LBA mode, bits 24-27 of LBA
        uint8_t drive_select = 0xE0 | ((lba >> 24) & 0x0F);
        outb(ATA_PRIMARY_DRIVE, drive_select);
        
        // Write sector count (we're reading 1 sector at a time)
        outb(ATA_PRIMARY_SECCOUNT, 1);
        
        // Write LBA (Logical Block Address) - 28-bit address split across 3 registers
        outb(ATA_PRIMARY_LBA_LO, (uint8_t)(lba & 0xFF));          // Bits 0-7
        outb(ATA_PRIMARY_LBA_MID, (uint8_t)((lba >> 8) & 0xFF));  // Bits 8-15
        outb(ATA_PRIMARY_LBA_HI, (uint8_t)((lba >> 16) & 0xFF));  // Bits 16-23
        // Bits 24-27 were set in drive_select above
        
        // Send the "read sectors" command
        outb(ATA_PRIMARY_CMD, ATA_CMD_READ_SECTORS);
        
        // Wait for the drive to fetch the data and set the DRQ flag
        if (!ata_wait_drq()) {
            return 0;  // Drive failed to provide data - abort
        }
        
        // Read 256 words (512 bytes = 1 sector) from the data port
        // insw reads 16 bits at a time, so 256 reads = 512 bytes
        insw(ATA_PRIMARY_DATA, buffer, 256);
        
        // Move to next sector
        buffer += 512;  // Advance buffer pointer
        lba++;          // Move to next LBA
    }
    
    return 1;  // Success!
}

//=============================================================================
// Main Kernel Loader Function
//=============================================================================
//
// This is it - the main event! This function is called from assembly code
// and is responsible for loading the kernel from the EXT4 filesystem.
//
// The process:
// 1. Initialize the ATA driver
// 2. Mount the EXT4 filesystem
// 3. Read the kernel binary from /nafuraito/kernel.bin
// 4. Return the kernel's entry point address
//
// If anything goes wrong, we return 0 to signal failure.
//
//=============================================================================

// Load kernel and return entry point address
// This function is called from assembly with boot information
// Parameters:
//   vbe - Video BIOS Extensions info (screen resolution, framebuffer, etc.)
//   mem - Memory map info (what RAM is available)
uint64_t load_kernel_c(vbe_info_t* vbe, memory_info_t* mem) {
    ext4_boot_ctx_t ext4_ctx;  // EXT4 filesystem context
    uint64_t bytes_read = 0;   // How many bytes we actually read
    int ret;                    // Return value from EXT4 functions
    
    // We don't use these yet, but we'll need them later to pass info to the kernel
    (void)vbe;
    (void)mem;
    
    // Debug message to serial port
    serial_str("C:LOAD\n");  // "C:" prefix means this is from C code
    
    // Step 1: Wait for the ATA drive to be ready
    // The drive might still be spinning up or initializing
    if (!ata_wait_ready()) {
        serial_str("C:ATA_FAIL\n");
        return 0;  // Can't read from disk - game over
    }
    serial_str("C:ATA_OK\n");
    
    // Step 2: Initialize the EXT4 filesystem
    // Our partition starts at sector 2048 (that's 1MB into the disk)
    // This is a common partition alignment for modern disks
    ret = ext4_boot_init(&ext4_ctx, 2048);
    if (ret != EXT4_SUCCESS) {
        serial_str("C:MOUNT_FAIL\n");
        return 0;  // Failed to mount - maybe corrupted filesystem?
    }
    serial_str("C:MOUNTED\n");
    
    // Step 3: Read the kernel file directly into memory
    // We'll load it at KERNEL_LOAD_ADDR (defined in bootloader.h)
    void* kernel_dest = (void*)KERNEL_LOAD_ADDR;
    uint64_t max_kernel_size = 10 * 1024 * 1024;  // Limit: 10MB (reasonable for a kernel)
    
    ret = ext4_boot_read_file(&ext4_ctx, "/nafuraito/kernel.bin", 
                               kernel_dest, max_kernel_size, &bytes_read);
    if (ret != EXT4_SUCCESS) {
        serial_str("C:READ_FAIL\n");
        return 0;  // Failed to read kernel - file not found or corrupted?
    }
    serial_str("C:READ_OK\n");
    
    // Step 4: Verify the kernel was actually loaded
    // Do a basic sanity check: make sure the first 8 bytes aren't zero
    // (A real kernel should have executable code at the start)
    uint64_t* kernel_start = (uint64_t*)KERNEL_LOAD_ADDR;
    if (*kernel_start == 0 || bytes_read == 0) {
        serial_str("C:VERIFY_FAIL\n");
        return 0;  // Kernel looks corrupted or empty
    }
    serial_str("C:OK\n");
    
    // Success! Return the kernel entry point address
    // The assembly code will jump to this address to start the kernel
    return KERNEL_LOAD_ADDR;
}
