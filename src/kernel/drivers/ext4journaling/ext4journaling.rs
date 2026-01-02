use core::ffi::{c_char, c_int, c_void};
use core::ptr;

//! EXT4 Journaling Filesystem Driver - Rust FFI Bindings
//!
//! This module provides safe Rust bindings to the EXT4 filesystem driver
//! with journaling (JBD2) support for the Nafuraito kernel.

#![no_std]
#![allow(non_camel_case_types)]


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
    pub fn from_code(code: c_int) -> Self {
        match code {
            0 => Ext4Error::Success,
            -1 => Ext4Error::InvalidMagic,
            -2 => Ext4Error::Io,
            -3 => Ext4Error::NoMem,
            -4 => Ext4Error::NotFound,
            -5 => Ext4Error::Invalid,
            -6 => Ext4Error::Journal,
            -7 => Ext4Error::NoSpace,
            -8 => Ext4Error::Permission,
            _ => Ext4Error::Invalid,
        }
    }

    pub fn is_success(&self) -> bool {
        *self == Ext4Error::Success
    }
}

/// Result type for EXT4 operations
pub type Ext4Result<T> = Result<T, Ext4Error>;

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
    pub s_volume_name: [c_char; 16],
    pub s_last_mounted: [c_char; 64],
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
    pub name: [c_char; EXT4_NAME_LEN],
}

impl Ext4DirEntry {
    /// Get the name as a byte slice
    pub fn name_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(self.name.as_ptr() as *const u8, self.name_len as usize)
        }
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

/// EXT4 Extent Index (for internal nodes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Ext4ExtentIdx {
    pub ei_block: u32,
    pub ei_leaf_lo: u32,
    pub ei_leaf_hi: u16,
    pub ei_unused: u16,
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

/// Opaque type for ext4_fs_t
#[repr(C)]
pub struct Ext4Fs {
    _opaque: [u8; 0],
}

/// Opaque type for ext4_file_t
#[repr(C)]
pub struct Ext4File {
    _opaque: [u8; 0],
}

/// Opaque type for ext4_dir_iterator_t
#[repr(C)]
pub struct Ext4DirIterator {
    _opaque: [u8; 0],
}

/// Block device operations callback type
pub type ReadBlockFn =
    unsafe extern "C" fn(dev: *mut c_void, block: u64, buffer: *mut c_void, count: usize) -> c_int;
pub type WriteBlockFn = unsafe extern "C" fn(
    dev: *mut c_void,
    block: u64,
    buffer: *const c_void,
    count: usize,
) -> c_int;
pub type SyncFn = unsafe extern "C" fn(dev: *mut c_void) -> c_int;

/// Block Device Operations structure
#[repr(C)]
pub struct Ext4BlockOps {
    pub read_block: Option<ReadBlockFn>,
    pub write_block: Option<WriteBlockFn>,
    pub sync: Option<SyncFn>,
}

// FFI declarations for C functions
extern "C" {
    fn ext4_mount(
        device: *mut c_void,
        ops: *mut Ext4BlockOps,
        fs_out: *mut *mut Ext4Fs,
    ) -> c_int;

    fn ext4_unmount(fs: *mut Ext4Fs) -> c_int;

    fn ext4_open(
        fs: *mut Ext4Fs,
        path: *const c_char,
        flags: u32,
        file_out: *mut *mut Ext4File,
    ) -> c_int;

    fn ext4_close(file: *mut Ext4File) -> c_int;

    fn ext4_read(
        file: *mut Ext4File,
        buffer: *mut c_void,
        size: usize,
        bytes_read: *mut usize,
    ) -> c_int;

    fn ext4_seek(file: *mut Ext4File, offset: i64, whence: c_int) -> c_int;

    fn ext4_tell(file: *mut Ext4File) -> u64;

    fn ext4_lookup_path(fs: *mut Ext4Fs, path: *const c_char, inode_out: *mut u32) -> c_int;

    fn ext4_dir_find_entry(
        fs: *mut Ext4Fs,
        dir_inode: *mut Ext4Inode,
        name: *const c_char,
        inode_out: *mut u32,
    ) -> c_int;

    fn ext4_dir_open(
        fs: *mut Ext4Fs,
        path: *const c_char,
        iter_out: *mut *mut Ext4DirIterator,
    ) -> c_int;

    fn ext4_dir_read(iter: *mut Ext4DirIterator, entry: *mut Ext4DirEntry) -> c_int;

    fn ext4_dir_close(iter: *mut Ext4DirIterator) -> c_int;

    fn ext4_stat(fs: *mut Ext4Fs, path: *const c_char, inode_out: *mut Ext4Inode) -> c_int;

    fn ext4_statfs(
        fs: *mut Ext4Fs,
        total_blocks: *mut u64,
        free_blocks: *mut u64,
        total_inodes: *mut u64,
        free_inodes: *mut u64,
    ) -> c_int;
}

/// Seek origins
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start = 0,
    Current = 1,
    End = 2,
}

/// Safe wrapper for EXT4 filesystem handle
pub struct Ext4Filesystem {
    inner: *mut Ext4Fs,
}

impl Ext4Filesystem {
    /// Mount an EXT4 filesystem
    ///
    /// # Safety
    /// The device pointer and block ops must remain valid for the lifetime of the filesystem.
    pub unsafe fn mount(device: *mut c_void, ops: &mut Ext4BlockOps) -> Ext4Result<Self> {
        let mut fs: *mut Ext4Fs = ptr::null_mut();
        let result = ext4_mount(device, ops as *mut Ext4BlockOps, &mut fs);
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(Ext4Filesystem { inner: fs })
        } else {
            Err(err)
        }
    }

    /// Open a file by path
    pub fn open(&mut self, path: &[u8], flags: u32) -> Ext4Result<Ext4FileHandle> {
        let mut file: *mut Ext4File = ptr::null_mut();
        // Ensure path is null-terminated
        let result = unsafe { ext4_open(self.inner, path.as_ptr() as *const c_char, flags, &mut file) };
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(Ext4FileHandle { inner: file })
        } else {
            Err(err)
        }
    }

    /// Lookup a path and return the inode number
    pub fn lookup_path(&mut self, path: &[u8]) -> Ext4Result<u32> {
        let mut inode: u32 = 0;
        let result = unsafe { ext4_lookup_path(self.inner, path.as_ptr() as *const c_char, &mut inode) };
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(inode)
        } else {
            Err(err)
        }
    }

    /// Open a directory for iteration
    pub fn open_dir(&mut self, path: &[u8]) -> Ext4Result<Ext4DirHandle> {
        let mut iter: *mut Ext4DirIterator = ptr::null_mut();
        let result = unsafe { ext4_dir_open(self.inner, path.as_ptr() as *const c_char, &mut iter) };
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(Ext4DirHandle { inner: iter })
        } else {
            Err(err)
        }
    }

    /// Get file/inode statistics
    pub fn stat(&mut self, path: &[u8]) -> Ext4Result<Ext4Inode> {
        let mut inode = unsafe { core::mem::zeroed::<Ext4Inode>() };
        let result = unsafe { ext4_stat(self.inner, path.as_ptr() as *const c_char, &mut inode) };
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(inode)
        } else {
            Err(err)
        }
    }

    /// Get filesystem statistics
    pub fn statfs(&mut self) -> Ext4Result<Ext4FsStats> {
        let mut total_blocks: u64 = 0;
        let mut free_blocks: u64 = 0;
        let mut total_inodes: u64 = 0;
        let mut free_inodes: u64 = 0;

        let result = unsafe {
            ext4_statfs(
                self.inner,
                &mut total_blocks,
                &mut free_blocks,
                &mut total_inodes,
                &mut free_inodes,
            )
        };
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(Ext4FsStats {
                total_blocks,
                free_blocks,
                total_inodes,
                free_inodes,
            })
        } else {
            Err(err)
        }
    }

    /// Find an entry in a directory by inode
    pub fn dir_find_entry(&mut self, dir_inode: &mut Ext4Inode, name: &[u8]) -> Ext4Result<u32> {
        let mut inode: u32 = 0;
        let result = unsafe {
            ext4_dir_find_entry(
                self.inner,
                dir_inode as *mut Ext4Inode,
                name.as_ptr() as *const c_char,
                &mut inode,
            )
        };
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(inode)
        } else {
            Err(err)
        }
    }

    /// Get the raw pointer (for advanced usage)
    pub fn as_ptr(&self) -> *mut Ext4Fs {
        self.inner
    }
}

impl Drop for Ext4Filesystem {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                ext4_unmount(self.inner);
            }
        }
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

/// Safe wrapper for EXT4 file handle
pub struct Ext4FileHandle {
    inner: *mut Ext4File,
}

impl Ext4FileHandle {
    /// Read data from the file
    pub fn read(&mut self, buffer: &mut [u8]) -> Ext4Result<usize> {
        let mut bytes_read: usize = 0;
        let result = unsafe {
            ext4_read(
                self.inner,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len(),
                &mut bytes_read,
            )
        };
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(bytes_read)
        } else {
            Err(err)
        }
    }

    /// Seek to a position in the file
    pub fn seek(&mut self, offset: i64, whence: SeekFrom) -> Ext4Result<()> {
        let result = unsafe { ext4_seek(self.inner, offset, whence as c_int) };
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Get current file position
    pub fn tell(&self) -> u64 {
        unsafe { ext4_tell(self.inner) }
    }

    /// Get the raw pointer (for advanced usage)
    pub fn as_ptr(&self) -> *mut Ext4File {
        self.inner
    }
}

impl Drop for Ext4FileHandle {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                ext4_close(self.inner);
            }
        }
    }
}

/// Safe wrapper for EXT4 directory iterator
pub struct Ext4DirHandle {
    inner: *mut Ext4DirIterator,
}

impl Ext4DirHandle {
    /// Read the next directory entry
    pub fn read(&mut self) -> Ext4Result<Ext4DirEntry> {
        let mut entry = unsafe { core::mem::zeroed::<Ext4DirEntry>() };
        let result = unsafe { ext4_dir_read(self.inner, &mut entry) };
        let err = Ext4Error::from_code(result);
        if err.is_success() {
            Ok(entry)
        } else {
            Err(err)
        }
    }

    /// Get the raw pointer (for advanced usage)
    pub fn as_ptr(&self) -> *mut Ext4DirIterator {
        self.inner
    }
}

impl Drop for Ext4DirHandle {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                ext4_dir_close(self.inner);
            }
        }
    }
}

/// Iterator implementation for directory entries
impl Iterator for Ext4DirHandle {
    type Item = Ext4DirEntry;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read() {
            Ok(entry) => Some(entry),
            Err(_) => None,
        }
    }
}

// Re-export C types with Rust naming convention aliases
pub type Ext4SuperblockT = Ext4Superblock;
pub type Ext4GroupDescT = Ext4GroupDesc;
pub type Ext4InodeT = Ext4Inode;
pub type Ext4DirEntryT = Ext4DirEntry;
pub type Ext4ExtentHeaderT = Ext4ExtentHeader;
pub type Ext4ExtentIdxT = Ext4ExtentIdx;
pub type Ext4ExtentT = Ext4Extent;
pub type Jbd2HeaderT = Jbd2Header;
pub type Jbd2SuperblockT = Jbd2Superblock;
pub type Ext4FsT = Ext4Fs;
pub type Ext4FileT = Ext4File;
pub type Ext4DirIteratorT = Ext4DirIterator;
pub type Ext4BlockOpsT = Ext4BlockOps;
