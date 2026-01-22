/**
 * ext4_boot.h - EXT4 Filesystem Reader for Bootloader
 * 
 * This is a minimal, self-contained ext4 reader for the stage2 bootloader.
 * It reads an ext4 filesystem with journaling support from a raw disk.
 * 
 * Design constraints:
 * - No dynamic memory allocation (uses static buffers)
 * - Read-only operation (no journal replay, just skip recovery)
 * - Supports both extent-based and indirect block mapping
 * - Simple path lookup for loading kernel
 */

#ifndef EXT4_BOOT_H
#define EXT4_BOOT_H

#include "bootloader.h"

/* EXT4 Magic Number */
#define EXT4_SUPER_MAGIC        0xEF53

/* EXT4 Superblock Offset (always at byte offset 1024) */
#define EXT4_SUPERBLOCK_OFFSET  1024
#define EXT4_SUPERBLOCK_SIZE    1024
#define EXT4_MIN_BLOCK_SIZE     1024

/* EXT4 Inode Constants */
#define EXT4_ROOT_INODE         2
#define EXT4_GOOD_OLD_INODE_SIZE 128

/* EXT4 File Types (in directory entries) */
#define EXT4_FT_UNKNOWN         0
#define EXT4_FT_REG_FILE        1
#define EXT4_FT_DIR             2

/* EXT4 Inode Flags */
#define EXT4_EXTENTS_FL         0x00080000

/* EXT4 Feature Flags */
#define EXT4_FEATURE_INCOMPAT_EXTENTS       0x0040
#define EXT4_FEATURE_INCOMPAT_64BIT         0x0080
#define EXT4_FEATURE_COMPAT_HAS_JOURNAL     0x0004

/* Block mapping constants */
#define EXT4_NDIR_BLOCKS        12
#define EXT4_IND_BLOCK          12
#define EXT4_DIND_BLOCK         13
#define EXT4_TIND_BLOCK         14
#define EXT4_N_BLOCKS           15
#define EXT4_NAME_LEN           255

/* Extent Tree Magic */
#define EXT4_EXT_MAGIC          0xF30A

/* Error Codes */
#define EXT4_SUCCESS            0
#define EXT4_ERR_INVALID_MAGIC  -1
#define EXT4_ERR_IO             -2
#define EXT4_ERR_NOTFOUND       -4
#define EXT4_ERR_INVALID        -5

/**
 * EXT4 Superblock Structure (relevant fields only)
 */
typedef struct __attribute__((packed)) {
    uint32_t s_inodes_count;          /* 0x00 */
    uint32_t s_blocks_count_lo;       /* 0x04 */
    uint32_t s_r_blocks_count_lo;     /* 0x08 */
    uint32_t s_free_blocks_count_lo;  /* 0x0C */
    uint32_t s_free_inodes_count;     /* 0x10 */
    uint32_t s_first_data_block;      /* 0x14 */
    uint32_t s_log_block_size;        /* 0x18 */
    uint32_t s_log_cluster_size;      /* 0x1C */
    uint32_t s_blocks_per_group;      /* 0x20 */
    uint32_t s_clusters_per_group;    /* 0x24 */
    uint32_t s_inodes_per_group;      /* 0x28 */
    uint32_t s_mtime;                 /* 0x2C */
    uint32_t s_wtime;                 /* 0x30 */
    uint16_t s_mnt_count;             /* 0x34 */
    uint16_t s_max_mnt_count;         /* 0x36 */
    uint16_t s_magic;                 /* 0x38 */
    uint16_t s_state;                 /* 0x3A */
    uint16_t s_errors;                /* 0x3C */
    uint16_t s_minor_rev_level;       /* 0x3E */
    uint32_t s_lastcheck;             /* 0x40 */
    uint32_t s_checkinterval;         /* 0x44 */
    uint32_t s_creator_os;            /* 0x48 */
    uint32_t s_rev_level;             /* 0x4C */
    uint16_t s_def_resuid;            /* 0x50 */
    uint16_t s_def_resgid;            /* 0x52 */
    /* EXT4 specific fields (revision 1+) */
    uint32_t s_first_ino;             /* 0x54 */
    uint16_t s_inode_size;            /* 0x58 */
    uint16_t s_block_group_nr;        /* 0x5A */
    uint32_t s_feature_compat;        /* 0x5C */
    uint32_t s_feature_incompat;      /* 0x60 */
    uint32_t s_feature_ro_compat;     /* 0x64 */
    uint8_t  s_uuid[16];              /* 0x68 */
    char     s_volume_name[16];       /* 0x78 */
    char     s_last_mounted[64];      /* 0x88 */
    uint32_t s_algorithm_usage_bitmap;/* 0xC8 */
    uint8_t  s_prealloc_blocks;       /* 0xCC */
    uint8_t  s_prealloc_dir_blocks;   /* 0xCD */
    uint16_t s_reserved_gdt_blocks;   /* 0xCE */
    /* Journal fields */
    uint8_t  s_journal_uuid[16];      /* 0xD0 */
    uint32_t s_journal_inum;          /* 0xE0 */
    uint32_t s_journal_dev;           /* 0xE4 */
    uint32_t s_last_orphan;           /* 0xE8 */
    uint32_t s_hash_seed[4];          /* 0xEC */
    uint8_t  s_def_hash_version;      /* 0xFC */
    uint8_t  s_jnl_backup_type;       /* 0xFD */
    uint16_t s_desc_size;             /* 0xFE */
    uint32_t s_default_mount_opts;    /* 0x100 */
    uint32_t s_first_meta_bg;         /* 0x104 */
    uint32_t s_mkfs_time;             /* 0x108 */
    uint32_t s_jnl_blocks[17];        /* 0x10C */
    /* 64-bit support */
    uint32_t s_blocks_count_hi;       /* 0x150 */
    uint32_t s_r_blocks_count_hi;     /* 0x154 */
    uint32_t s_free_blocks_count_hi;  /* 0x158 */
    uint16_t s_min_extra_isize;       /* 0x15C */
    uint16_t s_want_extra_isize;      /* 0x15E */
    uint32_t s_flags;                 /* 0x160 */
    /* ... rest of superblock not needed for read-only boot ... */
} ext4_superblock_t;

/**
 * EXT4 Block Group Descriptor (32-bit version, 32 bytes)
 * For 64-bit filesystems, this extends to 64 bytes
 */
typedef struct __attribute__((packed)) {
    uint32_t bg_block_bitmap_lo;      /* 0x00 */
    uint32_t bg_inode_bitmap_lo;      /* 0x04 */
    uint32_t bg_inode_table_lo;       /* 0x08 */
    uint16_t bg_free_blocks_count_lo; /* 0x0C */
    uint16_t bg_free_inodes_count_lo; /* 0x0E */
    uint16_t bg_used_dirs_count_lo;   /* 0x10 */
    uint16_t bg_flags;                /* 0x12 */
    uint32_t bg_exclude_bitmap_lo;    /* 0x14 */
    uint16_t bg_block_bitmap_csum_lo; /* 0x18 */
    uint16_t bg_inode_bitmap_csum_lo; /* 0x1A */
    uint16_t bg_itable_unused_lo;     /* 0x1C */
    uint16_t bg_checksum;             /* 0x1E */
    /* 64-bit fields (if desc_size >= 64) */
    uint32_t bg_block_bitmap_hi;      /* 0x20 */
    uint32_t bg_inode_bitmap_hi;      /* 0x24 */
    uint32_t bg_inode_table_hi;       /* 0x28 */
    uint16_t bg_free_blocks_count_hi; /* 0x2C */
    uint16_t bg_free_inodes_count_hi; /* 0x2E */
    uint16_t bg_used_dirs_count_hi;   /* 0x30 */
    uint16_t bg_itable_unused_hi;     /* 0x32 */
    uint32_t bg_exclude_bitmap_hi;    /* 0x34 */
    uint16_t bg_block_bitmap_csum_hi; /* 0x38 */
    uint16_t bg_inode_bitmap_csum_hi; /* 0x3A */
    uint32_t bg_reserved;             /* 0x3C */
} ext4_group_desc_t;

/**
 * EXT4 Inode Structure (256 bytes in modern ext4)
 */
typedef struct __attribute__((packed)) {
    uint16_t i_mode;                  /* 0x00 File mode */
    uint16_t i_uid;                   /* 0x02 Owner UID low */
    uint32_t i_size_lo;               /* 0x04 Size in bytes (low 32 bits) */
    uint32_t i_atime;                 /* 0x08 Access time */
    uint32_t i_ctime;                 /* 0x0C Creation time */
    uint32_t i_mtime;                 /* 0x10 Modification time */
    uint32_t i_dtime;                 /* 0x14 Deletion time */
    uint16_t i_gid;                   /* 0x18 Group ID low */
    uint16_t i_links_count;           /* 0x1A Links count */
    uint32_t i_blocks_lo;             /* 0x1C Blocks count (low) */
    uint32_t i_flags;                 /* 0x20 File flags */
    uint32_t i_osd1;                  /* 0x24 OS dependent 1 */
    uint32_t i_block[EXT4_N_BLOCKS];  /* 0x28 Block pointers or extent tree */
    uint32_t i_generation;            /* 0x64 File version (for NFS) */
    uint32_t i_file_acl_lo;           /* 0x68 File ACL (low) */
    uint32_t i_size_hi;               /* 0x6C Size in bytes (high 32 bits) */
    uint32_t i_obso_faddr;            /* 0x70 Obsolete fragment address */
    uint16_t i_blocks_hi;             /* 0x74 Blocks count (high) */
    uint16_t i_file_acl_hi;           /* 0x76 File ACL (high) */
    uint16_t i_uid_hi;                /* 0x78 Owner UID high */
    uint16_t i_gid_hi;                /* 0x7A Group ID high */
    uint16_t i_checksum_lo;           /* 0x7C Inode checksum (low) */
    uint16_t i_reserved;              /* 0x7E Reserved */
    /* Extended inode fields (if inode_size > 128) */
    uint16_t i_extra_isize;           /* 0x80 Size of extra fields */
    uint16_t i_checksum_hi;           /* 0x82 Inode checksum (high) */
    uint32_t i_ctime_extra;           /* 0x84 Extra ctime bits */
    uint32_t i_mtime_extra;           /* 0x88 Extra mtime bits */
    uint32_t i_atime_extra;           /* 0x8C Extra atime bits */
    uint32_t i_crtime;                /* 0x90 File creation time */
    uint32_t i_crtime_extra;          /* 0x94 Extra crtime bits */
    uint32_t i_version_hi;            /* 0x98 High version number */
    uint32_t i_projid;                /* 0x9C Project ID */
} ext4_inode_t;

/**
 * EXT4 Directory Entry Structure
 */
typedef struct __attribute__((packed)) {
    uint32_t inode;                   /* Inode number */
    uint16_t rec_len;                 /* Directory entry length */
    uint8_t  name_len;                /* Name length */
    uint8_t  file_type;               /* File type */
    char     name[EXT4_NAME_LEN];     /* File name (not null-terminated!) */
} ext4_dir_entry_t;

/**
 * EXT4 Extent Header
 */
typedef struct __attribute__((packed)) {
    uint16_t eh_magic;                /* Magic number 0xF30A */
    uint16_t eh_entries;              /* Number of valid entries */
    uint16_t eh_max;                  /* Max entries capacity */
    uint16_t eh_depth;                /* Tree depth (0 = leaf) */
    uint32_t eh_generation;           /* Generation of the tree */
} ext4_extent_header_t;

/**
 * EXT4 Extent Index (internal nodes)
 */
typedef struct __attribute__((packed)) {
    uint32_t ei_block;                /* Logical block covered */
    uint32_t ei_leaf_lo;              /* Physical block of child (low) */
    uint16_t ei_leaf_hi;              /* Physical block of child (high) */
    uint16_t ei_unused;               /* Unused */
} ext4_extent_idx_t;

/**
 * EXT4 Extent (leaf nodes)
 */
typedef struct __attribute__((packed)) {
    uint32_t ee_block;                /* First logical block covered */
    uint16_t ee_len;                  /* Number of blocks covered */
    uint16_t ee_start_hi;             /* Physical block (high bits) */
    uint32_t ee_start_lo;             /* Physical block (low bits) */
} ext4_extent_t;

/**
 * EXT4 Bootloader Context (all static, no dynamic allocation)
 */
typedef struct {
    /* Filesystem parameters (derived from superblock) */
    uint32_t block_size;              /* Block size in bytes */
    uint32_t blocks_per_group;        /* Blocks per block group */
    uint32_t inodes_per_group;        /* Inodes per block group */
    uint32_t inode_size;              /* Inode size in bytes */
    uint32_t desc_size;               /* Group descriptor size */
    uint32_t first_data_block;        /* First data block number */
    uint8_t  use_extents;             /* 1 if extents are used */
    uint8_t  use_64bit;               /* 1 if 64-bit filesystem */
    uint64_t partition_lba;           /* LBA offset of ext4 partition */
} ext4_boot_ctx_t;

/* Maximum block size we support (4KB is typical) */
#define EXT4_MAX_BLOCK_SIZE     4096

/* Static buffer for block reads */
#define EXT4_BLOCK_BUFFER_SIZE  EXT4_MAX_BLOCK_SIZE

/**
 * Initialize ext4 filesystem at given partition LBA offset
 * 
 * @param ctx       Output context structure (caller allocated)
 * @param part_lba  LBA sector offset of the ext4 partition
 * @return          EXT4_SUCCESS or error code
 */
int ext4_boot_init(ext4_boot_ctx_t *ctx, uint64_t part_lba);

/**
 * Read file from ext4 filesystem into memory
 * 
 * @param ctx       Initialized filesystem context
 * @param path      Path to file (e.g., "/nafuraito/kernel.bin")
 * @param dest      Destination memory address
 * @param max_size  Maximum bytes to read
 * @param size_out  Actual bytes read (output)
 * @return          EXT4_SUCCESS or error code
 */
int ext4_boot_read_file(ext4_boot_ctx_t *ctx, const char *path, 
                        void *dest, uint64_t max_size, uint64_t *size_out);

/**
 * Get file size without reading contents
 * 
 * @param ctx       Initialized filesystem context
 * @param path      Path to file
 * @param size_out  File size in bytes (output)
 * @return          EXT4_SUCCESS or error code
 */
int ext4_boot_get_file_size(ext4_boot_ctx_t *ctx, const char *path, uint64_t *size_out);

#endif /* EXT4_BOOT_H */
