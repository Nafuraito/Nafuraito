# LA57 (5-Level Paging) Implementation

## Overview

This document describes the LA57 (5-level paging) implementation in Nafuraito OS. LA57 extends the x86-64 virtual address space from 48 bits (256 TiB) to 57 bits (128 PiB).

## Features

- **Automatic Detection**: CPU feature detection via CPUID
- **Dynamic Initialization**: Automatically enables LA57 if supported by the CPU
- **Fallback Support**: Gracefully falls back to 4-level paging on older CPUs
- **Runtime Information**: Kernel displays paging mode at boot

## Architecture

### 4-Level Paging (LA48)
```
Virtual Address (48 bits used)
├── PML4 Index [47:39] (9 bits) → 512 entries
├── PDPT Index [38:30] (9 bits) → 512 entries
├── PD Index   [29:21] (9 bits) → 512 entries
├── PT Index   [20:12] (9 bits) → 512 entries
└── Offset     [11:0]  (12 bits) → 4KB page

Address Space: 256 TiB (2^48 bytes)
```

### 5-Level Paging (LA57)
```
Virtual Address (57 bits used)
├── PML5 Index [56:48] (9 bits) → 512 entries
├── PML4 Index [47:39] (9 bits) → 512 entries
├── PDPT Index [38:30] (9 bits) → 512 entries
├── PD Index   [29:21] (9 bits) → 512 entries
├── PT Index   [20:12] (9 bits) → 512 entries
└── Offset     [11:0]  (12 bits) → 4KB page

Address Space: 128 PiB (2^57 bytes)
```

## Implementation Details

### Bootloader (bridge.asm)

#### CPU Feature Detection
```asm
check_cpu_features:
    ; Check CPUID.07H:ECX.LA57[bit 16]
    mov eax, 7
    xor ecx, ecx
    cpuid
    test ecx, (1 << 16)
    jz .no_la57
    mov byte [cpu_la57_supported], 1
```

#### Paging Setup
The bootloader dynamically chooses between 4-level and 5-level paging:

**4-Level Paging Setup** (`setup_paging_4level`):
- PML4 at `0x10000`
- PDPT at `0x11000-0x11018` (4 entries for 4GB)
- PD at `0x12000-0x15FFF` (4 tables, 2048 entries total)
- Identity maps first 4GB with 2MB pages
- Memory usage: 24KB

**5-Level Paging Setup** (`setup_paging_5level`):
- PML5 at `0x10000` (new root)
- PML4 at `0x11000`
- PDPT at `0x12000-0x12018` (4 entries for 4GB)
- PD at `0x13000-0x16FFF` (4 tables, 2048 entries total)
- Identity maps first 4GB with 2MB pages
- Memory usage: 32KB

#### LA57 Enablement
```asm
; Enable LA57 in CR4 before enabling paging
mov eax, cr4
or eax, (1 << 5)            ; PAE bit
or eax, (1 << 12)           ; LA57 bit (if supported)
mov cr4, eax

; Load root page table (PML4 or PML5)
mov eax, [paging_root_addr]
mov cr3, eax

; Enable paging
mov eax, cr0
or eax, (1 << 31)           ; PG bit
mov cr0, eax
```

### Kernel (paging.rs)

#### CPUID Detection
```rust
pub fn check_la57_support() -> bool {
    // CPUID.07H:ECX.LA57[bit 16]
    let result = cpuid(0x07, 0);
    (result.ecx & (1 << 16)) != 0
}

pub fn is_la57_enabled() -> bool {
    let cr4 = read_cr4();
    (cr4 & (1 << 12)) != 0
}
```

#### Page Table Structures
```rust
// Page table entry (64 bits)
pub struct PageTableEntry(u64);

// Flags
pub const PRESENT: u64 = 1 << 0;     // P
pub const WRITABLE: u64 = 1 << 1;    // R/W
pub const USER: u64 = 1 << 2;        // U/S
pub const HUGE_PAGE: u64 = 1 << 7;   // PS (2MB/1GB)
pub const GLOBAL: u64 = 1 << 8;      // G
pub const NO_EXECUTE: u64 = 1 << 63; // NX

// Page table (512 entries, 4KB aligned)
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; 512],
}
```

#### Virtual Address Decomposition
```rust
pub struct VirtAddr {
    addr: u64,
}

impl VirtAddr {
    pub const fn pml5_index(&self) -> usize {
        ((self.addr >> 48) & 0x1FF) as usize
    }
    
    pub const fn pml4_index(&self) -> usize {
        ((self.addr >> 39) & 0x1FF) as usize
    }
    
    pub const fn pdpt_index(&self) -> usize {
        ((self.addr >> 30) & 0x1FF) as usize
    }
    
    pub const fn pd_index(&self) -> usize {
        ((self.addr >> 21) & 0x1FF) as usize
    }
    
    pub const fn pt_index(&self) -> usize {
        ((self.addr >> 12) & 0x1FF) as usize
    }
    
    pub const fn page_offset(&self) -> usize {
        (self.addr & 0xFFF) as usize
    }
}
```

#### Initialization
```rust
pub fn paging_init(root_table_phys: u64) -> Result<(), &'static str> {
    let la57_supported = check_la57_support();
    let nx_supported = check_nx_support();
    let mode = if is_la57_enabled() {
        PagingMode::FiveLevel
    } else {
        PagingMode::FourLevel
    };

    let manager = PagingManager {
        mode,
        root_table_phys,
        la57_supported,
        nx_supported,
    };

    unsafe {
        PAGING_MANAGER = Some(manager);
    }

    Ok(())
}
```

## Kernel Output

When the kernel boots, it displays paging information:
```
Initializing Paging...
  Current CR3: 0x10000
  Paging init: OK
  Mode: 5-level (LA57)    [or "4-level (LA48)"]
  LA57 support: Yes       [CPU capability]
  LA57 enabled: Yes       [Current state]
  NX support:   Yes       [No-Execute bit]
```

## Benefits of LA57

1. **Massive Address Space**: 128 PiB vs 256 TiB
   - Enables huge databases and in-memory computing
   - Supports large-scale virtualization
   - Future-proofs OS for growing memory needs

2. **Improved Virtualization**: 
   - Better nested paging performance
   - More efficient guest memory management

3. **Enterprise Features**:
   - Support for systems with >256 TiB RAM
   - Better memory isolation
   - Enhanced security boundaries

## CPU Support

LA57 is supported on:
- Intel Ice Lake (10th gen+) server CPUs
- Intel Sapphire Rapids and newer
- Future AMD processors with 5-level paging

## Performance Considerations

1. **TLB Coverage**: One additional level of page tables
   - Slightly higher TLB miss penalty
   - Mitigated by huge pages (2MB, 1GB)

2. **Memory Overhead**: 
   - 4-level: ~24KB for 4GB identity map
   - 5-level: ~32KB for 4GB identity map
   - Extra ~8KB per 512GB of address space

3. **Cache Effects**: 
   - One more memory access on TLB miss
   - Negligible for most workloads
   - Offset by larger addressable space

## Testing

The implementation has been tested with:
- QEMU emulation (4-level fallback)
- Build verification (symbols present)
- Bootloader integration (detection working)

To test with LA57 enabled in QEMU:
```bash
qemu-system-x86_64 -cpu Icelake-Server \
    -drive file=build/disk.img,format=raw \
    -m 4G
```

## Future Work

- [ ] Complete page table walk implementation
- [ ] Full map_page/unmap_page functions
- [ ] Page fault handler integration
- [ ] Huge page support (2MB, 1GB)
- [ ] Address translation functions
- [ ] PCID (Process Context ID) support
- [ ] INVPCID instruction utilization

## References

- Intel SDM Volume 3A, Section 4.5: "4-Level Paging and 5-Level Paging"
- Intel SDM Volume 2, CPUID instruction (Leaf 07H)
- AMD APM Volume 2, Chapter 5: "Page Translation and Protection"

## File Locations

- Bootloader: `/src/bootloader/stage2/bridge.asm`
- Paging Module: `/src/kernel/memory/paging.rs`
- Memory Module: `/src/kernel/memory/mod.rs`
- Kernel Entry: `/src/kernel/main.rs`
