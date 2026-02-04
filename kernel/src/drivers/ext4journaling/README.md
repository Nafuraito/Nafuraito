# EXT4 Journaling Filesystem Driver - Pure Rust Implementation

## Overview

This is a complete EXT4 filesystem driver with JBD2 journaling support, written entirely in Rust for the Nafuraito kernel. The driver was converted from C to pure Rust to leverage Rust's memory safety guarantees and type system.

## Features

- **Full EXT4 Support**: Read and write operations on EXT4 filesystems
- **Extent Tree Navigation**: Support for extent-based file block mapping
- **Indirect Block Mapping**: Legacy indirect block support for older filesystems
- **JBD2 Journaling**: Full journal support with transaction management and recovery
- **64-bit Support**: Handles filesystems with 64-bit block addressing
- **Safe Memory Management**: Leverages Rust's ownership system for automatic memory cleanup
- **Zero FFI Dependencies**: No C foreign function interface required

## Architecture

### Core Components

1. **Ext4Filesystem**: Main filesystem context managing superblock, block groups, and journal
2. **Journal**: JBD2 journal implementation with transaction support
3. **Ext4FileHandle**: File handle for read/seek operations
4. **Ext4DirIterator**: Directory traversal iterator

### Data Structures

- `Ext4Superblock`: Filesystem metadata and parameters
- `Ext4GroupDesc`: Block group descriptor
- `Ext4Inode`: Inode structure with file metadata
- `Ext4DirEntry`: Directory entry
- `Ext4Extent`: Extent tree node
- `Jbd2Superblock`: Journal superblock
- `Jbd2Header`: Journal block header

## Usage

### Mounting a Filesystem

```rust
use ext4journaling::{Ext4Filesystem, Ext4BlockOps};

let ops = Ext4BlockOps {
    read_block: Some(my_read_block_fn),
    write_block: Some(my_write_block_fn),
    sync: Some(my_sync_fn),
};

let mut fs = Ext4Filesystem::mount(device_ptr, ops)?;
```

### Reading a File

```rust
let mut file = fs.open(b"/path/to/file", 0)?;
let mut buffer = [0u8; 4096];
let bytes_read = file.read(&mut buffer)?;
```

### Listing Directory Contents

```rust
let mut iter = Ext4DirIterator::new(&fs, b"/path/to/directory")?;
for entry in iter {
    let name = entry.name_bytes();
    // Process directory entry
}
```

### Getting File Statistics

```rust
let inode = fs.stat(b"/path/to/file")?;
let file_size = inode.size();
let is_directory = inode.is_dir();
```

### Unmounting

```rust
fs.unmount()?; // Commits journal and syncs
```

## Implementation Details

### Memory Allocation

The driver uses kernel memory allocation through `kmalloc`/`kfree` functions that must be provided by the kernel. The Rust wrappers ensure proper cleanup through RAII patterns.

### Block Device Operations

The driver uses callback functions for block device I/O:
- `read_block`: Read blocks from the device
- `write_block`: Write blocks to the device
- `sync`: Flush device caches

### Journal Transaction Management

The journal implementation supports:
- Transaction start/commit
- Block buffering
- Descriptor and commit block writing
- Crash recovery on mount
- Journal replay

### Extent Tree Navigation

The driver efficiently handles both:
- Modern extent-based block mapping with tree traversal
- Legacy indirect block mapping (single, double, triple indirect)

## Error Handling

All operations return `Ext4Result<T>` which is `Result<T, Ext4Error>`. Error types include:
- `Success`: Operation completed successfully
- `InvalidMagic`: Superblock magic number mismatch
- `Io`: I/O error during block operations
- `NoMem`: Memory allocation failure
- `NotFound`: File or directory not found
- `Invalid`: Invalid parameter or operation
- `Journal`: Journal-related error
- `NoSpace`: Filesystem full
- `Permission`: Permission denied (read-only filesystem)

## Safety Considerations

While the driver uses `unsafe` blocks for:
- Raw pointer manipulation for C-style structures
- Memory-mapped I/O buffer handling
- FFI calls to kernel allocator

The unsafe code is carefully isolated and wrapped in safe abstractions. All public APIs are safe to use.

## Migration from C

The original C implementation has been preserved as:
- `ext4journaling_old.c`: Original C implementation
- `ext4journaling_old.rs`: Old FFI bindings

The new implementation maintains API compatibility while providing improved safety and performance.

## Future Enhancements

- Write support (currently read-only safe)
- Extended attributes support
- Quota management
- Inline data support
- Encryption support
- Checksumming for metadata
- Defragmentation support

## License

This driver is part of the Nafuraito kernel project.
