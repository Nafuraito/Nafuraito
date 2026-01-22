// Nafuraito Kernel Loader (C Implementation)
//
// Loads the kernel binary from an ext4 filesystem at /nafuraito/kernel.bin
// using ATA PIO mode and returns the kernel entry point address.

#include "bootloader.h"
#include "ext4_boot.h"

//=============================================================================
// Configuration
//=============================================================================

// Path to kernel binary on the ext4 filesystem
#define KERNEL_PATH "/nafuraito/kernel.bin"

// Maximum kernel size (8MB should be plenty)
#define MAX_KERNEL_SIZE (8 * 1024 * 1024)

// Partition LBA offset - ext4 partition starts after stage1 + stage2 bootloader
// Stage1 = 1 sector, Stage2 = 20 sectors, so partition starts at sector 64
#define EXT4_PARTITION_LBA 64

//=============================================================================
// Static ext4 context
//=============================================================================

static ext4_boot_ctx_t ext4_ctx;

//=============================================================================
// Main Kernel Loader Function
//=============================================================================

// Load kernel and return entry point address
// This function is called from assembly with boot information
uint64_t load_kernel_c(vbe_info_t* vbe, memory_info_t* mem) {
    int ret;
    uint64_t kernel_size;
    
    // Graphics mode is already set by VESA - no text clearing needed
    
    // Initialize ext4 filesystem driver
    // The ext4 partition starts after the bootloader sectors
    ret = ext4_boot_init(&ext4_ctx, EXT4_PARTITION_LBA);
    if (ret != EXT4_SUCCESS) {
        return 0; // Signal failure - could not initialize ext4
    }
    
    // Load kernel from ext4 filesystem
    void* kernel_dest = (void*)KERNEL_LOAD_ADDR;
    ret = ext4_boot_read_file(&ext4_ctx, KERNEL_PATH, kernel_dest, 
                               MAX_KERNEL_SIZE, &kernel_size);
    if (ret != EXT4_SUCCESS) {
        return 0; // Signal failure - could not read kernel
    }
    
    // Verify kernel was loaded (check if we got some data)
    if (kernel_size == 0) {
        return 0; // Signal failure - empty kernel
    }
    
    // Verify kernel magic/header (check if first 8 bytes are non-zero)
    uint64_t* kernel_start = (uint64_t*)KERNEL_LOAD_ADDR;
    if (*kernel_start == 0) {
        return 0; // Signal failure - invalid kernel
    }
    
    // Return kernel entry point address
    // The caller (assembly bridge) will:
    // 1. Set up the stack
    // 2. Prepare boot info in registers
    // 3. Jump to this address
    return KERNEL_LOAD_ADDR;
}
