/**
 * ext4_boot.c - EXT4 Filesystem Reader for Bootloader
 * 
 * This is a minimal, self-contained ext4 reader for the stage2 bootloader.
 * It reads an ext4 filesystem with journaling support from a raw disk.
 * 
 * Features:
 * - Read-only operation (safe for journaled filesystems)
 * - Supports extent-based block mapping (modern ext4)
 * - Supports indirect block mapping (legacy/compatibility)
 * - Uses static buffers (no dynamic memory allocation)
 * - Works with 1KB, 2KB, or 4KB block sizes
 */

#include "ext4_boot.h"

/* Static buffers for block reads - avoids dynamic allocation */
static uint8_t block_buffer[EXT4_BLOCK_BUFFER_SIZE] __attribute__((aligned(16)));
static uint8_t block_buffer2[EXT4_BLOCK_BUFFER_SIZE] __attribute__((aligned(16)));

/* Static superblock storage */
static ext4_superblock_t superblock;

/* ==========================================================================
 * ATA DISK I/O (borrowed from kernel_loader.c)
 * ========================================================================== */

/* Wait for ATA drive to be ready */
static int ata_wait_ready_ext4(void) {
    uint32_t timeout = 100000;
    uint8_t status;
    
    while (timeout--) {
        status = inb(ATA_PRIMARY_STATUS);
        if (status & ATA_STATUS_ERR) return 0;
        if (status & ATA_STATUS_BSY) continue;
        if (status & ATA_STATUS_RDY) return 1;
    }
    return 0;
}

/* Wait for data request */
static int ata_wait_drq_ext4(void) {
    uint32_t timeout = 100000;
    uint8_t status;
    
    while (timeout--) {
        status = inb(ATA_PRIMARY_STATUS);
        if (status & ATA_STATUS_BSY) continue;
        if (status & ATA_STATUS_ERR) return 0;
        if (status & ATA_STATUS_DRQ) return 1;
    }
    return 0;
}

/**
 * Read sectors from ATA drive
 * 
 * @param dest      Destination buffer
 * @param lba       Logical Block Address (sector number)
 * @param count     Number of 512-byte sectors to read
 * @return          1 on success, 0 on failure
 */
static int ata_read_sectors_ext4(void *dest, uint64_t lba, uint32_t count) {
    uint8_t *buffer = (uint8_t *)dest;
    
    for (uint32_t i = 0; i < count; i++) {
        if (!ata_wait_ready_ext4()) return 0;
        
        /* Select drive (LBA mode, master) */
        uint8_t drive_select = 0xE0 | ((lba >> 24) & 0x0F);
        outb(ATA_PRIMARY_DRIVE, drive_select);
        
        outb(ATA_PRIMARY_SECCOUNT, 1);
        outb(ATA_PRIMARY_LBA_LO, (uint8_t)(lba & 0xFF));
        outb(ATA_PRIMARY_LBA_MID, (uint8_t)((lba >> 8) & 0xFF));
        outb(ATA_PRIMARY_LBA_HI, (uint8_t)((lba >> 16) & 0xFF));
        outb(ATA_PRIMARY_CMD, ATA_CMD_READ_SECTORS);
        
        if (!ata_wait_drq_ext4()) return 0;
        
        insw(ATA_PRIMARY_DATA, buffer, 256);
        
        buffer += 512;
        lba++;
    }
    
    return 1;
}

/* ==========================================================================
 * HELPER FUNCTIONS
 * ========================================================================== */

static int ext4_strcmp(const char *s1, const char *s2, uint32_t len) {
    for (uint32_t i = 0; i < len; i++) {
        if (s1[i] != s2[i]) return 1;
    }
    return 0;
}

/* Copy memory */
static void ext4_memcpy(void *dest, const void *src, uint32_t n) {
    uint8_t *d = (uint8_t *)dest;
    const uint8_t *s = (const uint8_t *)src;
    while (n--) *d++ = *s++;
}

/* ==========================================================================
 * EXT4 BLOCK I/O
 * ========================================================================== */

/**
 * Read a filesystem block (1KB, 2KB, or 4KB)
 * 
 * @param ctx       Filesystem context
 * @param block     Block number
 * @param buffer    Destination buffer
 * @return          1 on success, 0 on failure
 */
static int ext4_read_block(ext4_boot_ctx_t *ctx, uint64_t block, void *buffer) {
    /* Calculate LBA: partition_lba + (block * block_size / 512) */
    uint64_t lba = ctx->partition_lba + (block * ctx->block_size / 512);
    uint32_t sectors = ctx->block_size / 512;
    
    return ata_read_sectors_ext4(buffer, lba, sectors);
}

/* ==========================================================================
 * EXT4 INODE HANDLING
 * ========================================================================== */

/**
 * Read an inode from the filesystem
 * 
 * @param ctx       Filesystem context
 * @param inode_num Inode number (1-based)
 * @param inode     Output inode structure
 * @return          EXT4_SUCCESS or error code
 */
static int ext4_read_inode(ext4_boot_ctx_t *ctx, uint32_t inode_num, ext4_inode_t *inode) {
    if (inode_num == 0) return EXT4_ERR_INVALID;
    
    /* Calculate which block group contains this inode */
    uint32_t group = (inode_num - 1) / ctx->inodes_per_group;
    uint32_t index = (inode_num - 1) % ctx->inodes_per_group;
    
    /* Read group descriptor */
    /* Group descriptors start at block 1 (for block_size >= 2KB) or block 2 (for 1KB blocks) */
    uint32_t gd_block = (ctx->block_size == 1024) ? 2 : 1;
    uint32_t gd_per_block = ctx->block_size / ctx->desc_size;
    uint32_t gd_block_offset = group / gd_per_block;
    uint32_t gd_index_in_block = group % gd_per_block;
    
    if (!ext4_read_block(ctx, gd_block + gd_block_offset, block_buffer)) {
        return EXT4_ERR_IO;
    }
    
    /* Get inode table location from group descriptor */
    ext4_group_desc_t *gd = (ext4_group_desc_t *)(block_buffer + gd_index_in_block * ctx->desc_size);
    uint64_t inode_table = gd->bg_inode_table_lo;
    if (ctx->use_64bit) {
        inode_table |= ((uint64_t)gd->bg_inode_table_hi << 32);
    }
    
    /* Calculate block and offset within block */
    uint32_t inodes_per_block = ctx->block_size / ctx->inode_size;
    uint64_t inode_block = inode_table + (index / inodes_per_block);
    uint32_t inode_offset = (index % inodes_per_block) * ctx->inode_size;
    
    if (!ext4_read_block(ctx, inode_block, block_buffer)) {
        return EXT4_ERR_IO;
    }
    
    ext4_memcpy(inode, block_buffer + inode_offset, sizeof(ext4_inode_t));
    
    return EXT4_SUCCESS;
}

/* ==========================================================================
 * EXT4 BLOCK MAPPING
 * ========================================================================== */

/**
 * Get physical block from extent tree
 */
static uint64_t ext4_get_block_extent(ext4_boot_ctx_t *ctx, ext4_inode_t *inode, 
                                       uint64_t logical_block) {
    ext4_extent_header_t *header;
    ext4_extent_t *extent;
    ext4_extent_idx_t *idx;
    uint64_t physical_block;
    int depth;
    
    /* Start with the extent tree root in the inode */
    header = (ext4_extent_header_t *)inode->i_block;
    
    if (header->eh_magic != EXT4_EXT_MAGIC) {
        return 0;
    }
    
    depth = header->eh_depth;
    
    /* Navigate internal nodes */
    while (depth > 0) {
        idx = (ext4_extent_idx_t *)(header + 1);
        
        /* Find the right child */
        ext4_extent_idx_t *best = NULL;
        for (int i = 0; i < header->eh_entries; i++) {
            if (idx[i].ei_block <= logical_block) {
                best = &idx[i];
            } else {
                break;
            }
        }
        
        if (!best) return 0;
        
        /* Read the child block */
        physical_block = best->ei_leaf_lo | ((uint64_t)best->ei_leaf_hi << 32);
        
        if (!ext4_read_block(ctx, physical_block, block_buffer2)) {
            return 0;
        }
        
        header = (ext4_extent_header_t *)block_buffer2;
        depth--;
    }
    
    /* Now at leaf level - find the extent */
    extent = (ext4_extent_t *)(header + 1);
    
    for (int i = 0; i < header->eh_entries; i++) {
        uint32_t ee_block = extent[i].ee_block;
        uint16_t ee_len = extent[i].ee_len & 0x7FFF; /* Mask uninitialized flag */
        
        if (logical_block >= ee_block && logical_block < ee_block + ee_len) {
            physical_block = extent[i].ee_start_lo | 
                            ((uint64_t)extent[i].ee_start_hi << 32);
            physical_block += (logical_block - ee_block);
            return physical_block;
        }
    }
    
    return 0;
}

/**
 * Get physical block using indirect block mapping (legacy)
 */
static uint64_t ext4_get_block_indirect(ext4_boot_ctx_t *ctx, ext4_inode_t *inode, 
                                         uint64_t logical_block) {
    uint32_t ptrs_per_block = ctx->block_size / sizeof(uint32_t);
    uint32_t *buffer;
    
    /* Direct blocks (0-11) */
    if (logical_block < EXT4_NDIR_BLOCKS) {
        return inode->i_block[logical_block];
    }
    
    logical_block -= EXT4_NDIR_BLOCKS;
    
    /* Single indirect block */
    if (logical_block < ptrs_per_block) {
        if (!ext4_read_block(ctx, inode->i_block[EXT4_IND_BLOCK], block_buffer2)) {
            return 0;
        }
        buffer = (uint32_t *)block_buffer2;
        return buffer[logical_block];
    }
    
    logical_block -= ptrs_per_block;
    
    /* Double indirect block */
    if (logical_block < ptrs_per_block * ptrs_per_block) {
        if (!ext4_read_block(ctx, inode->i_block[EXT4_DIND_BLOCK], block_buffer2)) {
            return 0;
        }
        buffer = (uint32_t *)block_buffer2;
        
        uint32_t ind_block = buffer[logical_block / ptrs_per_block];
        if (!ext4_read_block(ctx, ind_block, block_buffer2)) {
            return 0;
        }
        buffer = (uint32_t *)block_buffer2;
        
        return buffer[logical_block % ptrs_per_block];
    }
    
    logical_block -= ptrs_per_block * ptrs_per_block;
    
    /* Triple indirect block */
    if (!ext4_read_block(ctx, inode->i_block[EXT4_TIND_BLOCK], block_buffer2)) {
        return 0;
    }
    buffer = (uint32_t *)block_buffer2;
    
    uint32_t dind_block = buffer[logical_block / (ptrs_per_block * ptrs_per_block)];
    if (!ext4_read_block(ctx, dind_block, block_buffer2)) {
        return 0;
    }
    buffer = (uint32_t *)block_buffer2;
    
    uint32_t ind_block = buffer[(logical_block / ptrs_per_block) % ptrs_per_block];
    if (!ext4_read_block(ctx, ind_block, block_buffer2)) {
        return 0;
    }
    buffer = (uint32_t *)block_buffer2;
    
    return buffer[logical_block % ptrs_per_block];
}

/**
 * Get physical block for a logical block in an inode
 */
static uint64_t ext4_get_block(ext4_boot_ctx_t *ctx, ext4_inode_t *inode, 
                                uint64_t logical_block) {
    if (inode->i_flags & EXT4_EXTENTS_FL) {
        return ext4_get_block_extent(ctx, inode, logical_block);
    } else {
        return ext4_get_block_indirect(ctx, inode, logical_block);
    }
}

/* ==========================================================================
 * EXT4 DIRECTORY OPERATIONS
 * ========================================================================== */

/**
 * Find entry in a directory
 * 
 * @param ctx       Filesystem context
 * @param dir_inode Directory inode
 * @param name      Entry name to find
 * @param name_len  Length of name
 * @param inode_out Output inode number
 * @return          EXT4_SUCCESS or error code
 */
static int ext4_dir_find_entry(ext4_boot_ctx_t *ctx, ext4_inode_t *dir_inode,
                                const char *name, uint32_t name_len, 
                                uint32_t *inode_out) {
    uint64_t dir_size = dir_inode->i_size_lo;
    uint64_t offset = 0;
    
    while (offset < dir_size) {
        uint64_t logical_block = offset / ctx->block_size;
        uint64_t physical_block = ext4_get_block(ctx, dir_inode, logical_block);
        
        if (physical_block == 0) {
            offset += ctx->block_size;
            continue;
        }
        
        if (!ext4_read_block(ctx, physical_block, block_buffer)) {
            return EXT4_ERR_IO;
        }
        
        uint32_t block_offset = 0;
        while (block_offset < ctx->block_size) {
            ext4_dir_entry_t *entry = (ext4_dir_entry_t *)(block_buffer + block_offset);
            
            if (entry->rec_len == 0) {
                break;
            }
            
            if (entry->inode != 0 && entry->name_len == name_len) {
                if (ext4_strcmp(entry->name, name, name_len) == 0) {
                    *inode_out = entry->inode;
                    return EXT4_SUCCESS;
                }
            }
            
            block_offset += entry->rec_len;
        }
        
        offset += ctx->block_size;
    }
    
    return EXT4_ERR_NOTFOUND;
}

/**
 * Lookup a path and return the inode number
 * 
 * @param ctx       Filesystem context
 * @param path      Path to lookup (e.g., "/nafuraito/kernel.bin")
 * @param inode_out Output inode number
 * @return          EXT4_SUCCESS or error code
 */
static int ext4_lookup_path(ext4_boot_ctx_t *ctx, const char *path, uint32_t *inode_out) {
    ext4_inode_t inode;
    uint32_t current_inode = EXT4_ROOT_INODE;
    const char *p = path;
    char name[256];
    int ret;
    
    /* Skip leading slash */
    while (*p == '/') p++;
    
    /* Empty path or root */
    if (*p == '\0') {
        *inode_out = EXT4_ROOT_INODE;
        return EXT4_SUCCESS;
    }
    
    while (*p) {
        /* Extract next path component */
        uint32_t len = 0;
        while (p[len] && p[len] != '/' && len < 255) {
            name[len] = p[len];
            len++;
        }
        name[len] = '\0';
        
        if (len == 0) {
            p++;
            continue;
        }
        
        /* Read current directory inode */
        ret = ext4_read_inode(ctx, current_inode, &inode);
        if (ret != EXT4_SUCCESS) {
            return ret;
        }
        
        /* Search directory for name */
        ret = ext4_dir_find_entry(ctx, &inode, name, len, &current_inode);
        if (ret != EXT4_SUCCESS) {
            return ret;
        }
        
        p += len;
        while (*p == '/') p++;
    }
    
    *inode_out = current_inode;
    return EXT4_SUCCESS;
}

/* ==========================================================================
 * PUBLIC API
 * ========================================================================== */

/**
 * Initialize ext4 filesystem at given partition LBA offset
 */
int ext4_boot_init(ext4_boot_ctx_t *ctx, uint64_t part_lba) {
    ctx->partition_lba = part_lba;
    
    /* Read superblock (at byte offset 1024 from partition start) */
    /* That's sector 2 (relative to partition) for 512-byte sectors */
    if (!ata_read_sectors_ext4(block_buffer, part_lba + 2, 2)) {
        return EXT4_ERR_IO;
    }
    
    ext4_memcpy(&superblock, block_buffer, sizeof(ext4_superblock_t));
    
    /* Validate magic number */
    if (superblock.s_magic != EXT4_SUPER_MAGIC) {
        return EXT4_ERR_INVALID_MAGIC;
    }
    
    /* Calculate filesystem parameters */
    ctx->block_size = EXT4_MIN_BLOCK_SIZE << superblock.s_log_block_size;
    ctx->blocks_per_group = superblock.s_blocks_per_group;
    ctx->inodes_per_group = superblock.s_inodes_per_group;
    ctx->inode_size = superblock.s_inode_size;
    ctx->first_data_block = superblock.s_first_data_block;
    
    /* Check for 64-bit mode */
    if (superblock.s_feature_incompat & EXT4_FEATURE_INCOMPAT_64BIT) {
        ctx->use_64bit = 1;
        ctx->desc_size = superblock.s_desc_size;
        if (ctx->desc_size < 32) ctx->desc_size = 32;
    } else {
        ctx->use_64bit = 0;
        ctx->desc_size = 32;
    }
    
    /* Check for extent support */
    ctx->use_extents = (superblock.s_feature_incompat & EXT4_FEATURE_INCOMPAT_EXTENTS) ? 1 : 0;
    
    /* Validate block size */
    if (ctx->block_size > EXT4_MAX_BLOCK_SIZE || ctx->block_size < EXT4_MIN_BLOCK_SIZE) {
        return EXT4_ERR_INVALID;
    }
    
    return EXT4_SUCCESS;
}

/**
 * Get file size without reading contents
 */
int ext4_boot_get_file_size(ext4_boot_ctx_t *ctx, const char *path, uint64_t *size_out) {
    ext4_inode_t inode;
    uint32_t inode_num;
    int ret;
    
    ret = ext4_lookup_path(ctx, path, &inode_num);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    
    ret = ext4_read_inode(ctx, inode_num, &inode);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    
    *size_out = inode.i_size_lo | ((uint64_t)inode.i_size_hi << 32);
    return EXT4_SUCCESS;
}

/**
 * Read file from ext4 filesystem into memory
 */
int ext4_boot_read_file(ext4_boot_ctx_t *ctx, const char *path, 
                        void *dest, uint64_t max_size, uint64_t *size_out) {
    ext4_inode_t inode;
    uint32_t inode_num;
    uint64_t file_size;
    uint64_t bytes_read = 0;
    uint8_t *buffer = (uint8_t *)dest;
    int ret;
    
    /* Lookup file path */
    ret = ext4_lookup_path(ctx, path, &inode_num);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    
    /* Read file inode */
    ret = ext4_read_inode(ctx, inode_num, &inode);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    
    /* Get file size */
    file_size = inode.i_size_lo | ((uint64_t)inode.i_size_hi << 32);
    
    if (file_size > max_size) {
        file_size = max_size;
    }
    
    /* Read file blocks */
    uint64_t remaining = file_size;
    uint64_t logical_block = 0;
    
    while (remaining > 0) {
        uint64_t physical_block = ext4_get_block(ctx, &inode, logical_block);
        
        if (physical_block == 0) {
            /* Sparse file - zero this block */
            uint32_t to_zero = remaining > ctx->block_size ? ctx->block_size : remaining;
            for (uint32_t i = 0; i < to_zero; i++) {
                buffer[i] = 0;
            }
            buffer += to_zero;
            bytes_read += to_zero;
            remaining -= to_zero;
        } else {
            /* Read block directly to destination */
            uint32_t to_read = remaining > ctx->block_size ? ctx->block_size : remaining;
            
            /* Read full block (might be larger than remaining, but that's OK) */
            if (!ext4_read_block(ctx, physical_block, buffer)) {
                return EXT4_ERR_IO;
            }
            
            buffer += to_read;
            bytes_read += to_read;
            remaining -= to_read;
        }
        
        logical_block++;
    }
    
    *size_out = bytes_read;
    return EXT4_SUCCESS;
}
