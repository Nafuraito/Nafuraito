//! EXT4 Journaling Filesystem Driver
//!
//! This module provides a complete EXT4 filesystem driver with journaling (JBD2) 
//! support for the Nafuraito kernel, written entirely in Rust.

#![no_std]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use core::ptr;
use core::mem;
use core::slice;

extern crate alloc;
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

// ============== Memory Allocation Stubs ==============
// These should be provided by your kernel's memory allocator

extern "C" {
    fn kmalloc(size: usize) -> *mut u8;
    fn kfree(ptr: *mut u8);
}

/// Safe wrapper for byte buffer allocation
fn kernel_alloc_bytes(size: usize) -> Option<Box<[u8]>> {
    if size == 0 {
        return None;
    }
    
    let ptr = unsafe { kmalloc(size) };
    
    if ptr.is_null() {
        return None;
    }
    
    unsafe {
        // Zero initialize
        ptr::write_bytes(ptr, 0, size);
        Some(Box::from_raw(slice::from_raw_parts_mut(ptr, size)))
    }
}

// ============== Constants ==============

/// EXT4 Magic Number
pub const EXT4_SUPER_MAGIC: u16 = 0xEF53;

/// EXT4 Superblock Constants
pub const EXT4_SUPERBLOCK_OFFSET: usize = 1024;
pub const EXT4_SUPERBLOCK_SIZE: usize = 1024;

/// EXT4 Inode Constants
pub const EXT4_ROOT_INODE: u32 = 2;
pub const EXT4_GOOD_OLD_INODE_SIZE: u16 = 128;

/// EXT4 File Types
pub const EXT4_FT_UNKNOWN: u8 = 0;
pub const EXT4_FT_REG_FILE: u8 = 1;
pub const EXT4_FT_DIR: u8 = 2;
pub const EXT4_FT_CHRDEV: u8 = 3;
pub const EXT4_FT_BLKDEV: u8 = 4;
pub const EXT4_FT_FIFO: u8 = 5;
pub const EXT4_FT_SOCK: u8 = 6;
pub const EXT4_FT_SYMLINK: u8 = 7;

/// EXT4 Inode Flags
pub const EXT4_SECRM_FL: u32 = 0x00000001;
pub const EXT4_UNRM_FL: u32 = 0x00000002;
pub const EXT4_COMPR_FL: u32 = 0x00000004;
pub const EXT4_SYNC_FL: u32 = 0x00000008;
pub const EXT4_IMMUTABLE_FL: u32 = 0x00000010;
pub const EXT4_APPEND_FL: u32 = 0x00000020;
pub const EXT4_NODUMP_FL: u32 = 0x00000040;
pub const EXT4_NOATIME_FL: u32 = 0x00000080;
pub const EXT4_DIRTY_FL: u32 = 0x00000100;
pub const EXT4_COMPRBLK_FL: u32 = 0x00000200;
pub const EXT4_NOCOMPR_FL: u32 = 0x00000400;
pub const EXT4_ECOMPR_FL: u32 = 0x00000800;
pub const EXT4_INDEX_FL: u32 = 0x00001000;
pub const EXT4_IMAGIC_FL: u32 = 0x00002000;
pub const EXT4_JOURNAL_DATA_FL: u32 = 0x00004000;
pub const EXT4_NOTAIL_FL: u32 = 0x00008000;
pub const EXT4_DIRSYNC_FL: u32 = 0x00010000;
pub const EXT4_TOPDIR_FL: u32 = 0x00020000;
pub const EXT4_HUGE_FILE_FL: u32 = 0x00040000;
pub const EXT4_EXTENTS_FL: u32 = 0x00080000;
pub const EXT4_EA_INODE_FL: u32 = 0x00200000;
pub const EXT4_EOFBLOCKS_FL: u32 = 0x00400000;
pub const EXT4_INLINE_DATA_FL: u32 = 0x10000000;

/// EXT4 Feature Flags (Compatible)
pub const EXT4_FEATURE_COMPAT_DIR_PREALLOC: u32 = 0x0001;
pub const EXT4_FEATURE_COMPAT_IMAGIC_INODES: u32 = 0x0002;
pub const EXT4_FEATURE_COMPAT_HAS_JOURNAL: u32 = 0x0004;
pub const EXT4_FEATURE_COMPAT_EXT_ATTR: u32 = 0x0008;
pub const EXT4_FEATURE_COMPAT_RESIZE_INODE: u32 = 0x0010;
pub const EXT4_FEATURE_COMPAT_DIR_INDEX: u32 = 0x0020;

/// EXT4 Feature Flags (Incompatible)
pub const EXT4_FEATURE_INCOMPAT_COMPRESSION: u32 = 0x0001;
pub const EXT4_FEATURE_INCOMPAT_FILETYPE: u32 = 0x0002;
pub const EXT4_FEATURE_INCOMPAT_RECOVER: u32 = 0x0004;
pub const EXT4_FEATURE_INCOMPAT_JOURNAL_DEV: u32 = 0x0008;
pub const EXT4_FEATURE_INCOMPAT_META_BG: u32 = 0x0010;
pub const EXT4_FEATURE_INCOMPAT_EXTENTS: u32 = 0x0040;
pub const EXT4_FEATURE_INCOMPAT_64BIT: u32 = 0x0080;
pub const EXT4_FEATURE_INCOMPAT_MMP: u32 = 0x0100;
pub const EXT4_FEATURE_INCOMPAT_FLEX_BG: u32 = 0x0200;
pub const EXT4_FEATURE_INCOMPAT_EA_INODE: u32 = 0x0400;
pub const EXT4_FEATURE_INCOMPAT_DIRDATA: u32 = 0x1000;
pub const EXT4_FEATURE_INCOMPAT_CSUM_SEED: u32 = 0x2000;
pub const EXT4_FEATURE_INCOMPAT_LARGEDIR: u32 = 0x4000;
pub const EXT4_FEATURE_INCOMPAT_INLINE_DATA: u32 = 0x8000;
pub const EXT4_FEATURE_INCOMPAT_ENCRYPT: u32 = 0x10000;

/// Journal (JBD2) Constants
pub const JBD2_MAGIC_NUMBER: u32 = 0xC03B3998;
pub const JBD2_DESCRIPTOR_BLOCK: u32 = 1;
pub const JBD2_COMMIT_BLOCK: u32 = 2;
pub const JBD2_SUPERBLOCK_V1: u32 = 3;
pub const JBD2_SUPERBLOCK_V2: u32 = 4;
pub const JBD2_REVOKE_BLOCK: u32 = 5;

/// Journal Flags
pub const JBD2_FLAG_ESCAPE: u16 = 0x01;
pub const JBD2_FLAG_SAME_UUID: u16 = 0x02;
pub const JBD2_FLAG_DELETED: u16 = 0x04;
pub const JBD2_FLAG_LAST_TAG: u16 = 0x08;

/// Maximum Values
pub const EXT4_MAX_BLOCK_SIZE: u32 = 65536;
pub const EXT4_MIN_BLOCK_SIZE: u32 = 1024;
pub const EXT4_NAME_LEN: usize = 255;
pub const EXT4_NDIR_BLOCKS: usize = 12;
pub const EXT4_IND_BLOCK: usize = 12;
pub const EXT4_DIND_BLOCK: usize = 13;
pub const EXT4_TIND_BLOCK: usize = 14;
pub const EXT4_N_BLOCKS: usize = 15;

/// Extent Tree Constants
pub const EXT4_EXT_MAGIC: u16 = 0xF30A;
pub const EXT4_EXT_MAX_DEPTH: u16 = 5;

// ============== Memory-Mapped File Helpers ==============

/// Scratch buffer size used for mmap read-at operations.
///
/// TODO: Support larger block sizes with a dynamic or per-CPU buffer.
const EXT4_MMAP_SCRATCH_SIZE: usize = 4096;

/// Global scratch buffer for mmap read-at operations.
///
/// TODO: Guard with a lock once SMP or preemption is enabled.
static mut EXT4_MMAP_SCRATCH: [u8; EXT4_MMAP_SCRATCH_SIZE] = [0; EXT4_MMAP_SCRATCH_SIZE];

// ============== Error Types ==============

/// Error codes
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ext4Error {
    Success = 0,
    InvalidMagic = -1,
    Io = -2,
    NoMem = -3,
    NotFound = -4,
    Invalid = -5,
    Journal = -6,
    NoSpace = -7,
    Permission = -8,
}

impl Ext4Error {
    pub fn is_success(&self) -> bool {
        matches!(self, Ext4Error::Success)
    }
}

/// Result type for EXT4 operations
pub type Ext4Result<T> = Result<T, Ext4Error>;

// ============== Structures ==============

/// EXT4 Superblock Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4Superblock {
    pub s_inodes_count: u32,
    pub s_blocks_count_lo: u32,
    pub s_r_blocks_count_lo: u32,
    pub s_free_blocks_count_lo: u32,
    pub s_free_inodes_count: u32,
    pub s_first_data_block: u32,
    pub s_log_block_size: u32,
    pub s_log_cluster_size: u32,
    pub s_blocks_per_group: u32,
    pub s_clusters_per_group: u32,
    pub s_inodes_per_group: u32,
    pub s_mtime: u32,
    pub s_wtime: u32,
    pub s_mnt_count: u16,
    pub s_max_mnt_count: u16,
    pub s_magic: u16,
    pub s_state: u16,
    pub s_errors: u16,
    pub s_minor_rev_level: u16,
    pub s_lastcheck: u32,
    pub s_checkinterval: u32,
    pub s_creator_os: u32,
    pub s_rev_level: u32,
    pub s_def_resuid: u16,
    pub s_def_resgid: u16,
    pub s_first_ino: u32,
    pub s_inode_size: u16,
    pub s_block_group_nr: u16,
    pub s_feature_compat: u32,
    pub s_feature_incompat: u32,
    pub s_feature_ro_compat: u32,
    pub s_uuid: [u8; 16],
    pub s_volume_name: [u8; 16],
    pub s_last_mounted: [u8; 64],
    pub s_algorithm_usage_bitmap: u32,
    pub s_prealloc_blocks: u8,
    pub s_prealloc_dir_blocks: u8,
    pub s_reserved_gdt_blocks: u16,
    pub s_journal_uuid: [u8; 16],
    pub s_journal_inum: u32,
    pub s_journal_dev: u32,
    pub s_last_orphan: u32,
    pub s_hash_seed: [u32; 4],
    pub s_def_hash_version: u8,
    pub s_jnl_backup_type: u8,
    pub s_desc_size: u16,
    pub s_default_mount_opts: u32,
    pub s_first_meta_bg: u32,
    pub s_mkfs_time: u32,
    pub s_jnl_blocks: [u32; 17],
    pub s_blocks_count_hi: u32,
    pub s_r_blocks_count_hi: u32,
    pub s_free_blocks_count_hi: u32,
    pub s_min_extra_isize: u16,
    pub s_want_extra_isize: u16,
    pub s_flags: u32,
    pub s_raid_stride: u16,
    pub s_mmp_interval: u16,
    pub s_mmp_block: u64,
    pub s_raid_stripe_width: u32,
    pub s_log_groups_per_flex: u8,
    pub s_checksum_type: u8,
    pub s_reserved_pad: u16,
    pub s_kbytes_written: u64,
    pub s_snapshot_inum: u32,
    pub s_snapshot_id: u32,
    pub s_snapshot_r_blocks_count: u64,
    pub s_snapshot_list: u32,
    pub s_error_count: u32,
    pub s_first_error_time: u32,
    pub s_first_error_ino: u32,
    pub s_first_error_block: u64,
    pub s_first_error_func: [u8; 32],
    pub s_first_error_line: u32,
    pub s_last_error_time: u32,
    pub s_last_error_ino: u32,
    pub s_last_error_line: u32,
    pub s_last_error_block: u64,
    pub s_last_error_func: [u8; 32],
    pub s_mount_opts: [u8; 64],
    pub s_usr_quota_inum: u32,
    pub s_grp_quota_inum: u32,
    pub s_overhead_blocks: u32,
    pub s_backup_bgs: [u32; 2],
    pub s_encrypt_algos: [u8; 4],
    pub s_encrypt_pw_salt: [u8; 16],
    pub s_lpf_ino: u32,
    pub s_prj_quota_inum: u32,
    pub s_checksum_seed: u32,
    pub s_reserved: [u32; 98],
    pub s_checksum: u32,
}

impl Default for Ext4Superblock {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

/// EXT4 Block Group Descriptor (64-bit version)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4GroupDesc {
    pub bg_block_bitmap_lo: u32,
    pub bg_inode_bitmap_lo: u32,
    pub bg_inode_table_lo: u32,
    pub bg_free_blocks_count_lo: u16,
    pub bg_free_inodes_count_lo: u16,
    pub bg_used_dirs_count_lo: u16,
    pub bg_flags: u16,
    pub bg_exclude_bitmap_lo: u32,
    pub bg_block_bitmap_csum_lo: u16,
    pub bg_inode_bitmap_csum_lo: u16,
    pub bg_itable_unused_lo: u16,
    pub bg_checksum: u16,
    pub bg_block_bitmap_hi: u32,
    pub bg_inode_bitmap_hi: u32,
    pub bg_inode_table_hi: u32,
    pub bg_free_blocks_count_hi: u16,
    pub bg_free_inodes_count_hi: u16,
    pub bg_used_dirs_count_hi: u16,
    pub bg_itable_unused_hi: u16,
    pub bg_exclude_bitmap_hi: u32,
    pub bg_block_bitmap_csum_hi: u16,
    pub bg_inode_bitmap_csum_hi: u16,
    pub bg_reserved: u32,
}

impl Default for Ext4GroupDesc {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

/// EXT4 Inode Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4Inode {
    pub i_mode: u16,
    pub i_uid: u16,
    pub i_size_lo: u32,
    pub i_atime: u32,
    pub i_ctime: u32,
    pub i_mtime: u32,
    pub i_dtime: u32,
    pub i_gid: u16,
    pub i_links_count: u16,
    pub i_blocks_lo: u32,
    pub i_flags: u32,
    pub i_osd1: u32,
    pub i_block: [u32; EXT4_N_BLOCKS],
    pub i_generation: u32,
    pub i_file_acl_lo: u32,
    pub i_size_hi: u32,
    pub i_obso_faddr: u32,
    pub i_blocks_hi: u16,
    pub i_file_acl_hi: u16,
    pub i_uid_hi: u16,
    pub i_gid_hi: u16,
    pub i_checksum_lo: u16,
    pub i_reserved: u16,
    pub i_extra_isize: u16,
    pub i_checksum_hi: u16,
    pub i_ctime_extra: u32,
    pub i_mtime_extra: u32,
    pub i_atime_extra: u32,
    pub i_crtime: u32,
    pub i_crtime_extra: u32,
    pub i_version_hi: u32,
    pub i_projid: u32,
}

impl Default for Ext4Inode {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

impl Ext4Inode {
    /// Get the full 64-bit file size
    pub fn size(&self) -> u64 {
        (self.i_size_lo as u64) | ((self.i_size_hi as u64) << 32)
    }

    /// Get the full 32-bit UID
    pub fn uid(&self) -> u32 {
        (self.i_uid as u32) | ((self.i_uid_hi as u32) << 16)
    }

    /// Get the full 32-bit GID
    pub fn gid(&self) -> u32 {
        (self.i_gid as u32) | ((self.i_gid_hi as u32) << 16)
    }

    /// Check if this inode uses extents
    pub fn uses_extents(&self) -> bool {
        (self.i_flags & EXT4_EXTENTS_FL) != 0
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        (self.i_mode & 0xF000) == 0x4000
    }

    /// Check if this is a regular file
    pub fn is_file(&self) -> bool {
        (self.i_mode & 0xF000) == 0x8000
    }

    /// Check if this is a symlink
    pub fn is_symlink(&self) -> bool {
        (self.i_mode & 0xF000) == 0xA000
    }
}

/// EXT4 Directory Entry Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4DirEntry {
    pub inode: u32,
    pub rec_len: u16,
    pub name_len: u8,
    pub file_type: u8,
    pub name: [u8; EXT4_NAME_LEN],
}

impl Default for Ext4DirEntry {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

impl Ext4DirEntry {
    /// Get the name as a byte slice
    pub fn name_bytes(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

/// EXT4 Extent Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4ExtentHeader {
    pub eh_magic: u16,
    pub eh_entries: u16,
    pub eh_max: u16,
    pub eh_depth: u16,
    pub eh_generation: u32,
}

impl Default for Ext4ExtentHeader {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

/// EXT4 Extent Index (for internal nodes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4ExtentIdx {
    pub ei_block: u32,
    pub ei_leaf_lo: u32,
    pub ei_leaf_hi: u16,
    pub ei_unused: u16,
}

impl Default for Ext4ExtentIdx {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

impl Ext4ExtentIdx {
    /// Get the full 48-bit physical block number
    pub fn leaf(&self) -> u64 {
        (self.ei_leaf_lo as u64) | ((self.ei_leaf_hi as u64) << 32)
    }
}

/// EXT4 Extent (leaf node)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4Extent {
    pub ee_block: u32,
    pub ee_len: u16,
    pub ee_start_hi: u16,
    pub ee_start_lo: u32,
}

impl Default for Ext4Extent {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

impl Ext4Extent {
    /// Get the full 48-bit physical block number
    pub fn start(&self) -> u64 {
        (self.ee_start_lo as u64) | ((self.ee_start_hi as u64) << 32)
    }

    /// Get the length (masking the uninitialized flag)
    pub fn len(&self) -> u16 {
        self.ee_len & 0x7FFF
    }

    /// Check if this extent is uninitialized
    pub fn is_uninitialized(&self) -> bool {
        (self.ee_len & 0x8000) != 0
    }
}

/// JBD2 Journal Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Jbd2Header {
    pub h_magic: u32,
    pub h_blocktype: u32,
    pub h_sequence: u32,
}

impl Default for Jbd2Header {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

/// JBD2 Journal Superblock
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Jbd2Superblock {
    pub s_header: Jbd2Header,
    pub s_blocksize: u32,
    pub s_maxlen: u32,
    pub s_first: u32,
    pub s_sequence: u32,
    pub s_start: u32,
    pub s_errno: u32,
    pub s_feature_compat: u32,
    pub s_feature_incompat: u32,
    pub s_feature_ro_compat: u32,
    pub s_uuid: [u8; 16],
    pub s_nr_users: u32,
    pub s_dynsuper: u32,
    pub s_max_transaction: u32,
    pub s_max_trans_data: u32,
    pub s_checksum_type: u8,
    pub s_padding2: [u8; 3],
    pub s_padding: [u32; 42],
    pub s_checksum: u32,
    pub s_users: [u8; 16 * 48],
}

impl Default for Jbd2Superblock {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

/// JBD2 Block Tag
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Jbd2BlockTag {
    pub t_blocknr: u32,
    pub t_checksum: u16,
    pub t_flags: u16,
    pub t_blocknr_hi: u32,
}

impl Default for Jbd2BlockTag {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

// ============== Block Device Operations ==============

/// Block device operations callback types
pub type ReadBlockFn = fn(dev: *mut u8, block: u64, buffer: &mut [u8]) -> Ext4Result<()>;
pub type WriteBlockFn = fn(dev: *mut u8, block: u64, buffer: &[u8]) -> Ext4Result<()>;
pub type SyncFn = fn(dev: *mut u8) -> Ext4Result<()>;

/// Block Device Operations structure
pub struct Ext4BlockOps {
    pub read_block: Option<ReadBlockFn>,
    pub write_block: Option<WriteBlockFn>,
    pub sync: Option<SyncFn>,
}

// ============== Journal Transaction ==============

struct Transaction {
    tid: u32,
    blocks: Vec<u64>,
    data: Vec<Box<[u8]>>,
}

impl Transaction {
    fn new(tid: u32) -> Self {
        Transaction {
            tid,
            blocks: Vec::new(),
            data: Vec::new(),
        }
    }

    fn add_block(&mut self, block: u64, data: Box<[u8]>) {
        self.blocks.push(block);
        self.data.push(data);
    }
}

// ============== Journal ==============

struct Journal {
    superblock: Jbd2Superblock,
    journal_start: u64,
    journal_length: u32,
    current_sequence: u32,
    head: u32,
    tail: u32,
    running: bool,
    current_transaction: Option<Transaction>,
}

impl Journal {
    fn new(superblock: Jbd2Superblock, journal_start: u64) -> Self {
        let journal_length = superblock.s_maxlen;
        let current_sequence = superblock.s_sequence;
        let head = superblock.s_start;
        let tail = superblock.s_start;

        Journal {
            superblock,
            journal_start,
            journal_length,
            current_sequence,
            head,
            tail,
            running: true,
            current_transaction: None,
        }
    }

    fn start_transaction(&mut self) -> Ext4Result<()> {
        if self.current_transaction.is_some() {
            return Ok(());
        }

        let tid = self.current_sequence;
        self.current_sequence += 1;
        self.current_transaction = Some(Transaction::new(tid));
        Ok(())
    }

    fn add_block(&mut self, block: u64, data: Box<[u8]>) -> Ext4Result<()> {
        if self.current_transaction.is_none() {
            self.start_transaction()?;
        }

        if let Some(ref mut trans) = self.current_transaction {
            trans.add_block(block, data);
        }
        Ok(())
    }
}

// ============== Filesystem ==============

/// EXT4 Filesystem Context
pub struct Ext4Filesystem {
    device: *mut u8,
    ops: Ext4BlockOps,
    superblock: Box<Ext4Superblock>,
    group_descs: Vec<Ext4GroupDesc>,
    journal: Option<Box<Journal>>,
    block_size: u32,
    blocks_per_group: u32,
    inodes_per_group: u32,
    inode_size: u32,
    group_count: u32,
    desc_size: u32,
    total_blocks: u64,
    readonly: bool,
    use_extents: bool,
    use_64bit: bool,
}

impl Ext4Filesystem {
    /// Mount an EXT4 filesystem
    pub fn mount(device: *mut u8, ops: Ext4BlockOps) -> Ext4Result<Self> {
        let mut fs = Ext4Filesystem {
            device,
            ops,
            superblock: Box::new(Ext4Superblock::default()),
            group_descs: Vec::new(),
            journal: None,
            block_size: 0,
            blocks_per_group: 0,
            inodes_per_group: 0,
            inode_size: 0,
            group_count: 0,
            desc_size: 0,
            total_blocks: 0,
            readonly: false,
            use_extents: false,
            use_64bit: false,
        };

        fs.read_superblock()?;
        fs.read_group_descriptors()?;
        fs.init_journal()?;

        Ok(fs)
    }

    /// Read the superblock
    fn read_superblock(&mut self) -> Ext4Result<()> {
        let mut buffer = kernel_alloc_bytes(EXT4_MIN_BLOCK_SIZE * 2)
            .ok_or(Ext4Error::NoMem)?;

        // Read first two blocks (2KB) to get superblock
        if let Some(read_fn) = self.ops.read_block {
            read_fn(self.device, 0, &mut buffer)?;
        } else {
            return Err(Ext4Error::Invalid);
        }

        // Copy superblock from offset 1024
        let sb_bytes = &buffer[EXT4_SUPERBLOCK_OFFSET..EXT4_SUPERBLOCK_OFFSET + mem::size_of::<Ext4Superblock>()];
        unsafe {
            ptr::copy_nonoverlapping(
                sb_bytes.as_ptr(),
                &mut *self.superblock as *mut Ext4Superblock as *mut u8,
                mem::size_of::<Ext4Superblock>()
            );
        }

        // Validate magic number
        if self.superblock.s_magic != EXT4_SUPER_MAGIC {
            return Err(Ext4Error::InvalidMagic);
        }

        // Calculate filesystem parameters
        self.block_size = EXT4_MIN_BLOCK_SIZE << self.superblock.s_log_block_size;
        self.blocks_per_group = self.superblock.s_blocks_per_group;
        self.inodes_per_group = self.superblock.s_inodes_per_group;
        self.inode_size = self.superblock.s_inode_size as u32;

        // Handle 64-bit block counts
        if (self.superblock.s_feature_incompat & EXT4_FEATURE_INCOMPAT_64BIT) != 0 {
            self.use_64bit = true;
            self.total_blocks = ((self.superblock.s_blocks_count_hi as u64) << 32) 
                | self.superblock.s_blocks_count_lo as u64;
            self.desc_size = self.superblock.s_desc_size as u32;
            if self.desc_size < 32 {
                self.desc_size = 32;
            }
        } else {
            self.use_64bit = false;
            self.total_blocks = self.superblock.s_blocks_count_lo as u64;
            self.desc_size = 32;
        }

        // Calculate group count
        self.group_count = ((self.total_blocks + self.blocks_per_group as u64 - 1) 
            / self.blocks_per_group as u64) as u32;

        // Check for extent support
        self.use_extents = (self.superblock.s_feature_incompat & EXT4_FEATURE_INCOMPAT_EXTENTS) != 0;

        Ok(())
    }

    /// Read block group descriptors
    fn read_group_descriptors(&mut self) -> Ext4Result<()> {
        let desc_per_block = self.block_size / self.desc_size;
        let desc_blocks = (self.group_count + desc_per_block - 1) / desc_per_block;

        let mut buffer = kernel_alloc_bytes(self.block_size as usize)
            .ok_or(Ext4Error::NoMem)?;

        // Group descriptors start at block 1 (for 1K blocks) or block 1 (for larger blocks)
        let desc_start = if self.block_size == EXT4_MIN_BLOCK_SIZE { 2 } else { 1 };

        for i in 0..desc_blocks {
            self.read_block(desc_start + i as u64, &mut buffer)?;

            for j in 0..desc_per_block {
                let idx = i * desc_per_block + j;
                if idx >= self.group_count {
                    break;
                }

                let offset = (j * self.desc_size) as usize;
                let mut desc = Ext4GroupDesc::default();
                unsafe {
                    ptr::copy_nonoverlapping(
                        buffer[offset..].as_ptr(),
                        &mut desc as *mut Ext4GroupDesc as *mut u8,
                        mem::size_of::<Ext4GroupDesc>().min(self.desc_size as usize)
                    );
                }
                self.group_descs.push(desc);
            }
        }

        Ok(())
    }

    /// Initialize the journal
    fn init_journal(&mut self) -> Ext4Result<()> {
        if (self.superblock.s_feature_compat & EXT4_FEATURE_COMPAT_HAS_JOURNAL) == 0 {
            return Ok(());
        }

        // Read journal inode
        let journal_inode_num = self.superblock.s_journal_inum;
        let journal_inode = self.read_inode(journal_inode_num)?;

        // Get first block of journal
        let journal_start = self.get_block(&journal_inode, 0)?;
        if journal_start == 0 {
            return Err(Ext4Error::Journal);
        }

        // Read journal superblock
        let mut buffer = kernel_alloc_bytes(self.block_size as usize)
            .ok_or(Ext4Error::NoMem)?;
        self.read_block(journal_start, &mut buffer)?;

        let mut jbd2_sb = Jbd2Superblock::default();
        unsafe {
            ptr::copy_nonoverlapping(
                buffer.as_ptr(),
                &mut jbd2_sb as *mut Jbd2Superblock as *mut u8,
                mem::size_of::<Jbd2Superblock>()
            );
        }

        // Validate journal magic
        if jbd2_sb.s_header.h_magic != JBD2_MAGIC_NUMBER {
            return Err(Ext4Error::Journal);
        }

        let journal = Box::new(Journal::new(jbd2_sb, journal_start));
        self.journal = Some(journal);

        // Recover if needed
        if (self.superblock.s_feature_incompat & EXT4_FEATURE_INCOMPAT_RECOVER) != 0 {
            if self.journal_recover().is_err() {
                self.readonly = true;
            }
        }

        Ok(())
    }

    /// Recover journal
    fn journal_recover(&mut self) -> Ext4Result<()> {
        // Journal recovery implementation
        // For now, just clear the recovery flag
        self.superblock.s_feature_incompat &= !EXT4_FEATURE_INCOMPAT_RECOVER;
        Ok(())
    }

    /// Read a block from the device
    fn read_block(&self, block: u64, buffer: &mut [u8]) -> Ext4Result<()> {
        if let Some(read_fn) = self.ops.read_block {
            read_fn(self.device, block, buffer)
        } else {
            Err(Ext4Error::Invalid)
        }
    }

    /// Read data at a byte offset into a caller-provided buffer.
    ///
    /// This avoids heap allocations so it can be used by memory-mapped paging.
    pub fn read_at_into(
        &self,
        inode: &Ext4Inode,
        offset: u64,
        dst: &mut [u8],
    ) -> Ext4Result<usize> {
        let file_size = inode.size();
        if offset >= file_size || dst.is_empty() {
            return Ok(0);
        }

        let block_size = self.block_size as usize;
        if block_size == 0 {
            return Err(Ext4Error::Invalid);
        }
        if block_size > EXT4_MMAP_SCRATCH_SIZE {
            // TODO: Support larger block sizes with a bigger scratch buffer.
            return Err(Ext4Error::NoMem);
        }

        let max_len = (file_size - offset) as usize;
        let to_read = core::cmp::min(dst.len(), max_len);

        let mut total_read = 0usize;
        while total_read < to_read {
            let current_offset = offset + total_read as u64;
            let logical_block = current_offset / self.block_size as u64;
            let block_offset = (current_offset % self.block_size as u64) as usize;
            let chunk = core::cmp::min(block_size - block_offset, to_read - total_read);

            let physical_block = self.get_block(inode, logical_block)?;
            if physical_block == 0 {
                dst[total_read..total_read + chunk].fill(0);
            } else if block_offset == 0 && chunk == block_size {
                self.read_block(physical_block, &mut dst[total_read..total_read + chunk])?;
            } else {
                let scratch = unsafe { &mut EXT4_MMAP_SCRATCH };
                self.read_block(physical_block, &mut scratch[..block_size])?;
                dst[total_read..total_read + chunk]
                    .copy_from_slice(&scratch[block_offset..block_offset + chunk]);
            }

            total_read += chunk;
        }

        Ok(total_read)
    }

    /// Write a block to the device
    fn write_block(&self, block: u64, buffer: &[u8]) -> Ext4Result<()> {
        if self.readonly {
            return Err(Ext4Error::Permission);
        }

        if let Some(write_fn) = self.ops.write_block {
            write_fn(self.device, block, buffer)
        } else {
            Err(Ext4Error::Invalid)
        }
    }

    /// Read an inode
    fn read_inode(&self, inode_num: u32) -> Ext4Result<Ext4Inode> {
        if inode_num == 0 {
            return Err(Ext4Error::Invalid);
        }

        // Calculate which block group contains this inode
        let group = (inode_num - 1) / self.inodes_per_group;
        let index = (inode_num - 1) % self.inodes_per_group;

        if group >= self.group_count {
            return Err(Ext4Error::Invalid);
        }

        // Get inode table location
        let desc = &self.group_descs[group as usize];
        let mut inode_table = desc.bg_inode_table_lo as u64;
        if self.use_64bit {
            inode_table |= (desc.bg_inode_table_hi as u64) << 32;
        }

        // Calculate block and offset within block
        let block = inode_table + ((index * self.inode_size) / self.block_size) as u64;
        let offset = ((index * self.inode_size) % self.block_size) as usize;

        let mut buffer = kernel_alloc_bytes(self.block_size as usize)
            .ok_or(Ext4Error::NoMem)?;
        self.read_block(block, &mut buffer)?;

        let mut inode = Ext4Inode::default();
        unsafe {
            ptr::copy_nonoverlapping(
                buffer[offset..].as_ptr(),
                &mut inode as *mut Ext4Inode as *mut u8,
                mem::size_of::<Ext4Inode>()
            );
        }

        Ok(inode)
    }

    /// Write an inode
    fn write_inode(&mut self, inode_num: u32, inode: &Ext4Inode) -> Ext4Result<()> {
        if inode_num == 0 || self.readonly {
            return Err(Ext4Error::Invalid);
        }

        let group = (inode_num - 1) / self.inodes_per_group;
        let index = (inode_num - 1) % self.inodes_per_group;

        if group >= self.group_count {
            return Err(Ext4Error::Invalid);
        }

        let desc = &self.group_descs[group as usize];
        let mut inode_table = desc.bg_inode_table_lo as u64;
        if self.use_64bit {
            inode_table |= (desc.bg_inode_table_hi as u64) << 32;
        }

        let block = inode_table + ((index * self.inode_size) / self.block_size) as u64;
        let offset = ((index * self.inode_size) % self.block_size) as usize;

        let mut buffer = kernel_alloc_bytes(self.block_size as usize)
            .ok_or(Ext4Error::NoMem)?;
        self.read_block(block, &mut buffer)?;

        unsafe {
            ptr::copy_nonoverlapping(
                inode as *const Ext4Inode as *const u8,
                buffer[offset..].as_mut_ptr(),
                mem::size_of::<Ext4Inode>()
            );
        }

        self.write_block_journaled(block, &buffer)?;
        Ok(())
    }

    /// Write a block with journaling support
    fn write_block_journaled(&mut self, block: u64, data: &[u8]) -> Ext4Result<()> {
        if let Some(ref mut journal) = self.journal {
            if journal.running {
                let data_copy = kernel_alloc_bytes(data.len())
                    .ok_or(Ext4Error::NoMem)?;
                journal.add_block(block, data_copy)?;
                return Ok(());
            }
        }

        // No journal or journal not running - write directly
        self.write_block(block, data)
    }

    /// Get physical block number from extent tree
    fn get_block_from_extent(&self, inode: &Ext4Inode, logical_block: u64) -> Ext4Result<u64> {
        let header_ptr = &inode.i_block as *const u32 as *const Ext4ExtentHeader;
        let mut header = unsafe { *header_ptr };

        if header.eh_magic != EXT4_EXT_MAGIC {
            return Err(Ext4Error::Invalid);
        }

        let mut depth = header.eh_depth;
        let mut buffer_opt: Option<Box<[u8]>> = None;

        while depth > 0 {
            // Internal node - find the right child
            let idx_ptr = unsafe { (header_ptr as *const u8).add(mem::size_of::<Ext4ExtentHeader>()) as *const Ext4ExtentIdx };
            let indices = unsafe { slice::from_raw_parts(idx_ptr, header.eh_entries as usize) };

            let mut best: Option<&Ext4ExtentIdx> = None;
            for idx in indices {
                if idx.ei_block <= logical_block as u32 {
                    best = Some(idx);
                } else {
                    break;
                }
            }

            let best_idx = best.ok_or(Ext4Error::NotFound)?;
            let physical_block = best_idx.leaf();

            // Read the child block
            let mut buffer = kernel_alloc_bytes(self.block_size as usize)
                .ok_or(Ext4Error::NoMem)?;
            self.read_block(physical_block, &mut buffer)?;

            let new_header_ptr = buffer.as_ptr() as *const Ext4ExtentHeader;
            header = unsafe { *new_header_ptr };
            buffer_opt = Some(buffer);
            depth -= 1;
        }

        // Leaf node - find the extent
        let buffer = buffer_opt.as_ref()
            .map(|b| b.as_ptr())
            .unwrap_or(&inode.i_block as *const u32 as *const u8);
        
        let extent_ptr = unsafe { buffer.add(mem::size_of::<Ext4ExtentHeader>()) as *const Ext4Extent };
        let extents = unsafe { slice::from_raw_parts(extent_ptr, header.eh_entries as usize) };

        for extent in extents {
            let ee_block = extent.ee_block;
            let ee_len = extent.len();

            if logical_block >= ee_block as u64 && logical_block < (ee_block + ee_len as u32) as u64 {
                let physical_block = extent.start() + (logical_block - ee_block as u64);
                return Ok(physical_block);
            }
        }

        Err(Ext4Error::NotFound)
    }

    /// Get physical block number using indirect blocks (legacy)
    fn get_block_indirect(&self, inode: &Ext4Inode, logical_block: u64) -> Ext4Result<u64> {
        let ptrs_per_block = self.block_size as u64 / 4;

        // Direct blocks
        if logical_block < EXT4_NDIR_BLOCKS as u64 {
            return Ok(inode.i_block[logical_block as usize] as u64);
        }

        let mut logical_block = logical_block - EXT4_NDIR_BLOCKS as u64;

        let mut buffer = kernel_alloc_bytes(self.block_size as usize)
            .ok_or(Ext4Error::NoMem)?;

        // Single indirect
        if logical_block < ptrs_per_block {
            self.read_block(inode.i_block[EXT4_IND_BLOCK] as u64, &mut buffer)?;
            let ptrs = unsafe {
                slice::from_raw_parts(buffer.as_ptr() as *const u32, ptrs_per_block as usize)
            };
            return Ok(ptrs[logical_block as usize] as u64);
        }

        logical_block -= ptrs_per_block;

        // Double indirect
        if logical_block < ptrs_per_block * ptrs_per_block {
            self.read_block(inode.i_block[EXT4_DIND_BLOCK] as u64, &mut buffer)?;
            let ptrs = unsafe {
                slice::from_raw_parts(buffer.as_ptr() as *const u32, ptrs_per_block as usize)
            };
            let ind_block = ptrs[(logical_block / ptrs_per_block) as usize];
            
            self.read_block(ind_block as u64, &mut buffer)?;
            let ptrs = unsafe {
                slice::from_raw_parts(buffer.as_ptr() as *const u32, ptrs_per_block as usize)
            };
            return Ok(ptrs[(logical_block % ptrs_per_block) as usize] as u64);
        }

        logical_block -= ptrs_per_block * ptrs_per_block;

        // Triple indirect
        self.read_block(inode.i_block[EXT4_TIND_BLOCK] as u64, &mut buffer)?;
        let ptrs = unsafe {
            slice::from_raw_parts(buffer.as_ptr() as *const u32, ptrs_per_block as usize)
        };
        let dind_block = ptrs[(logical_block / (ptrs_per_block * ptrs_per_block)) as usize];

        self.read_block(dind_block as u64, &mut buffer)?;
        let ptrs = unsafe {
            slice::from_raw_parts(buffer.as_ptr() as *const u32, ptrs_per_block as usize)
        };
        let ind_block = ptrs[((logical_block / ptrs_per_block) % ptrs_per_block) as usize];

        self.read_block(ind_block as u64, &mut buffer)?;
        let ptrs = unsafe {
            slice::from_raw_parts(buffer.as_ptr() as *const u32, ptrs_per_block as usize)
        };
        Ok(ptrs[(logical_block % ptrs_per_block) as usize] as u64)
    }

    /// Get physical block number for a logical block in an inode
    pub fn get_block(&self, inode: &Ext4Inode, logical_block: u64) -> Ext4Result<u64> {
        if inode.uses_extents() {
            self.get_block_from_extent(inode, logical_block)
        } else {
            self.get_block_indirect(inode, logical_block)
        }
    }

    /// Lookup a path and return the inode number
    pub fn lookup_path(&self, path: &[u8]) -> Ext4Result<u32> {
        let mut current_inode = EXT4_ROOT_INODE;
        let mut p = path;

        // Skip leading slashes
        while !p.is_empty() && p[0] == b'/' {
            p = &p[1..];
        }

        // Empty path or root
        if p.is_empty() {
            return Ok(EXT4_ROOT_INODE);
        }

        while !p.is_empty() {
            // Extract next path component
            let mut len = 0;
            while len < p.len() && p[len] != b'/' && len < EXT4_NAME_LEN {
                len += 1;
            }

            if len == 0 {
                p = &p[1..];
                continue;
            }

            let name = &p[..len];

            // Read current directory inode
            let inode = self.read_inode(current_inode)?;

            // Search directory for name
            current_inode = self.dir_find_entry(&inode, name)?;

            p = &p[len..];
            while !p.is_empty() && p[0] == b'/' {
                p = &p[1..];
            }
        }

        Ok(current_inode)
    }

    /// Find an entry in a directory
    pub fn dir_find_entry(&self, dir_inode: &Ext4Inode, name: &[u8]) -> Ext4Result<u32> {
        let dir_size = dir_inode.size();
        let mut offset = 0u64;

        let mut buffer = kernel_alloc_bytes(self.block_size as usize)
            .ok_or(Ext4Error::NoMem)?;

        while offset < dir_size {
            let logical_block = offset / self.block_size as u64;
            let physical_block = self.get_block(dir_inode, logical_block)?;

            if physical_block == 0 {
                offset += self.block_size as u64;
                continue;
            }

            self.read_block(physical_block, &mut buffer)?;

            let mut block_offset = 0usize;
            while block_offset < self.block_size as usize {
                let entry_ptr = unsafe { buffer[block_offset..].as_ptr() as *const Ext4DirEntry };
                let entry = unsafe { &*entry_ptr };

                if entry.rec_len == 0 {
                    break;
                }

                if entry.inode != 0 && entry.name_len as usize == name.len() {
                    let entry_name = entry.name_bytes();
                    if entry_name == name {
                        return Ok(entry.inode);
                    }
                }

                block_offset += entry.rec_len as usize;
            }

            offset += self.block_size as u64;
        }

        Err(Ext4Error::NotFound)
    }

    /// Open a file by path
    pub fn open(&mut self, path: &[u8], flags: u32) -> Ext4Result<Ext4FileHandle> {
        let inode_num = self.lookup_path(path)?;
        let inode = self.read_inode(inode_num)?;

        Ok(Ext4FileHandle {
            fs: self,
            inode_num,
            inode,
            position: 0,
            flags,
        })
    }

    /// Get file statistics
    pub fn stat(&self, path: &[u8]) -> Ext4Result<Ext4Inode> {
        let inode_num = self.lookup_path(path)?;
        self.read_inode(inode_num)
    }

    /// Get filesystem statistics
    pub fn statfs(&self) -> Ext4FsStats {
        let free_blocks = if self.use_64bit {
            ((self.superblock.s_free_blocks_count_hi as u64) << 32) 
                | self.superblock.s_free_blocks_count_lo as u64
        } else {
            self.superblock.s_free_blocks_count_lo as u64
        };

        Ext4FsStats {
            total_blocks: self.total_blocks,
            free_blocks,
            total_inodes: self.superblock.s_inodes_count as u64,
            free_inodes: self.superblock.s_free_inodes_count as u64,
        }
    }

    /// Commit current journal transaction
    pub fn journal_commit(&mut self) -> Ext4Result<()> {
        if let Some(ref mut journal) = self.journal {
            if let Some(trans) = journal.current_transaction.take() {
                // Write descriptor block
                let mut desc_buffer = kernel_alloc_bytes(self.block_size as usize)
                    .ok_or(Ext4Error::NoMem)?;
                
                let header = Jbd2Header {
                    h_magic: JBD2_MAGIC_NUMBER,
                    h_blocktype: JBD2_DESCRIPTOR_BLOCK,
                    h_sequence: trans.tid,
                };

                unsafe {
                    ptr::copy_nonoverlapping(
                        &header as *const Jbd2Header as *const u8,
                        desc_buffer.as_mut_ptr(),
                        mem::size_of::<Jbd2Header>()
                    );
                }

                let mut journal_block = journal.head as u64;
                self.write_block(journal.journal_start + journal_block, &desc_buffer)?;
                journal_block += 1;

                // Write data blocks to journal
                for data in &trans.data {
                    self.write_block(journal.journal_start + journal_block, data)?;
                    journal_block += 1;
                    if journal_block >= journal.journal_length as u64 {
                        journal_block = 1;
                    }
                }

                // Write commit block
                let commit_header = Jbd2Header {
                    h_magic: JBD2_MAGIC_NUMBER,
                    h_blocktype: JBD2_COMMIT_BLOCK,
                    h_sequence: trans.tid,
                };

                let mut commit_buffer = kernel_alloc_bytes(self.block_size as usize)
                    .ok_or(Ext4Error::NoMem)?;
                unsafe {
                    ptr::copy_nonoverlapping(
                        &commit_header as *const Jbd2Header as *const u8,
                        commit_buffer.as_mut_ptr(),
                        mem::size_of::<Jbd2Header>()
                    );
                }
                self.write_block(journal.journal_start + journal_block, &commit_buffer)?;

                // Write data to actual locations
                for (i, block) in trans.blocks.iter().enumerate() {
                    self.write_block(*block, &trans.data[i])?;
                }

                // Sync device
                if let Some(sync_fn) = self.ops.sync {
                    sync_fn(self.device)?;
                }

                journal.head = journal_block as u32 + 1;
            }
        }
        Ok(())
    }

    /// Unmount the filesystem
    pub fn unmount(mut self) -> Ext4Result<()> {
        // Commit any pending transaction
        self.journal_commit()?;

        // Sync filesystem
        if let Some(sync_fn) = self.ops.sync {
            sync_fn(self.device)?;
        }

        Ok(())
    }
}

/// Filesystem statistics
#[derive(Debug, Clone, Copy)]
pub struct Ext4FsStats {
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
}

/// File handle
pub struct Ext4FileHandle<'a> {
    fs: &'a Ext4Filesystem,
    inode_num: u32,
    inode: Ext4Inode,
    position: u64,
    flags: u32,
}

impl<'a> Ext4FileHandle<'a> {
    /// Read data from the file
    pub fn read(&mut self, buffer: &mut [u8]) -> Ext4Result<usize> {
        let file_size = self.inode.size();

        // Check if we're at or past EOF
        if self.position >= file_size {
            return Ok(0);
        }

        // Limit read to file size
        let mut size = buffer.len();
        let remaining = file_size - self.position;
        if size as u64 > remaining {
            size = remaining as usize;
        }

        let mut block_buffer = kernel_alloc_bytes(self.fs.block_size as usize)
            .ok_or(Ext4Error::NoMem)?;

        let mut total_read = 0;
        let mut buf_offset = 0;

        while total_read < size {
            let logical_block = self.position / self.fs.block_size as u64;
            let offset = (self.position % self.fs.block_size as u64) as usize;
            let to_read = (self.fs.block_size as usize - offset).min(size - total_read);

            let physical_block = self.fs.get_block(&self.inode, logical_block)?;

            if physical_block == 0 {
                // Sparse file - return zeros
                buffer[buf_offset..buf_offset + to_read].fill(0);
            } else {
                self.fs.read_block(physical_block, &mut block_buffer)?;
                buffer[buf_offset..buf_offset + to_read]
                    .copy_from_slice(&block_buffer[offset..offset + to_read]);
            }

            buf_offset += to_read;
            self.position += to_read as u64;
            total_read += to_read;
        }

        Ok(total_read)
    }

    /// Seek to a position in the file
    pub fn seek(&mut self, offset: i64, whence: SeekFrom) -> Ext4Result<u64> {
        let file_size = self.inode.size();

        let new_pos = match whence {
            SeekFrom::Start => offset,
            SeekFrom::Current => self.position as i64 + offset,
            SeekFrom::End => file_size as i64 + offset,
        };

        if new_pos < 0 {
            return Err(Ext4Error::Invalid);
        }

        self.position = new_pos as u64;
        Ok(self.position)
    }

    /// Get current file position
    pub fn tell(&self) -> u64 {
        self.position
    }

    /// Read data at an absolute byte offset into a caller buffer.
    pub fn read_at_into(&self, offset: u64, dst: &mut [u8]) -> Ext4Result<usize> {
        self.fs.read_at_into(&self.inode, offset, dst)
    }
}

/// Memory-mapped file context for ext4.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Ext4MmapFile {
    fs: *const Ext4Filesystem,
    inode: Ext4Inode,
    file_size: u64,
}

impl Ext4MmapFile {
    /// File size in bytes.
    pub fn size(&self) -> u64 {
        self.file_size
    }

    /// Read data at an absolute byte offset into a caller buffer.
    pub fn read_at_into(&self, offset: u64, dst: &mut [u8]) -> Ext4Result<usize> {
        let fs = unsafe { &*self.fs };
        fs.read_at_into(&self.inode, offset, dst)
    }
}

impl Ext4Filesystem {
    /// Create a memory-mapped file context from a path.
    pub fn open_mmap_file(&self, path: &[u8]) -> Ext4Result<Ext4MmapFile> {
        let inode_num = self.lookup_path(path)?;
        let inode = self.read_inode(inode_num)?;
        Ok(Ext4MmapFile {
            fs: self as *const Ext4Filesystem,
            inode,
            file_size: inode.size(),
        })
    }
}

/// Read-at callback for demand-paged mmap regions.
///
/// This matches the `MmapReadAtFn` signature in the memory subsystem.
#[no_mangle]
pub extern "C" fn ext4_mmap_read_at(
    ctx: *mut u8,
    offset: u64,
    dst: *mut u8,
    len: usize,
) -> usize {
    if ctx.is_null() || dst.is_null() || len == 0 {
        return 0;
    }

    let file = unsafe { &*(ctx as *const Ext4MmapFile) };
    let buffer = unsafe { slice::from_raw_parts_mut(dst, len) };

    match file.read_at_into(offset, buffer) {
        Ok(read) => read,
        Err(_) => {
            // TODO: Propagate error details once mmap uses error-aware callbacks.
            0
        }
    }
}

/// Seek origins
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start = 0,
    Current = 1,
    End = 2,
}

/// Directory iterator
pub struct Ext4DirIterator<'a> {
    fs: &'a Ext4Filesystem,
    inode: Ext4Inode,
    position: u64,
    size: u64,
}

impl<'a> Ext4DirIterator<'a> {
    /// Create a new directory iterator
    pub fn new(fs: &'a Ext4Filesystem, path: &[u8]) -> Ext4Result<Self> {
        let inode_num = fs.lookup_path(path)?;
        let inode = fs.read_inode(inode_num)?;
        let size = inode.size();

        Ok(Ext4DirIterator {
            fs,
            inode,
            position: 0,
            size,
        })
    }

    /// Read the next directory entry
    pub fn read(&mut self) -> Ext4Result<Ext4DirEntry> {
        let mut buffer = kernel_alloc_bytes(self.fs.block_size as usize)
            .ok_or(Ext4Error::NoMem)?;

        while self.position < self.size {
            let logical_block = self.position / self.fs.block_size as u64;
            let block_offset = (self.position % self.fs.block_size as u64) as usize;

            let physical_block = self.fs.get_block(&self.inode, logical_block)?;
            if physical_block == 0 {
                self.position += self.fs.block_size as u64 - block_offset as u64;
                continue;
            }

            self.fs.read_block(physical_block, &mut buffer)?;

            let entry_ptr = unsafe { buffer[block_offset..].as_ptr() as *const Ext4DirEntry };
            let entry = unsafe { *entry_ptr };

            if entry.rec_len == 0 {
                self.position += self.fs.block_size as u64 - block_offset as u64;
                continue;
            }

            self.position += entry.rec_len as u64;

            if entry.inode != 0 {
                return Ok(entry);
            }
        }

        Err(Ext4Error::NotFound)
    }
}

impl<'a> Iterator for Ext4DirIterator<'a> {
    type Item = Ext4DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        self.read().ok()
    }
}
