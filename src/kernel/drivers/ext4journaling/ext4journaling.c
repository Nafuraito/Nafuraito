/**
 * ext4journaling.c - EXT4 Journaling Filesystem Driver
 * 
 * This driver implements support for the EXT4 filesystem with journaling (JBD2)
 * for the Nafuraito kernel.
 */

#include "../stdint.h"
#include <stddef.h> // XXX
#include <stdbool.h>

/* EXT4 Magic Number */
#define EXT4_SUPER_MAGIC        0xEF53

/* EXT4 Superblock Offsets */
#define EXT4_SUPERBLOCK_OFFSET  1024
#define EXT4_SUPERBLOCK_SIZE    1024

/* EXT4 Inode Constants */
#define EXT4_ROOT_INODE         2
#define EXT4_GOOD_OLD_INODE_SIZE 128

/* EXT4 File Types */
#define EXT4_FT_UNKNOWN         0
#define EXT4_FT_REG_FILE        1
#define EXT4_FT_DIR             2
#define EXT4_FT_CHRDEV          3
#define EXT4_FT_BLKDEV          4
#define EXT4_FT_FIFO            5
#define EXT4_FT_SOCK            6
#define EXT4_FT_SYMLINK         7

/* EXT4 Inode Flags */
#define EXT4_SECRM_FL           0x00000001
#define EXT4_UNRM_FL            0x00000002
#define EXT4_COMPR_FL           0x00000004
#define EXT4_SYNC_FL            0x00000008
#define EXT4_IMMUTABLE_FL       0x00000010
#define EXT4_APPEND_FL          0x00000020
#define EXT4_NODUMP_FL          0x00000040
#define EXT4_NOATIME_FL         0x00000080
#define EXT4_DIRTY_FL           0x00000100
#define EXT4_COMPRBLK_FL        0x00000200
#define EXT4_NOCOMPR_FL         0x00000400
#define EXT4_ECOMPR_FL          0x00000800
#define EXT4_INDEX_FL           0x00001000
#define EXT4_IMAGIC_FL          0x00002000
#define EXT4_JOURNAL_DATA_FL    0x00004000
#define EXT4_NOTAIL_FL          0x00008000
#define EXT4_DIRSYNC_FL         0x00010000
#define EXT4_TOPDIR_FL          0x00020000
#define EXT4_HUGE_FILE_FL       0x00040000
#define EXT4_EXTENTS_FL         0x00080000
#define EXT4_EA_INODE_FL        0x00200000
#define EXT4_EOFBLOCKS_FL       0x00400000
#define EXT4_INLINE_DATA_FL     0x10000000

/* EXT4 Feature Flags */
#define EXT4_FEATURE_COMPAT_DIR_PREALLOC    0x0001
#define EXT4_FEATURE_COMPAT_IMAGIC_INODES   0x0002
#define EXT4_FEATURE_COMPAT_HAS_JOURNAL     0x0004
#define EXT4_FEATURE_COMPAT_EXT_ATTR        0x0008
#define EXT4_FEATURE_COMPAT_RESIZE_INODE    0x0010
#define EXT4_FEATURE_COMPAT_DIR_INDEX       0x0020

#define EXT4_FEATURE_INCOMPAT_COMPRESSION   0x0001
#define EXT4_FEATURE_INCOMPAT_FILETYPE      0x0002
#define EXT4_FEATURE_INCOMPAT_RECOVER       0x0004
#define EXT4_FEATURE_INCOMPAT_JOURNAL_DEV   0x0008
#define EXT4_FEATURE_INCOMPAT_META_BG       0x0010
#define EXT4_FEATURE_INCOMPAT_EXTENTS       0x0040
#define EXT4_FEATURE_INCOMPAT_64BIT         0x0080
#define EXT4_FEATURE_INCOMPAT_MMP           0x0100
#define EXT4_FEATURE_INCOMPAT_FLEX_BG       0x0200
#define EXT4_FEATURE_INCOMPAT_EA_INODE      0x0400
#define EXT4_FEATURE_INCOMPAT_DIRDATA       0x1000
#define EXT4_FEATURE_INCOMPAT_CSUM_SEED     0x2000
#define EXT4_FEATURE_INCOMPAT_LARGEDIR      0x4000
#define EXT4_FEATURE_INCOMPAT_INLINE_DATA   0x8000
#define EXT4_FEATURE_INCOMPAT_ENCRYPT       0x10000

/* Journal (JBD2) Constants */
#define JBD2_MAGIC_NUMBER       0xC03B3998
#define JBD2_DESCRIPTOR_BLOCK   1
#define JBD2_COMMIT_BLOCK       2
#define JBD2_SUPERBLOCK_V1      3
#define JBD2_SUPERBLOCK_V2      4
#define JBD2_REVOKE_BLOCK       5

/* Journal Flags */
#define JBD2_FLAG_ESCAPE        0x01
#define JBD2_FLAG_SAME_UUID     0x02
#define JBD2_FLAG_DELETED       0x04
#define JBD2_FLAG_LAST_TAG      0x08

/* Error Codes */
#define EXT4_SUCCESS            0
#define EXT4_ERR_INVALID_MAGIC  -1
#define EXT4_ERR_IO             -2
#define EXT4_ERR_NOMEM          -3
#define EXT4_ERR_NOTFOUND       -4
#define EXT4_ERR_INVALID        -5
#define EXT4_ERR_JOURNAL        -6
#define EXT4_ERR_NOSPC          -7
#define EXT4_ERR_PERM           -8

/* Maximum Values */
#define EXT4_MAX_BLOCK_SIZE     65536
#define EXT4_MIN_BLOCK_SIZE     1024
#define EXT4_NAME_LEN           255
#define EXT4_NDIR_BLOCKS        12
#define EXT4_IND_BLOCK          12
#define EXT4_DIND_BLOCK         13
#define EXT4_TIND_BLOCK         14
#define EXT4_N_BLOCKS           15

/* Extent Tree Constants */
#define EXT4_EXT_MAGIC          0xF30A
#define EXT4_EXT_MAX_DEPTH      5

/**
 * EXT4 Superblock Structure
 */
typedef struct __attribute__((packed)) {
    uint32_t s_inodes_count;
    uint32_t s_blocks_count_lo;
    uint32_t s_r_blocks_count_lo;
    uint32_t s_free_blocks_count_lo;
    uint32_t s_free_inodes_count;
    uint32_t s_first_data_block;
    uint32_t s_log_block_size;
    uint32_t s_log_cluster_size;
    uint32_t s_blocks_per_group;
    uint32_t s_clusters_per_group;
    uint32_t s_inodes_per_group;
    uint32_t s_mtime;
    uint32_t s_wtime;
    uint16_t s_mnt_count;
    uint16_t s_max_mnt_count;
    uint16_t s_magic;
    uint16_t s_state;
    uint16_t s_errors;
    uint16_t s_minor_rev_level;
    uint32_t s_lastcheck;
    uint32_t s_checkinterval;
    uint32_t s_creator_os;
    uint32_t s_rev_level;
    uint16_t s_def_resuid;
    uint16_t s_def_resgid;
    /* EXT4 specific fields */
    uint32_t s_first_ino;
    uint16_t s_inode_size;
    uint16_t s_block_group_nr;
    uint32_t s_feature_compat;
    uint32_t s_feature_incompat;
    uint32_t s_feature_ro_compat;
    uint8_t  s_uuid[16];
    char     s_volume_name[16];
    char     s_last_mounted[64];
    uint32_t s_algorithm_usage_bitmap;
    uint8_t  s_prealloc_blocks;
    uint8_t  s_prealloc_dir_blocks;
    uint16_t s_reserved_gdt_blocks;
    /* Journal fields */
    uint8_t  s_journal_uuid[16];
    uint32_t s_journal_inum;
    uint32_t s_journal_dev;
    uint32_t s_last_orphan;
    uint32_t s_hash_seed[4];
    uint8_t  s_def_hash_version;
    uint8_t  s_jnl_backup_type;
    uint16_t s_desc_size;
    uint32_t s_default_mount_opts;
    uint32_t s_first_meta_bg;
    uint32_t s_mkfs_time;
    uint32_t s_jnl_blocks[17];
    /* 64-bit support */
    uint32_t s_blocks_count_hi;
    uint32_t s_r_blocks_count_hi;
    uint32_t s_free_blocks_count_hi;
    uint16_t s_min_extra_isize;
    uint16_t s_want_extra_isize;
    uint32_t s_flags;
    uint16_t s_raid_stride;
    uint16_t s_mmp_interval;
    uint64_t s_mmp_block;
    uint32_t s_raid_stripe_width;
    uint8_t  s_log_groups_per_flex;
    uint8_t  s_checksum_type;
    uint16_t s_reserved_pad;
    uint64_t s_kbytes_written;
    uint32_t s_snapshot_inum;
    uint32_t s_snapshot_id;
    uint64_t s_snapshot_r_blocks_count;
    uint32_t s_snapshot_list;
    uint32_t s_error_count;
    uint32_t s_first_error_time;
    uint32_t s_first_error_ino;
    uint64_t s_first_error_block;
    uint8_t  s_first_error_func[32];
    uint32_t s_first_error_line;
    uint32_t s_last_error_time;
    uint32_t s_last_error_ino;
    uint32_t s_last_error_line;
    uint64_t s_last_error_block;
    uint8_t  s_last_error_func[32];
    uint8_t  s_mount_opts[64];
    uint32_t s_usr_quota_inum;
    uint32_t s_grp_quota_inum;
    uint32_t s_overhead_blocks;
    uint32_t s_backup_bgs[2];
    uint8_t  s_encrypt_algos[4];
    uint8_t  s_encrypt_pw_salt[16];
    uint32_t s_lpf_ino;
    uint32_t s_prj_quota_inum;
    uint32_t s_checksum_seed;
    uint32_t s_reserved[98];
    uint32_t s_checksum;
} ext4_superblock_t;

/**
 * EXT4 Block Group Descriptor (64-bit version)
 */
typedef struct __attribute__((packed)) {
    uint32_t bg_block_bitmap_lo;
    uint32_t bg_inode_bitmap_lo;
    uint32_t bg_inode_table_lo;
    uint16_t bg_free_blocks_count_lo;
    uint16_t bg_free_inodes_count_lo;
    uint16_t bg_used_dirs_count_lo;
    uint16_t bg_flags;
    uint32_t bg_exclude_bitmap_lo;
    uint16_t bg_block_bitmap_csum_lo;
    uint16_t bg_inode_bitmap_csum_lo;
    uint16_t bg_itable_unused_lo;
    uint16_t bg_checksum;
    /* 64-bit fields */
    uint32_t bg_block_bitmap_hi;
    uint32_t bg_inode_bitmap_hi;
    uint32_t bg_inode_table_hi;
    uint16_t bg_free_blocks_count_hi;
    uint16_t bg_free_inodes_count_hi;
    uint16_t bg_used_dirs_count_hi;
    uint16_t bg_itable_unused_hi;
    uint32_t bg_exclude_bitmap_hi;
    uint16_t bg_block_bitmap_csum_hi;
    uint16_t bg_inode_bitmap_csum_hi;
    uint32_t bg_reserved;
} ext4_group_desc_t;

/**
 * EXT4 Inode Structure
 */
typedef struct __attribute__((packed)) {
    uint16_t i_mode;
    uint16_t i_uid;
    uint32_t i_size_lo;
    uint32_t i_atime;
    uint32_t i_ctime;
    uint32_t i_mtime;
    uint32_t i_dtime;
    uint16_t i_gid;
    uint16_t i_links_count;
    uint32_t i_blocks_lo;
    uint32_t i_flags;
    uint32_t i_osd1;
    uint32_t i_block[EXT4_N_BLOCKS];
    uint32_t i_generation;
    uint32_t i_file_acl_lo;
    uint32_t i_size_hi;
    uint32_t i_obso_faddr;
    uint16_t i_blocks_hi;
    uint16_t i_file_acl_hi;
    uint16_t i_uid_hi;
    uint16_t i_gid_hi;
    uint16_t i_checksum_lo;
    uint16_t i_reserved;
    uint16_t i_extra_isize;
    uint16_t i_checksum_hi;
    uint32_t i_ctime_extra;
    uint32_t i_mtime_extra;
    uint32_t i_atime_extra;
    uint32_t i_crtime;
    uint32_t i_crtime_extra;
    uint32_t i_version_hi;
    uint32_t i_projid;
} ext4_inode_t;

/**
 * EXT4 Directory Entry Structure
 */
typedef struct __attribute__((packed)) {
    uint32_t inode;
    uint16_t rec_len;
    uint8_t  name_len;
    uint8_t  file_type;
    char     name[EXT4_NAME_LEN];
} ext4_dir_entry_t;

/**
 * EXT4 Extent Header
 */
typedef struct __attribute__((packed)) {
    uint16_t eh_magic;
    uint16_t eh_entries;
    uint16_t eh_max;
    uint16_t eh_depth;
    uint32_t eh_generation;
} ext4_extent_header_t;

/**
 * EXT4 Extent Index (for internal nodes)
 */
typedef struct __attribute__((packed)) {
    uint32_t ei_block;
    uint32_t ei_leaf_lo;
    uint16_t ei_leaf_hi;
    uint16_t ei_unused;
} ext4_extent_idx_t;

/**
 * EXT4 Extent (leaf node)
 */
typedef struct __attribute__((packed)) {
    uint32_t ee_block;
    uint16_t ee_len;
    uint16_t ee_start_hi;
    uint32_t ee_start_lo;
} ext4_extent_t;

/**
 * JBD2 Journal Header
 */
typedef struct __attribute__((packed)) {
    uint32_t h_magic;
    uint32_t h_blocktype;
    uint32_t h_sequence;
} jbd2_header_t;

/**
 * JBD2 Journal Superblock
 */
typedef struct __attribute__((packed)) {
    jbd2_header_t s_header;
    uint32_t s_blocksize;
    uint32_t s_maxlen;
    uint32_t s_first;
    uint32_t s_sequence;
    uint32_t s_start;
    uint32_t s_errno;
    /* V2 fields */
    uint32_t s_feature_compat;
    uint32_t s_feature_incompat;
    uint32_t s_feature_ro_compat;
    uint8_t  s_uuid[16];
    uint32_t s_nr_users;
    uint32_t s_dynsuper;
    uint32_t s_max_transaction;
    uint32_t s_max_trans_data;
    uint8_t  s_checksum_type;
    uint8_t  s_padding2[3];
    uint32_t s_padding[42];
    uint32_t s_checksum;
    uint8_t  s_users[16 * 48];
} jbd2_superblock_t;

/**
 * JBD2 Block Tag (for descriptor blocks)
 */
typedef struct __attribute__((packed)) {
    uint32_t t_blocknr;
    uint16_t t_checksum;
    uint16_t t_flags;
    uint32_t t_blocknr_hi;
} jbd2_block_tag_t;

/**
 * JBD2 Revoke Header
 */
typedef struct __attribute__((packed)) {
    jbd2_header_t r_header;
    uint32_t r_count;
} jbd2_revoke_header_t;

/**
 * Journal Transaction Structure
 */
typedef struct ext4_transaction {
    uint32_t tid;
    uint32_t state;
    uint32_t block_count;
    uint64_t *blocks;
    void **data;
    struct ext4_transaction *next;
} ext4_transaction_t;

/**
 * Journal Structure
 */
typedef struct {
    jbd2_superblock_t *superblock;
    uint64_t journal_start;
    uint32_t journal_length;
    uint32_t current_sequence;
    uint32_t head;
    uint32_t tail;
    bool running;
    ext4_transaction_t *current_transaction;
    ext4_transaction_t *committing_transaction;
} ext4_journal_t;

/**
 * Block Device Operations (abstract interface)
 */
typedef struct {
    int (*read_block)(void *dev, uint64_t block, void *buffer, size_t count);
    int (*write_block)(void *dev, uint64_t block, const void *buffer, size_t count);
    int (*sync)(void *dev);
} ext4_block_ops_t;

/**
 * EXT4 Filesystem Context
 */
typedef struct {
    void *device;
    ext4_block_ops_t *ops;
    ext4_superblock_t *superblock;
    ext4_group_desc_t *group_descs;
    ext4_journal_t *journal;
    uint32_t block_size;
    uint32_t blocks_per_group;
    uint32_t inodes_per_group;
    uint32_t inode_size;
    uint32_t group_count;
    uint32_t desc_size;
    uint64_t total_blocks;
    bool readonly;
    bool use_extents;
    bool use_64bit;
} ext4_fs_t;

/**
 * File Handle Structure
 */
typedef struct {
    ext4_fs_t *fs;
    uint32_t inode_num;
    ext4_inode_t inode;
    uint64_t position;
    uint32_t flags;
} ext4_file_t;

/**
 * Directory Iterator Structure
 */
typedef struct {
    ext4_fs_t *fs;
    uint32_t inode_num;
    ext4_inode_t inode;
    uint64_t position;
    uint64_t size;
} ext4_dir_iterator_t;

/* Forward declarations */
static int ext4_read_superblock(ext4_fs_t *fs);
static int ext4_read_group_descriptors(ext4_fs_t *fs);
static int ext4_read_inode(ext4_fs_t *fs, uint32_t inode_num, ext4_inode_t *inode);
static int ext4_write_inode(ext4_fs_t *fs, uint32_t inode_num, const ext4_inode_t *inode);
static int ext4_read_block(ext4_fs_t *fs, uint64_t block, void *buffer);
static int ext4_write_block(ext4_fs_t *fs, uint64_t block, const void *buffer);
static uint64_t ext4_get_block_from_extent(ext4_fs_t *fs, ext4_inode_t *inode, uint64_t logical_block);
static uint64_t ext4_get_block_indirect(ext4_fs_t *fs, ext4_inode_t *inode, uint64_t logical_block);
static int ext4_journal_init(ext4_fs_t *fs);
static int ext4_journal_start_transaction(ext4_fs_t *fs);
static int ext4_journal_commit_transaction(ext4_fs_t *fs);
static int ext4_journal_add_block(ext4_fs_t *fs, uint64_t block, const void *data);
static int ext4_journal_recover(ext4_fs_t *fs);

/* Memory allocation stubs - implement according to your kernel */
extern void *kmalloc(size_t size);
extern void kfree(void *ptr);
extern void *memset(void *s, int c, size_t n);
extern void *memcpy(void *dest, const void *src, size_t n);
extern int memcmp(const void *s1, const void *s2, size_t n);
extern size_t strlen(const char *s);
extern int strncmp(const char *s1, const char *s2, size_t n);

/**
 * Calculate CRC32C checksum (used by EXT4 metadata checksums)
 */
static uint32_t ext4_crc32c(uint32_t crc, const void *buf, size_t len) {
    static const uint32_t crc32c_table[256] = {
        0x00000000, 0xF26B8303, 0xE13B70F7, 0x1350F3F4,
        0xC79A971F, 0x3548DC1C, 0x2618D0E8, 0xD4E353EB,
        /* ... full table would be here ... */
    };
    
    const uint8_t *p = (const uint8_t *)buf;
    crc = ~crc;
    while (len--) {
        crc = crc32c_table[(crc ^ *p++) & 0xFF] ^ (crc >> 8);
    }
    return ~crc;
}

/**
 * Read a block from the device
 */
static int ext4_read_block(ext4_fs_t *fs, uint64_t block, void *buffer) {
    if (!fs || !fs->ops || !fs->ops->read_block || !buffer) {
        return EXT4_ERR_INVALID;
    }
    return fs->ops->read_block(fs->device, block, buffer, fs->block_size);
}

/**
 * Write a block to the device
 */
static int ext4_write_block(ext4_fs_t *fs, uint64_t block, const void *buffer) {
    if (!fs || !fs->ops || !fs->ops->write_block || !buffer) {
        return EXT4_ERR_INVALID;
    }
    if (fs->readonly) {
        return EXT4_ERR_PERM;
    }
    return fs->ops->write_block(fs->device, block, buffer, fs->block_size);
}

/**
 * Read the superblock from the device
 */
static int ext4_read_superblock(ext4_fs_t *fs) {
    uint8_t buffer[EXT4_SUPERBLOCK_SIZE];
    int ret;
    
    /* Superblock is at offset 1024 bytes (block 0 for 1K blocks, or within block 0 for larger) */
    ret = fs->ops->read_block(fs->device, 0, buffer, EXT4_MIN_BLOCK_SIZE * 2);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    
    fs->superblock = kmalloc(sizeof(ext4_superblock_t));
    if (!fs->superblock) {
        return EXT4_ERR_NOMEM;
    }
    
    memcpy(fs->superblock, buffer + EXT4_SUPERBLOCK_OFFSET, sizeof(ext4_superblock_t));
    
    /* Validate magic number */
    if (fs->superblock->s_magic != EXT4_SUPER_MAGIC) {
        kfree(fs->superblock);
        fs->superblock = NULL;
        return EXT4_ERR_INVALID_MAGIC;
    }
    
    /* Calculate filesystem parameters */
    fs->block_size = EXT4_MIN_BLOCK_SIZE << fs->superblock->s_log_block_size;
    fs->blocks_per_group = fs->superblock->s_blocks_per_group;
    fs->inodes_per_group = fs->superblock->s_inodes_per_group;
    fs->inode_size = fs->superblock->s_inode_size;
    
    /* Handle 64-bit block counts */
    if (fs->superblock->s_feature_incompat & EXT4_FEATURE_INCOMPAT_64BIT) {
        fs->use_64bit = true;
        fs->total_blocks = ((uint64_t)fs->superblock->s_blocks_count_hi << 32) | 
                           fs->superblock->s_blocks_count_lo;
        fs->desc_size = fs->superblock->s_desc_size;
        if (fs->desc_size < 32) {
            fs->desc_size = 32;
        }
    } else {
        fs->use_64bit = false;
        fs->total_blocks = fs->superblock->s_blocks_count_lo;
        fs->desc_size = 32;
    }
    
    /* Calculate group count */
    fs->group_count = (fs->total_blocks + fs->blocks_per_group - 1) / fs->blocks_per_group;
    
    /* Check for extent support */
    fs->use_extents = (fs->superblock->s_feature_incompat & EXT4_FEATURE_INCOMPAT_EXTENTS) != 0;
    
    return EXT4_SUCCESS;
}

/**
 * Read block group descriptors
 */
static int ext4_read_group_descriptors(ext4_fs_t *fs) {
    uint32_t desc_per_block;
    uint32_t desc_blocks;
    uint64_t desc_start;
    uint8_t *buffer;
    int ret;
    
    desc_per_block = fs->block_size / fs->desc_size;
    desc_blocks = (fs->group_count + desc_per_block - 1) / desc_per_block;
    
    fs->group_descs = kmalloc(fs->group_count * sizeof(ext4_group_desc_t));
    if (!fs->group_descs) {
        return EXT4_ERR_NOMEM;
    }
    
    buffer = kmalloc(fs->block_size);
    if (!buffer) {
        kfree(fs->group_descs);
        fs->group_descs = NULL;
        return EXT4_ERR_NOMEM;
    }
    
    /* Group descriptors start at block 1 (for 1K blocks) or block 1 (for larger blocks) */
    desc_start = (fs->block_size == EXT4_MIN_BLOCK_SIZE) ? 2 : 1;
    
    for (uint32_t i = 0; i < desc_blocks; i++) {
        ret = ext4_read_block(fs, desc_start + i, buffer);
        if (ret != EXT4_SUCCESS) {
            kfree(buffer);
            kfree(fs->group_descs);
            fs->group_descs = NULL;
            return ret;
        }
        
        for (uint32_t j = 0; j < desc_per_block && (i * desc_per_block + j) < fs->group_count; j++) {
            memcpy(&fs->group_descs[i * desc_per_block + j], 
                   buffer + j * fs->desc_size, 
                   sizeof(ext4_group_desc_t));
        }
    }
    
    kfree(buffer);
    return EXT4_SUCCESS;
}

/**
 * Read an inode from the filesystem
 */
static int ext4_read_inode(ext4_fs_t *fs, uint32_t inode_num, ext4_inode_t *inode) {
    uint32_t group;
    uint32_t index;
    uint64_t inode_table;
    uint64_t block;
    uint32_t offset;
    uint8_t *buffer;
    int ret;
    
    if (!fs || !inode || inode_num == 0) {
        return EXT4_ERR_INVALID;
    }
    
    /* Calculate which block group contains this inode */
    group = (inode_num - 1) / fs->inodes_per_group;
    index = (inode_num - 1) % fs->inodes_per_group;
    
    if (group >= fs->group_count) {
        return EXT4_ERR_INVALID;
    }
    
    /* Get inode table location */
    inode_table = fs->group_descs[group].bg_inode_table_lo;
    if (fs->use_64bit) {
        inode_table |= ((uint64_t)fs->group_descs[group].bg_inode_table_hi << 32);
    }
    
    /* Calculate block and offset within block */
    block = inode_table + (index * fs->inode_size) / fs->block_size;
    offset = (index * fs->inode_size) % fs->block_size;
    
    buffer = kmalloc(fs->block_size);
    if (!buffer) {
        return EXT4_ERR_NOMEM;
    }
    
    ret = ext4_read_block(fs, block, buffer);
    if (ret != EXT4_SUCCESS) {
        kfree(buffer);
        return ret;
    }
    
    memcpy(inode, buffer + offset, sizeof(ext4_inode_t));
    kfree(buffer);
    
    return EXT4_SUCCESS;
}

/**
 * Write an inode to the filesystem
 */
static int ext4_write_inode(ext4_fs_t *fs, uint32_t inode_num, const ext4_inode_t *inode) {
    uint32_t group;
    uint32_t index;
    uint64_t inode_table;
    uint64_t block;
    uint32_t offset;
    uint8_t *buffer;
    int ret;
    
    if (!fs || !inode || inode_num == 0 || fs->readonly) {
        return EXT4_ERR_INVALID;
    }
    
    group = (inode_num - 1) / fs->inodes_per_group;
    index = (inode_num - 1) % fs->inodes_per_group;
    
    if (group >= fs->group_count) {
        return EXT4_ERR_INVALID;
    }
    
    inode_table = fs->group_descs[group].bg_inode_table_lo;
    if (fs->use_64bit) {
        inode_table |= ((uint64_t)fs->group_descs[group].bg_inode_table_hi << 32);
    }
    
    block = inode_table + (index * fs->inode_size) / fs->block_size;
    offset = (index * fs->inode_size) % fs->block_size;
    
    buffer = kmalloc(fs->block_size);
    if (!buffer) {
        return EXT4_ERR_NOMEM;
    }
    
    /* Read the block first */
    ret = ext4_read_block(fs, block, buffer);
    if (ret != EXT4_SUCCESS) {
        kfree(buffer);
        return ret;
    }
    
    /* Update the inode */
    memcpy(buffer + offset, inode, sizeof(ext4_inode_t));
    
    /* Write back with journaling */
    ret = ext4_journal_add_block(fs, block, buffer);
    if (ret != EXT4_SUCCESS) {
        /* Fallback to direct write */
        ret = ext4_write_block(fs, block, buffer);
    }
    
    kfree(buffer);
    return ret;
}

/**
 * Get physical block from extent tree
 */
static uint64_t ext4_get_block_from_extent(ext4_fs_t *fs, ext4_inode_t *inode, uint64_t logical_block) {
    ext4_extent_header_t *header;
    ext4_extent_t *extent;
    ext4_extent_idx_t *idx;
    uint8_t *buffer = NULL;
    uint64_t physical_block;
    int depth;
    
    header = (ext4_extent_header_t *)inode->i_block;
    
    if (header->eh_magic != EXT4_EXT_MAGIC) {
        return 0;
    }
    
    depth = header->eh_depth;
    
    while (depth > 0) {
        /* Internal node - find the right child */
        idx = (ext4_extent_idx_t *)(header + 1);
        
        ext4_extent_idx_t *best = NULL;
        for (int i = 0; i < header->eh_entries; i++) {
            if (idx[i].ei_block <= logical_block) {
                best = &idx[i];
            } else {
                break;
            }
        }
        
        if (!best) {
            if (buffer) kfree(buffer);
            return 0;
        }
        
        /* Read the child block */
        physical_block = best->ei_leaf_lo | ((uint64_t)best->ei_leaf_hi << 32);
        
        if (!buffer) {
            buffer = kmalloc(fs->block_size);
            if (!buffer) {
                return 0;
            }
        }
        
        if (ext4_read_block(fs, physical_block, buffer) != EXT4_SUCCESS) {
            kfree(buffer);
            return 0;
        }
        
        header = (ext4_extent_header_t *)buffer;
        depth--;
    }
    
    /* Leaf node - find the extent */
    extent = (ext4_extent_t *)(header + 1);
    
    for (int i = 0; i < header->eh_entries; i++) {
        uint32_t ee_block = extent[i].ee_block;
        uint16_t ee_len = extent[i].ee_len & 0x7FFF; /* Mask uninitialized flag */
        
        if (logical_block >= ee_block && logical_block < ee_block + ee_len) {
            physical_block = extent[i].ee_start_lo | 
                            ((uint64_t)extent[i].ee_start_hi << 32);
            physical_block += (logical_block - ee_block);
            
            if (buffer) kfree(buffer);
            return physical_block;
        }
    }
    
    if (buffer) kfree(buffer);
    return 0;
}

/**
 * Get physical block using indirect block mapping (legacy)
 */
static uint64_t ext4_get_block_indirect(ext4_fs_t *fs, ext4_inode_t *inode, uint64_t logical_block) {
    uint32_t ptrs_per_block = fs->block_size / sizeof(uint32_t);
    uint32_t *buffer;
    uint64_t physical_block;
    
    /* Direct blocks */
    if (logical_block < EXT4_NDIR_BLOCKS) {
        return inode->i_block[logical_block];
    }
    
    logical_block -= EXT4_NDIR_BLOCKS;
    
    buffer = kmalloc(fs->block_size);
    if (!buffer) {
        return 0;
    }
    
    /* Single indirect */
    if (logical_block < ptrs_per_block) {
        if (ext4_read_block(fs, inode->i_block[EXT4_IND_BLOCK], buffer) != EXT4_SUCCESS) {
            kfree(buffer);
            return 0;
        }
        physical_block = buffer[logical_block];
        kfree(buffer);
        return physical_block;
    }
    
    logical_block -= ptrs_per_block;
    
    /* Double indirect */
    if (logical_block < ptrs_per_block * ptrs_per_block) {
        if (ext4_read_block(fs, inode->i_block[EXT4_DIND_BLOCK], buffer) != EXT4_SUCCESS) {
            kfree(buffer);
            return 0;
        }
        
        uint32_t ind_block = buffer[logical_block / ptrs_per_block];
        if (ext4_read_block(fs, ind_block, buffer) != EXT4_SUCCESS) {
            kfree(buffer);
            return 0;
        }
        
        physical_block = buffer[logical_block % ptrs_per_block];
        kfree(buffer);
        return physical_block;
    }
    
    logical_block -= ptrs_per_block * ptrs_per_block;
    
    /* Triple indirect */
    if (ext4_read_block(fs, inode->i_block[EXT4_TIND_BLOCK], buffer) != EXT4_SUCCESS) {
        kfree(buffer);
        return 0;
    }
    
    uint32_t dind_block = buffer[logical_block / (ptrs_per_block * ptrs_per_block)];
    if (ext4_read_block(fs, dind_block, buffer) != EXT4_SUCCESS) {
        kfree(buffer);
        return 0;
    }
    
    uint32_t ind_block = buffer[(logical_block / ptrs_per_block) % ptrs_per_block];
    if (ext4_read_block(fs, ind_block, buffer) != EXT4_SUCCESS) {
        kfree(buffer);
        return 0;
    }
    
    physical_block = buffer[logical_block % ptrs_per_block];
    kfree(buffer);
    return physical_block;
}

/**
 * Get physical block number for a logical block in an inode
 */
static uint64_t ext4_get_block(ext4_fs_t *fs, ext4_inode_t *inode, uint64_t logical_block) {
    if (inode->i_flags & EXT4_EXTENTS_FL) {
        return ext4_get_block_from_extent(fs, inode, logical_block);
    } else {
        return ext4_get_block_indirect(fs, inode, logical_block);
    }
}

/* ============== JOURNAL (JBD2) IMPLEMENTATION ============== */

/**
 * Initialize the journal
 */
static int ext4_journal_init(ext4_fs_t *fs) {
    ext4_inode_t journal_inode;
    uint8_t *buffer;
    int ret;
    
    if (!(fs->superblock->s_feature_compat & EXT4_FEATURE_COMPAT_HAS_JOURNAL)) {
        fs->journal = NULL;
        return EXT4_SUCCESS;
    }
    
    fs->journal = kmalloc(sizeof(ext4_journal_t));
    if (!fs->journal) {
        return EXT4_ERR_NOMEM;
    }
    memset(fs->journal, 0, sizeof(ext4_journal_t));
    
    /* Read journal inode */
    ret = ext4_read_inode(fs, fs->superblock->s_journal_inum, &journal_inode);
    if (ret != EXT4_SUCCESS) {
        kfree(fs->journal);
        fs->journal = NULL;
        return ret;
    }
    
    /* Get first block of journal */
    fs->journal->journal_start = ext4_get_block(fs, &journal_inode, 0);
    if (fs->journal->journal_start == 0) {
        kfree(fs->journal);
        fs->journal = NULL;
        return EXT4_ERR_JOURNAL;
    }
    
    /* Read journal superblock */
    buffer = kmalloc(fs->block_size);
    if (!buffer) {
        kfree(fs->journal);
        fs->journal = NULL;
        return EXT4_ERR_NOMEM;
    }
    
    ret = ext4_read_block(fs, fs->journal->journal_start, buffer);
    if (ret != EXT4_SUCCESS) {
        kfree(buffer);
        kfree(fs->journal);
        fs->journal = NULL;
        return ret;
    }
    
    fs->journal->superblock = kmalloc(sizeof(jbd2_superblock_t));
    if (!fs->journal->superblock) {
        kfree(buffer);
        kfree(fs->journal);
        fs->journal = NULL;
        return EXT4_ERR_NOMEM;
    }
    
    memcpy(fs->journal->superblock, buffer, sizeof(jbd2_superblock_t));
    kfree(buffer);
    
    /* Validate journal magic */
    if (fs->journal->superblock->s_header.h_magic != JBD2_MAGIC_NUMBER) {
        kfree(fs->journal->superblock);
        kfree(fs->journal);
        fs->journal = NULL;
        return EXT4_ERR_JOURNAL;
    }
    
    fs->journal->journal_length = fs->journal->superblock->s_maxlen;
    fs->journal->current_sequence = fs->journal->superblock->s_sequence;
    fs->journal->head = fs->journal->superblock->s_start;
    fs->journal->tail = fs->journal->superblock->s_start;
    fs->journal->running = true;
    
    /* Recover if needed */
    if (fs->superblock->s_feature_incompat & EXT4_FEATURE_INCOMPAT_RECOVER) {
        ret = ext4_journal_recover(fs);
        if (ret != EXT4_SUCCESS) {
            /* Journal recovery failed - mount read-only */
            fs->readonly = true;
        }
    }
    
    return EXT4_SUCCESS;
}

/**
 * Start a new journal transaction
 */
static int ext4_journal_start_transaction(ext4_fs_t *fs) {
    if (!fs->journal || !fs->journal->running) {
        return EXT4_SUCCESS; /* No journal or journal not active */
    }
    
    if (fs->journal->current_transaction) {
        /* Transaction already in progress */
        return EXT4_SUCCESS;
    }
    
    fs->journal->current_transaction = kmalloc(sizeof(ext4_transaction_t));
    if (!fs->journal->current_transaction) {
        return EXT4_ERR_NOMEM;
    }
    
    memset(fs->journal->current_transaction, 0, sizeof(ext4_transaction_t));
    fs->journal->current_transaction->tid = fs->journal->current_sequence++;
    fs->journal->current_transaction->state = 0; /* Running */
    
    return EXT4_SUCCESS;
}

/**
 * Add a block to the current transaction
 */
static int ext4_journal_add_block(ext4_fs_t *fs, uint64_t block, const void *data) {
    ext4_transaction_t *trans;
    uint64_t *new_blocks;
    void **new_data;
    void *block_copy;
    
    if (!fs->journal || !fs->journal->running) {
        /* No journal - write directly */
        return ext4_write_block(fs, block, data);
    }
    
    if (!fs->journal->current_transaction) {
        int ret = ext4_journal_start_transaction(fs);
        if (ret != EXT4_SUCCESS) {
            return ret;
        }
    }
    
    trans = fs->journal->current_transaction;
    
    /* Allocate space for new block entry */
    new_blocks = kmalloc((trans->block_count + 1) * sizeof(uint64_t));
    new_data = kmalloc((trans->block_count + 1) * sizeof(void *));
    block_copy = kmalloc(fs->block_size);
    
    if (!new_blocks || !new_data || !block_copy) {
        if (new_blocks) kfree(new_blocks);
        if (new_data) kfree(new_data);
        if (block_copy) kfree(block_copy);
        return EXT4_ERR_NOMEM;
    }
    
    /* Copy existing entries */
    if (trans->block_count > 0) {
        memcpy(new_blocks, trans->blocks, trans->block_count * sizeof(uint64_t));
        memcpy(new_data, trans->data, trans->block_count * sizeof(void *));
        kfree(trans->blocks);
        kfree(trans->data);
    }
    
    /* Add new entry */
    memcpy(block_copy, data, fs->block_size);
    new_blocks[trans->block_count] = block;
    new_data[trans->block_count] = block_copy;
    
    trans->blocks = new_blocks;
    trans->data = new_data;
    trans->block_count++;
    
    return EXT4_SUCCESS;
}

/**
 * Write journal descriptor block
 */
static int ext4_journal_write_descriptor(ext4_fs_t *fs, uint32_t sequence, 
                                          uint64_t *blocks, uint32_t count, 
                                          uint64_t journal_block) {
    uint8_t *buffer;
    jbd2_header_t *header;
    jbd2_block_tag_t *tag;
    int ret;
    
    buffer = kmalloc(fs->block_size);
    if (!buffer) {
        return EXT4_ERR_NOMEM;
    }
    
    memset(buffer, 0, fs->block_size);
    
    header = (jbd2_header_t *)buffer;
    header->h_magic = JBD2_MAGIC_NUMBER;
    header->h_blocktype = JBD2_DESCRIPTOR_BLOCK;
    header->h_sequence = sequence;
    
    tag = (jbd2_block_tag_t *)(buffer + sizeof(jbd2_header_t));
    
    for (uint32_t i = 0; i < count; i++) {
        tag[i].t_blocknr = (uint32_t)blocks[i];
        tag[i].t_blocknr_hi = (uint32_t)(blocks[i] >> 32);
        tag[i].t_flags = (i == count - 1) ? JBD2_FLAG_LAST_TAG : 0;
        tag[i].t_checksum = 0; /* Would calculate checksum here */
    }
    
    ret = ext4_write_block(fs, fs->journal->journal_start + journal_block, buffer);
    kfree(buffer);
    
    return ret;
}

/**
 * Write journal commit block
 */
static int ext4_journal_write_commit(ext4_fs_t *fs, uint32_t sequence, uint64_t journal_block) {
    uint8_t *buffer;
    jbd2_header_t *header;
    int ret;
    
    buffer = kmalloc(fs->block_size);
    if (!buffer) {
        return EXT4_ERR_NOMEM;
    }
    
    memset(buffer, 0, fs->block_size);
    
    header = (jbd2_header_t *)buffer;
    header->h_magic = JBD2_MAGIC_NUMBER;
    header->h_blocktype = JBD2_COMMIT_BLOCK;
    header->h_sequence = sequence;
    
    ret = ext4_write_block(fs, fs->journal->journal_start + journal_block, buffer);
    kfree(buffer);
    
    return ret;
}

/**
 * Commit the current transaction
 */
static int ext4_journal_commit_transaction(ext4_fs_t *fs) {
    ext4_transaction_t *trans;
    uint64_t journal_block;
    int ret;
    
    if (!fs->journal || !fs->journal->running) {
        return EXT4_SUCCESS;
    }
    
    trans = fs->journal->current_transaction;
    if (!trans || trans->block_count == 0) {
        return EXT4_SUCCESS;
    }
    
    journal_block = fs->journal->head;
    
    /* Write descriptor block */
    ret = ext4_journal_write_descriptor(fs, trans->tid, trans->blocks, 
                                         trans->block_count, journal_block);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    journal_block++;
    
    /* Write data blocks to journal */
    for (uint32_t i = 0; i < trans->block_count; i++) {
        ret = ext4_write_block(fs, fs->journal->journal_start + journal_block, 
                               trans->data[i]);
        if (ret != EXT4_SUCCESS) {
            return ret;
        }
        journal_block++;
        
        /* Handle journal wrap-around */
        if (journal_block >= fs->journal->journal_length) {
            journal_block = 1; /* Skip superblock */
        }
    }
    
    /* Write commit block */
    ret = ext4_journal_write_commit(fs, trans->tid, journal_block);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    journal_block++;
    
    /* Now write data to actual locations */
    for (uint32_t i = 0; i < trans->block_count; i++) {
        ret = ext4_write_block(fs, trans->blocks[i], trans->data[i]);
        if (ret != EXT4_SUCCESS) {
            return ret;
        }
    }
    
    /* Sync device */
    if (fs->ops->sync) {
        fs->ops->sync(fs->device);
    }
    
    /* Update journal head */
    fs->journal->head = journal_block;
    
    /* Update journal superblock */
    fs->journal->superblock->s_sequence = fs->journal->current_sequence;
    fs->journal->superblock->s_start = fs->journal->tail;
    
    uint8_t *buffer = kmalloc(fs->block_size);
    if (buffer) {
        memset(buffer, 0, fs->block_size);
        memcpy(buffer, fs->journal->superblock, sizeof(jbd2_superblock_t));
        ext4_write_block(fs, fs->journal->journal_start, buffer);
        kfree(buffer);
    }
    
    /* Free transaction */
    for (uint32_t i = 0; i < trans->block_count; i++) {
        kfree(trans->data[i]);
    }
    kfree(trans->blocks);
    kfree(trans->data);
    kfree(trans);
    
    fs->journal->current_transaction = NULL;
    
    return EXT4_SUCCESS;
}

/**
 * Recover journal after unclean unmount
 */
static int ext4_journal_recover(ext4_fs_t *fs) {
    uint8_t *buffer;
    jbd2_header_t *header;
    uint64_t block;
    uint32_t expected_sequence;
    bool found_commit;
    int ret;
    
    if (!fs->journal) {
        return EXT4_SUCCESS;
    }
    
    buffer = kmalloc(fs->block_size);
    if (!buffer) {
        return EXT4_ERR_NOMEM;
    }
    
    block = fs->journal->superblock->s_start;
    expected_sequence = fs->journal->superblock->s_sequence;
    
    /* Scan journal for valid transactions */
    while (1) {
        ret = ext4_read_block(fs, fs->journal->journal_start + block, buffer);
        if (ret != EXT4_SUCCESS) {
            break;
        }
        
        header = (jbd2_header_t *)buffer;
        
        if (header->h_magic != JBD2_MAGIC_NUMBER) {
            break;
        }
        
        if (header->h_sequence != expected_sequence) {
            break;
        }
        
        if (header->h_blocktype == JBD2_DESCRIPTOR_BLOCK) {
            /* Found descriptor - replay data blocks */
            jbd2_block_tag_t *tag = (jbd2_block_tag_t *)(buffer + sizeof(jbd2_header_t));
            uint64_t data_block = block + 1;
            
            found_commit = false;
            
            while (1) {
                uint64_t target = tag->t_blocknr | ((uint64_t)tag->t_blocknr_hi << 32);
                
                /* Read data block from journal */
                uint8_t *data_buffer = kmalloc(fs->block_size);
                if (data_buffer) {
                    if (ext4_read_block(fs, fs->journal->journal_start + data_block, 
                                        data_buffer) == EXT4_SUCCESS) {
                        /* Replay to target location */
                        ext4_write_block(fs, target, data_buffer);
                    }
                    kfree(data_buffer);
                }
                
                data_block++;
                if (tag->t_flags & JBD2_FLAG_LAST_TAG) {
                    break;
                }
                tag++;
            }
            
            /* Check for commit block */
            ret = ext4_read_block(fs, fs->journal->journal_start + data_block, buffer);
            if (ret == EXT4_SUCCESS) {
                header = (jbd2_header_t *)buffer;
                if (header->h_magic == JBD2_MAGIC_NUMBER &&
                    header->h_blocktype == JBD2_COMMIT_BLOCK &&
                    header->h_sequence == expected_sequence) {
                    found_commit = true;
                    block = data_block + 1;
                }
            }
            
            if (!found_commit) {
                break;
            }
            
            expected_sequence++;
            
        } else if (header->h_blocktype == JBD2_REVOKE_BLOCK) {
            /* Skip revoke blocks for now */
            block++;
        } else {
            break;
        }
        
        /* Handle wrap-around */
        if (block >= fs->journal->journal_length) {
            block = 1;
        }
    }
    
    kfree(buffer);
    
    /* Clear recovery flag */
    fs->superblock->s_feature_incompat &= ~EXT4_FEATURE_INCOMPAT_RECOVER;
    
    /* Update journal superblock */
    fs->journal->superblock->s_start = 0;
    fs->journal->superblock->s_sequence = expected_sequence;
    fs->journal->head = 1;
    fs->journal->tail = 1;
    
    return EXT4_SUCCESS;
}

/**
 * Shutdown the journal
 */
static int ext4_journal_shutdown(ext4_fs_t *fs) {
    if (!fs->journal) {
        return EXT4_SUCCESS;
    }
    
    /* Commit any pending transaction */
    if (fs->journal->current_transaction) {
        ext4_journal_commit_transaction(fs);
    }
    
    fs->journal->running = false;
    
    /* Update journal superblock */
    fs->journal->superblock->s_start = 0;
    
    uint8_t *buffer = kmalloc(fs->block_size);
    if (buffer) {
        memset(buffer, 0, fs->block_size);
        memcpy(buffer, fs->journal->superblock, sizeof(jbd2_superblock_t));
        ext4_write_block(fs, fs->journal->journal_start, buffer);
        kfree(buffer);
    }
    
    kfree(fs->journal->superblock);
    kfree(fs->journal);
    fs->journal = NULL;
    
    return EXT4_SUCCESS;
}

/* ============== PUBLIC API ============== */

/**
 * Mount an EXT4 filesystem
 */
int ext4_mount(void *device, ext4_block_ops_t *ops, ext4_fs_t **fs_out) {
    ext4_fs_t *fs;
    int ret;
    
    if (!device || !ops || !fs_out) {
        return EXT4_ERR_INVALID;
    }
    
    fs = kmalloc(sizeof(ext4_fs_t));
    if (!fs) {
        return EXT4_ERR_NOMEM;
    }
    
    memset(fs, 0, sizeof(ext4_fs_t));
    fs->device = device;
    fs->ops = ops;
    fs->readonly = false;
    
    /* Read and validate superblock */
    ret = ext4_read_superblock(fs);
    if (ret != EXT4_SUCCESS) {
        kfree(fs);
        return ret;
    }
    
    /* Read group descriptors */
    ret = ext4_read_group_descriptors(fs);
    if (ret != EXT4_SUCCESS) {
        kfree(fs->superblock);
        kfree(fs);
        return ret;
    }
    
    /* Initialize journal */
    ret = ext4_journal_init(fs);
    if (ret != EXT4_SUCCESS) {
        kfree(fs->group_descs);
        kfree(fs->superblock);
        kfree(fs);
        return ret;
    }
    
    *fs_out = fs;
    return EXT4_SUCCESS;
}

/**
 * Unmount an EXT4 filesystem
 */
int ext4_unmount(ext4_fs_t *fs) {
    if (!fs) {
        return EXT4_ERR_INVALID;
    }
    
    /* Shutdown journal */
    ext4_journal_shutdown(fs);
    
    /* Sync filesystem */
    if (fs->ops->sync) {
        fs->ops->sync(fs->device);
    }
    
    /* Free resources */
    if (fs->group_descs) {
        kfree(fs->group_descs);
    }
    if (fs->superblock) {
        kfree(fs->superblock);
    }
    kfree(fs);
    
    return EXT4_SUCCESS;
}

/**
 * Open a file by path
 */
int ext4_open(ext4_fs_t *fs, const char *path, uint32_t flags, ext4_file_t **file_out) {
    ext4_file_t *file;
    ext4_inode_t inode;
    uint32_t inode_num;
    int ret;
    
    if (!fs || !path || !file_out) {
        return EXT4_ERR_INVALID;
    }
    
    /* Lookup path to get inode number */
    ret = ext4_lookup_path(fs, path, &inode_num);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    
    /* Read inode */
    ret = ext4_read_inode(fs, inode_num, &inode);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    
    /* Allocate file handle */
    file = kmalloc(sizeof(ext4_file_t));
    if (!file) {
        return EXT4_ERR_NOMEM;
    }
    
    file->fs = fs;
    file->inode_num = inode_num;
    memcpy(&file->inode, &inode, sizeof(ext4_inode_t));
    file->position = 0;
    file->flags = flags;
    
    *file_out = file;
    return EXT4_SUCCESS;
}

/**
 * Close a file
 */
int ext4_close(ext4_file_t *file) {
    if (!file) {
        return EXT4_ERR_INVALID;
    }
            
    kfree(file);
    return EXT4_SUCCESS;
}

/**
 * Read data from a file
 */
int ext4_read(ext4_file_t *file, void *buffer, size_t size, size_t *bytes_read) {
    uint64_t file_size;
    uint64_t remaining;
    uint8_t *buf = (uint8_t *)buffer;
    uint8_t *block_buffer;
    size_t total_read = 0;
    int ret;
    
    if (!file || !buffer || !bytes_read) {
        return EXT4_ERR_INVALID;
    }
    
    /* Calculate file size */
    file_size = file->inode.i_size_lo;
    if (file->fs->superblock->s_feature_ro_compat & 0x0002) { /* Large file support */
        file_size |= ((uint64_t)file->inode.i_size_hi << 32);
    }
    
    /* Check if we're at or past EOF */
    if (file->position >= file_size) {
        *bytes_read = 0;
        return EXT4_SUCCESS;
    }
    
    /* Limit read to file size */
    remaining = file_size - file->position;
    if (size > remaining) {
        size = remaining;
    }
    
    block_buffer = kmalloc(file->fs->block_size);
    if (!block_buffer) {
        return EXT4_ERR_NOMEM;
    }
    
    while (size > 0) {
        uint64_t logical_block = file->position / file->fs->block_size;
        uint32_t offset = file->position % file->fs->block_size;
        uint32_t to_read = file->fs->block_size - offset;
        
        if (to_read > size) {
            to_read = size;
        }
        
        uint64_t physical_block = ext4_get_block(file->fs, &file->inode, logical_block);
        
        if (physical_block == 0) {
            /* Sparse file - return zeros */
            memset(buf, 0, to_read);
        } else {
            ret = ext4_read_block(file->fs, physical_block, block_buffer);
            if (ret != EXT4_SUCCESS) {
                kfree(block_buffer);
                *bytes_read = total_read;
                return ret;
            }
            memcpy(buf, block_buffer + offset, to_read);
        }
        
        buf += to_read;
        file->position += to_read;
        total_read += to_read;
        size -= to_read;
    }
    
    kfree(block_buffer);
    *bytes_read = total_read;
    return EXT4_SUCCESS;
}

/**
 * Seek to a position in a file
 */
int ext4_seek(ext4_file_t *file, int64_t offset, int whence) {
    uint64_t file_size;
    int64_t new_pos;
    
    if (!file) {
        return EXT4_ERR_INVALID;
    }
    
    file_size = file->inode.i_size_lo;
    if (file->fs->superblock->s_feature_ro_compat & 0x0002) {
        file_size |= ((uint64_t)file->inode.i_size_hi << 32);
    }
    
    switch (whence) {
        case 0: /* SEEK_SET */
            new_pos = offset;
            break;
        case 1: /* SEEK_CUR */
            new_pos = (int64_t)file->position + offset;
            break;
        case 2: /* SEEK_END */
            new_pos = (int64_t)file_size + offset;
            break;
        default:
            return EXT4_ERR_INVALID;
    }
    
    if (new_pos < 0) {
        return EXT4_ERR_INVALID;
    }
    
    file->position = (uint64_t)new_pos;
    return EXT4_SUCCESS;
}

/**
 * Get current file position
 */
uint64_t ext4_tell(ext4_file_t *file) {
    if (!file) {
        return 0;
    }
    return file->position;
}

/**
 * Lookup a path and return the inode number
 */
int ext4_lookup_path(ext4_fs_t *fs, const char *path, uint32_t *inode_out) {
    ext4_inode_t inode;
    uint32_t current_inode = EXT4_ROOT_INODE;
    const char *p = path;
    char name[EXT4_NAME_LEN + 1];
    int ret;
    
    if (!fs || !path || !inode_out) {
        return EXT4_ERR_INVALID;
    }
    
    /* Skip leading slash */
    while (*p == '/') {
        p++;
    }
    
    /* Empty path or root */
    if (*p == '\0') {
        *inode_out = EXT4_ROOT_INODE;
        return EXT4_SUCCESS;
    }
    
    while (*p) {
        /* Extract next path component */
        size_t len = 0;
        while (p[len] && p[len] != '/' && len < EXT4_NAME_LEN) {
            name[len] = p[len];
            len++;
        }
        name[len] = '\0';
        
        if (len == 0) {
            p++;
            continue;
        }
        
        /* Read current directory inode */
        ret = ext4_read_inode(fs, current_inode, &inode);
        if (ret != EXT4_SUCCESS) {
            return ret;
        }
        
        /* Search directory for name */
        ret = ext4_dir_find_entry(fs, &inode, name, &current_inode);
        if (ret != EXT4_SUCCESS) {
            return ret;
        }
        
        p += len;
        while (*p == '/') {
            p++;
        }
    }
    
    *inode_out = current_inode;
    return EXT4_SUCCESS;
}

/**
 * Find an entry in a directory
 */
int ext4_dir_find_entry(ext4_fs_t *fs, ext4_inode_t *dir_inode, 
                        const char *name, uint32_t *inode_out) {
    uint64_t dir_size;
    uint64_t offset = 0;
    uint8_t *buffer;
    int ret;
    
    if (!fs || !dir_inode || !name || !inode_out) {
        return EXT4_ERR_INVALID;
    }
    
    dir_size = dir_inode->i_size_lo;
    
    buffer = kmalloc(fs->block_size);
    if (!buffer) {
        return EXT4_ERR_NOMEM;
    }
    
    while (offset < dir_size) {
        uint64_t logical_block = offset / fs->block_size;
        uint64_t physical_block = ext4_get_block(fs, dir_inode, logical_block);
        
        if (physical_block == 0) {
            offset += fs->block_size;
            continue;
        }
        
        ret = ext4_read_block(fs, physical_block, buffer);
        if (ret != EXT4_SUCCESS) {
            kfree(buffer);
            return ret;
        }
        
        uint32_t block_offset = 0;
        while (block_offset < fs->block_size) {
            ext4_dir_entry_t *entry = (ext4_dir_entry_t *)(buffer + block_offset);
            
            if (entry->rec_len == 0) {
                break;
            }
            
            if (entry->inode != 0 && entry->name_len == strlen(name)) {
                if (strncmp(entry->name, name, entry->name_len) == 0) {
                    *inode_out = entry->inode;
                    kfree(buffer);
                    return EXT4_SUCCESS;
                }
            }
            
            block_offset += entry->rec_len;
        }
        
        offset += fs->block_size;
    }
    
    kfree(buffer);
    return EXT4_ERR_NOTFOUND;
}

/**
 * Open a directory for iteration
 */
int ext4_dir_open(ext4_fs_t *fs, const char *path, ext4_dir_iterator_t **iter_out) {
    ext4_dir_iterator_t *iter;
    uint32_t inode_num;
    int ret;
    
    if (!fs || !path || !iter_out) {
        return EXT4_ERR_INVALID;
    }
    
    ret = ext4_lookup_path(fs, path, &inode_num);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    
    iter = kmalloc(sizeof(ext4_dir_iterator_t));
    if (!iter) {
        return EXT4_ERR_NOMEM;
    }
    
    iter->fs = fs;
    iter->inode_num = inode_num;
    iter->position = 0;
    
    ret = ext4_read_inode(fs, inode_num, &iter->inode);
    if (ret != EXT4_SUCCESS) {
        kfree(iter);
        return ret;
    }
    
    iter->size = iter->inode.i_size_lo;
    
    *iter_out = iter;
    return EXT4_SUCCESS;
}

/**
 * Read next directory entry
 */
int ext4_dir_read(ext4_dir_iterator_t *iter, ext4_dir_entry_t *entry) {
    uint8_t *buffer;
    uint64_t logical_block;
    uint64_t physical_block;
    uint32_t block_offset;
    ext4_dir_entry_t *dir_entry;
    int ret;
    
    if (!iter || !entry) {
        return EXT4_ERR_INVALID;
    }
    
    while (iter->position < iter->size) {
        logical_block = iter->position / iter->fs->block_size;
        block_offset = iter->position % iter->fs->block_size;
        
        physical_block = ext4_get_block(iter->fs, &iter->inode, logical_block);
        if (physical_block == 0) {
            iter->position += iter->fs->block_size - block_offset;
            continue;
        }
        
        buffer = kmalloc(iter->fs->block_size);
        if (!buffer) {
            return EXT4_ERR_NOMEM;
        }
        
        ret = ext4_read_block(iter->fs, physical_block, buffer);
        if (ret != EXT4_SUCCESS) {
            kfree(buffer);
            return ret;
        }
        
        dir_entry = (ext4_dir_entry_t *)(buffer + block_offset);
        
        if (dir_entry->rec_len == 0) {
            kfree(buffer);
            iter->position += iter->fs->block_size - block_offset;
            continue;
        }
        
        iter->position += dir_entry->rec_len;
        
        if (dir_entry->inode != 0) {
            memcpy(entry, dir_entry, sizeof(ext4_dir_entry_t));
            kfree(buffer);
            return EXT4_SUCCESS;
        }
        
        kfree(buffer);
    }
    
    return EXT4_ERR_NOTFOUND;
}

/**
 * Close directory iterator
 */
int ext4_dir_close(ext4_dir_iterator_t *iter) {
    if (!iter) {
        return EXT4_ERR_INVALID;
    }
    
    kfree(iter);
    return EXT4_SUCCESS;
}

/**
 * Get file/inode statistics
 */
int ext4_stat(ext4_fs_t *fs, const char *path, ext4_inode_t *inode_out) {
    uint32_t inode_num;
    int ret;
    
    if (!fs || !path || !inode_out) {
        return EXT4_ERR_INVALID;
    }
    
    ret = ext4_lookup_path(fs, path, &inode_num);
    if (ret != EXT4_SUCCESS) {
        return ret;
    }
    
    return ext4_read_inode(fs, inode_num, inode_out);
}

/**
 * Get filesystem statistics
 */
int ext4_statfs(ext4_fs_t *fs, uint64_t *total_blocks, uint64_t *free_blocks,
                uint64_t *total_inodes, uint64_t *free_inodes) {
    if (!fs) {
        return EXT4_ERR_INVALID;
    }
    
    if (total_blocks) {
        *total_blocks = fs->total_blocks;
    }
    
    if (free_blocks) {
        *free_blocks = fs->superblock->s_free_blocks_count_lo;
        if (fs->use_64bit) {
            *free_blocks |= ((uint64_t)fs->superblock->s_free_blocks_count_hi << 32);
        }
    }
    
    if (total_inodes) {
        *total_inodes = fs->superblock->s_inodes_count;
    }
    
    if (free_inodes) {
        *free_inodes = fs->superblock->s_free_inodes_count;
    }
    
    return EXT4_SUCCESS;
}
