# EXT4 Driver Quick Reference

## Basic Usage Examples

### 1. Define Block Device Operations

```rust
use ext4journaling::{Ext4BlockOps, Ext4Result, Ext4Error};

fn my_read_block(dev: *mut u8, block: u64, buffer: &mut [u8]) -> Ext4Result<()> {
    // Your block device read implementation
    // Read 'buffer.len()' bytes from 'block' on device 'dev'
    Ok(())
}

fn my_write_block(dev: *mut u8, block: u64, buffer: &[u8]) -> Ext4Result<()> {
    // Your block device write implementation
    Ok(())
}

fn my_sync(dev: *mut u8) -> Ext4Result<()> {
    // Flush device caches
    Ok(())
}

let ops = Ext4BlockOps {
    read_block: Some(my_read_block),
    write_block: Some(my_write_block),
    sync: Some(my_sync),
};
```

### 2. Mount Filesystem

```rust
use ext4journaling::Ext4Filesystem;

// device_ptr is your block device pointer
let device_ptr: *mut u8 = get_block_device();

match Ext4Filesystem::mount(device_ptr, ops) {
    Ok(mut fs) => {
        // Filesystem mounted successfully
        println!("Mounted EXT4 filesystem");
    }
    Err(e) => {
        println!("Mount failed: {:?}", e);
    }
}
```

### 3. Read a File

```rust
// Open file
let mut file = fs.open(b"/path/to/file.txt", 0)?;

// Read data
let mut buffer = [0u8; 4096];
let bytes_read = file.read(&mut buffer)?;
println!("Read {} bytes", bytes_read);

// Seek to position
file.seek(100, ext4journaling::SeekFrom::Start)?;

// Get current position
let pos = file.tell();
```

### 4. List Directory

```rust
use ext4journaling::Ext4DirIterator;

let iter = Ext4DirIterator::new(&fs, b"/home")?;

for entry in iter {
    let name = entry.name_bytes();
    let name_str = core::str::from_utf8(name).unwrap_or("<invalid>");
    
    let file_type = match entry.file_type {
        ext4journaling::EXT4_FT_REG_FILE => "file",
        ext4journaling::EXT4_FT_DIR => "directory",
        ext4journaling::EXT4_FT_SYMLINK => "symlink",
        _ => "other",
    };
    
    println!("{}: {}", name_str, file_type);
}
```

### 5. Get File Information

```rust
let inode = fs.stat(b"/etc/config.txt")?;

println!("File size: {} bytes", inode.size());
println!("UID: {}", inode.uid());
println!("GID: {}", inode.gid());
println!("Is directory: {}", inode.is_dir());
println!("Is file: {}", inode.is_file());
println!("Uses extents: {}", inode.uses_extents());
```

### 6. Get Filesystem Statistics

```rust
let stats = fs.statfs();

println!("Total blocks: {}", stats.total_blocks);
println!("Free blocks: {}", stats.free_blocks);
println!("Total inodes: {}", stats.total_inodes);
println!("Free inodes: {}", stats.free_inodes);

let used_percent = ((stats.total_blocks - stats.free_blocks) * 100) 
                   / stats.total_blocks;
println!("Usage: {}%", used_percent);
```

### 7. Write Operations (with Journaling)

```rust
// Read inode
let mut inode = fs.read_inode(inode_number)?;

// Modify inode
inode.i_mtime = current_timestamp();

// Write with journaling
fs.write_inode(inode_number, &inode)?;

// Commit transaction
fs.journal_commit()?;
```

### 8. Unmount Filesystem

```rust
// This commits any pending journal transactions
// and syncs all data to disk
fs.unmount()?;
```

## Error Handling

```rust
use ext4journaling::{Ext4Error, Ext4Result};

fn handle_ext4_operation() -> Ext4Result<()> {
    let fs = Ext4Filesystem::mount(device, ops)?;
    
    match fs.open(b"/file.txt", 0) {
        Ok(file) => { /* use file */ }
        Err(Ext4Error::NotFound) => {
            println!("File not found");
        }
        Err(Ext4Error::Permission) => {
            println!("Permission denied");
        }
        Err(e) => {
            println!("Other error: {:?}", e);
        }
    }
    
    Ok(())
}
```

## Advanced: Path Resolution

```rust
// The driver handles path resolution automatically
let inode_num = fs.lookup_path(b"/usr/local/bin/app")?;
let inode = fs.read_inode(inode_num)?;

// Manual directory traversal
let root_inode = fs.read_inode(ext4journaling::EXT4_ROOT_INODE)?;
let usr_inode_num = fs.dir_find_entry(&root_inode, b"usr")?;
let usr_inode = fs.read_inode(usr_inode_num)?;
let local_inode_num = fs.dir_find_entry(&usr_inode, b"local")?;
```

## Advanced: Block Mapping

```rust
// Get physical block for logical block
let inode = fs.read_inode(inode_num)?;
let logical_block = 100;
let physical_block = fs.get_block(&inode, logical_block)?;

// For sparse files, physical_block will be 0
if physical_block == 0 {
    println!("Sparse block - contains zeros");
}
```

## Constants Reference

### File Types
- `EXT4_FT_UNKNOWN`: Unknown file type
- `EXT4_FT_REG_FILE`: Regular file
- `EXT4_FT_DIR`: Directory
- `EXT4_FT_CHRDEV`: Character device
- `EXT4_FT_BLKDEV`: Block device
- `EXT4_FT_FIFO`: FIFO
- `EXT4_FT_SOCK`: Socket
- `EXT4_FT_SYMLINK`: Symbolic link

### Special Inodes
- `EXT4_ROOT_INODE`: Root directory inode (2)
- `EXT4_GOOD_OLD_INODE_SIZE`: Default inode size (128)

### Block Sizes
- `EXT4_MIN_BLOCK_SIZE`: Minimum block size (1024)
- `EXT4_MAX_BLOCK_SIZE`: Maximum block size (65536)

### Limits
- `EXT4_NAME_LEN`: Maximum filename length (255)

## Performance Tips

1. **Buffer Size**: Use buffers matching block size for best performance
   ```rust
   let block_size = fs.block_size as usize;
   let mut buffer = vec![0u8; block_size];
   ```

2. **Sequential Reads**: Read files sequentially when possible
   ```rust
   loop {
       let n = file.read(&mut buffer)?;
       if n == 0 { break; }
       process_data(&buffer[..n]);
   }
   ```

3. **Journal Batching**: Batch multiple writes before committing
   ```rust
   for inode_num in modified_inodes {
       fs.write_inode(inode_num, &inode)?;
   }
   fs.journal_commit()?; // Commit all at once
   ```

4. **Directory Caching**: Cache directory listings if needed
   ```rust
   let entries: Vec<_> = Ext4DirIterator::new(&fs, path)?
       .collect();
   // Now entries can be reused without re-reading
   ```

## Safety Notes

- All public APIs are safe to call
- `unsafe` is only used internally for low-level operations
- Memory is automatically managed via RAII
- No manual cleanup required
- Drop trait ensures proper cleanup even on panic

## Integration Example

```rust
// In your kernel's VFS layer
pub struct VfsFile {
    ext4_file: Option<ext4journaling::Ext4FileHandle>,
    // other filesystem types...
}

impl VfsFile {
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize, VfsError> {
        if let Some(ref mut f) = self.ext4_file {
            f.read(buffer).map_err(|e| VfsError::from(e))
        } else {
            Err(VfsError::InvalidFileType)
        }
    }
}
```
