# Nafuraito OS: BIOS/E820 to UEFI Compatibility Analysis

## EXECUTIVE SUMMARY

The Nafuraito OS codebase is **heavily dependent on BIOS and E820 memory maps** for early boot and memory management. For UEFI compatibility, you'll need to replace or supplement these components with UEFI-native equivalents. This document identifies all BIOS/E820-specific code that needs changes.

---

## 1. MEMORY MAP / E820 SUBSYSTEM

### 1.1 E820 Structures (kernel/src/memory/e820.rs - Lines 1-465)

**Key Structs:**

#### E820Entry (24 bytes per entry) - Lines 160-176
```rust
#[repr(C, packed)]
pub struct E820Entry {
    pub base: u64,              // Base address of region
    pub length: u64,            // Length in bytes
    pub region_type: u32,       // Type: 1=Usable, 2=Reserved, 3=ACPI Reclaimable, 
                                //       4=ACPI NVS, 5=Bad Memory
    pub acpi_extended: u32,     // Extended attributes (ACPI 3.0+)
                                // Bit 0: if clear, ignore entry
                                // Bit 1: if set, non-volatile
}
```

#### E820Type Enum - Lines 114-158
```rust
pub enum E820Type {
    Usable = 1,
    Reserved = 2,
    AcpiReclaimable = 3,       // ← ACPI-specific
    AcpiNvs = 4,               // ← ACPI-specific
    BadMemory = 5,
    Unknown = 0xFF,
}
```

#### Memory Zones - Lines 35-87
```rust
pub enum MemoryZone {
    Dma = 0,        // 0-16MB (ISA DMA)
    Dma32 = 1,      // 16MB-4GB (32-bit DMA)
    Normal = 2,     // 4GB+ (64-bit)
    HighMem = 3,    // Very high memory
}
```

#### MemoryMap Struct - Lines 255-463
```rust
pub struct MemoryMap {
    pub(crate) entries_ptr: *const E820Entry,
    pub(crate) count: usize,
}
```

#### **CRITICAL: MemoryMap::parse() - Lines 271-298**
This is the core E820 parser called from BIOS bootloader:
```rust
pub unsafe fn parse(memory_map_addr: u64) -> Self {
    let entries_ptr = memory_map_addr as *const E820Entry;
    let mut count = 0usize;
    
    while count < MAX_MEMORY_ENTRIES {
        let entry_ptr = entries_ptr.wrapping_add(count);
        let entry = core::ptr::read_unaligned(entry_ptr);
        
        // Valid E820 types are 1-5
        if region_type < 1 || region_type > 5 {
            break;
        }
        if length == 0 {
            break;
        }
        count += 1;
    }
    
    MemoryMap {
        entries_ptr,
        count,
    }
}
```

**Entry count determination:** Scans until it finds an entry with invalid type (not 1-5) or zero length.

**Constants - Lines 17-30:**
```rust
pub const MEMORY_MAP_ADDR: usize = 0x6000;      // Where bootloader puts E820 map
pub const MAX_MEMORY_ENTRIES: usize = 64;       // Max entries supported
pub const DMA_ZONE_END: u64 = 16 * 1024 * 1024; // 16MB
pub const DMA32_ZONE_END: u64 = 4 * 1024 * 1024 * 1024; // 4GB
pub const MAX_NUMA_NODES: usize = 8;
```

---

### 1.2 Global Memory Map Storage (kernel/src/memory/lib.rs - Lines 1-50)

The memory map is stored globally before PMM/paging init:

```rust
pub(crate) struct MemoryMapStorage {
    pub map: MemoryMap,
    pub initialized: bool,
}

pub(crate) static mut MEMORY_MAP_STORAGE: MemoryMapStorage = MemoryMapStorage {
    map: MemoryMap {
        entries_ptr: core::ptr::null(),
        count: 0,
    },
    initialized: false,
};

// Line 47-50: Initialization entry point
#[no_mangle]
pub unsafe fn memory_init(memory_map_addr: u64) {
    MEMORY_MAP_STORAGE.map = MemoryMap::parse(memory_map_addr);
    MEMORY_MAP_STORAGE.initialized = true;
}

// Export functions for C/kernel use
#[no_mangle]
pub unsafe extern "C" fn memory_get_total_usable() -> u64 {
    if MEMORY_MAP_STORAGE.initialized {
        MEMORY_MAP_STORAGE.map.total_usable_memory()
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn memory_get_highest_address() -> u64 {
    if MEMORY_MAP_STORAGE.initialized {
        MEMORY_MAP_STORAGE.map.highest_usable_address()
    } else {
        0
    }
}
```

---

### 1.3 How kernel/src/memory/mod.rs Uses E820 (Lines 1-180)

The mod.rs re-exports all E820 types and provides the entry point:

```rust
pub use self::e820::{
    E820Entry, E820EntryExtended, E820Type, MemoryMap, MemoryZone, NumaNode, 
    DMA32_ZONE_END, DMA_ZONE_END, MAX_NUMA_NODES, MEMORY_MAP_ADDR,
};
```

---

### 1.4 Entry Point in kernel/src/main.rs - Lines 898-905

```rust
unsafe {
    video::print("Initializing Memory... \0");
    memory::init(memory_map_addr);           // Calls memory_init(u64)
    video::print("OK\n\0");

    // Debug: Print highest address
    video::print("  Highest addr: \0");
    print_hex_u64(memory::memory_get_highest_address());
    video::print("\n\0");
}
```

Where `memory_map_addr` comes from the bootloader (register r9).

---

## 2. PHYSICAL MEMORY MANAGER (PMM) - kernel/src/memory/pmm.rs

### 2.1 PMM Initialization (Lines 241-410)

The PMM reads from the global memory map (E820) populated earlier:

**Function: PhysicalMemoryManager::init() - Lines 241-410**

```rust
pub unsafe fn init(memory_map: &MemoryMap) -> Result<(), MemoryError> {
    // Step 1: Get highest address from E820 map
    let highest_address = memory_map.highest_usable_address();
    
    if highest_address == 0 {
        return Err(MemoryError {
            code: 1,
            message: "No memory detected!",
            fatal: true,
        });
    }

    // Step 2: Calculate bitmap size (1 bit per 4KB page)
    let total_frames = (highest_address / PAGE_SIZE as u64) as usize;
    let bitmap_size = (total_frames + 7) / 8;
    let bitmap_size_aligned = ((bitmap_size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;

    // Step 3: Find space for bitmap (after kernel at 10MB)
    const KERNEL_END: u64 = 0x100000 + (9 * 1024 * 1024);
    
    let entry_count = memory_map.entry_count();
    let mut bitmap_base_address: u64 = 0;
    
    for i in 0..entry_count {
        if let Some(entry) = memory_map.get(i) {
            if entry.is_usable() {
                // Find first usable region after kernel
                // ...
                if usable_length >= bitmap_size_aligned as u64 {
                    bitmap_base_address = region_start;
                    found_region = true;
                    break;
                }
            }
        }
    }

    // Step 4: Initialize bitmap (all bits = 1 = "used")
    for byte_idx in 0..bitmap_size {
        core::ptr::write_volatile(bitmap_ptr.add(byte_idx), 0xFF);
    }

    // Step 5: Mark E820 usable regions as free (bits = 0)
    for i in 0..entry_count {
        if let Some(entry) = memory_map.get(i) {
            if entry.is_usable() {
                let start_frame = ((entry.base + PAGE_SIZE - 1) / PAGE_SIZE) as usize;
                let mut end_frame = (entry.end() / PAGE_SIZE) as usize;
                
                for frame in start_frame..end_frame {
                    let byte_idx = frame / 8;
                    let bit_offset = frame % 8;
                    *bitmap_ptr.add(byte_idx) &= !(1 << bit_offset);
                }
            }
        }
    }

    // Step 6: Mark reserved areas as used again
    // - First 1MB (0x0 - 0x100000)
    // - Kernel (0x100000 - 0xA00000)
    // - Kernel stack (31MB - 1MB = 0x1F00000 - 0x2000000)
    // - Bitmap itself
    // - Memory map storage (at 0x500 per BIOS convention)
}
```

### 2.2 C-Compatible Wrapper: pmm_init_c() - Lines 1113-1138

```rust
#[no_mangle]
pub unsafe extern "C" fn pmm_init_c(_memory_map_addr: u64) -> i32 {
    // Check if memory map is initialized (must call memory_init first!)
    if !MEMORY_MAP_STORAGE.initialized {
        PhysicalMemoryManager::serial_print_str("PMM: ERROR - Memory map not initialized!\n");
        return -9;
    }
    
    // Call init with global memory map
    match PhysicalMemoryManager::init(&MEMORY_MAP_STORAGE.map) {
        Ok(()) => {
            if PMM_INSTANCE.is_some() {
                0  // Success
            } else {
                -10
            }
        }
        Err(e) => -(e.code as i32)
    }
}
```

**NOTE:** Despite taking `memory_map_addr` parameter, it **ignores it** and uses the global `MEMORY_MAP_STORAGE` populated by `memory_init()`.

### 2.3 How PMM Is Called in Kernel (kernel/src/main.rs - Lines 983-1002)

```rust
unsafe {
    video::print("Initializing PMM...\n\0");
    video::print("  Memory map addr: \0");
    print_hex_u64(memory_map_addr);
    video::print("\n  Total usable: \0");
    print_hex_u64(memory::memory_get_total_usable());
    video::print("\n  Starting PMM init...\n\0");

    let pmm_result = memory::pmm_init_c(memory_map_addr);
    
    if pmm_result == 0 {
        video::print("OK (\0");
        print_hex_u64(memory::pmm_free_memory());
        video::print(" bytes free)\n\0");
    } else {
        video::print("FAILED (error=\0");
        print_hex_u64((-pmm_result) as u64);
        video::print(")\n\0");
    }
}
```

**Key PMM Constants (Line 39):**
```rust
pub const PAGE_SIZE: usize = 4096; // 4 KB
```

**Hardcoded Reserved Regions in PMM init (Lines 265-379):**
- **First 1MB:** 0x0 - 0x100000 (BIOS/bootloader area)
- **Kernel:** 0x100000 - 0xA00000 (10MB)
- **Kernel stack:** 0x1F00000 - 0x2000000 (31MB-32MB, 1MB size)
- **Bitmap:** Allocated after kernel (typically ~0xA00000+)
- **Memory map:** At 0x500 (BIOS convention)

---

## 3. PAGING / PAGE TABLES - kernel/src/memory/paging.rs

### 3.1 Page Table Setup (Lines 1-55, 156-240)

**Page size constants (Lines 42-48):**
```rust
pub const PAGE_SIZE: usize = 4096;              // 4 KiB
pub const PAGE_SIZE_2MB: usize = 2 * 1024 * 1024;
pub const PAGE_SIZE_1GB: usize = 1024 * 1024 * 1024;
pub const PAGE_TABLE_ENTRIES: usize = 512;
pub const LA48_VIRT_ADDR_BITS: usize = 48;     // 256 TiB (4-level paging)
pub const LA57_VIRT_ADDR_BITS: usize = 57;     // 128 PiB (5-level paging)
```

**PageTableEntry struct (156-203):**
```rust
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry(u64);
```

**PageTable struct (205-240) - 512 entries, 4KB aligned:**
```rust
#[repr(C, align(4096))]
pub struct PageTable {
    entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}
```

### 3.2 Paging Initialization (Lines 507-533)

**paging_init() - Called after PMM init:**
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
        root_table_phys,        // CR3 value passed from bootloader
        la57_supported,
        nx_supported,
    };

    unsafe {
        PAGING_MANAGER = Some(manager);
    }

    Ok(())
}
```

**Critical: root_table_phys comes from CR3 register set by bootloader!**

### 3.3 Page Mapping Function (Lines 629-700)

Maps virtual address to physical address. Uses PMM to allocate intermediate page tables:

```rust
pub unsafe fn map_page(
    virt_addr: VirtAddr,
    phys_addr: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    let manager = get_paging_manager().ok_or("Paging not initialized")?;
    
    // Walks page tables:
    // - If 5-level (LA57): PML5 -> PML4 -> PDPT -> PD -> PT -> Physical page
    // - If 4-level (LA48): PML4 -> PDPT -> PD -> PT -> Physical page
}
```

### 3.4 C Wrapper: paging_init_c() - Lines 1370-1375

```rust
#[no_mangle]
pub extern "C" fn paging_init_c(root_table_phys: u64) -> i32 {
    match paging_init(root_table_phys) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}
```

### 3.5 How Paging Is Initialized in Kernel (kernel/src/main.rs - Lines 906-928)

```rust
unsafe {
    video::print("Initializing Paging...\n\0");
    let cr3 = memory::paging_read_cr3();  // Read CR3 set by bootloader
    video::print("  Current CR3: \0");
    print_hex_u64(cr3);
    video::print("\n\0");

    let paging_result = memory::paging_init_c(cr3);  // Pass CR3 to init
    if paging_result == 0 {
        video::print("  Paging init: OK\n\0");

        // Check paging mode
        let mode = memory::paging_get_mode();
        video::print("  Mode: \0");
        if mode == 1 {
            video::print("5-level (LA57)\0");
        } else if mode == 0 {
            video::print("4-level (LA48)\0");
        }
        video::print("\n\0");
    } else {
        video::print("  Paging init: FAILED\n\0");
    }
}
```

### 3.6 Address Mapping in Bootloader-Set Page Tables

From BIOS bridge.asm (Lines 600-700), the bootloader **identity-maps first 4GB**:
- Virtual address 0x0 -> Physical address 0x0
- Virtual address 0x100000 (kernel) -> Physical address 0x100000
- Virtual address 0x6000 (E820 map) -> Physical address 0x6000

The kernel **assumes this identity mapping exists** and initializes paging based on CR3 set by bootloader.

---

## 4. KERNEL ENTRY POINT: kernel/src/main.rs enter() - Lines 832-1070+

### Full enter() Function (Complete Boot Sequence):

**Function signature (Lines 832-839):**
```rust
pub extern "C" fn enter(
    fb_addr: u32,           // Framebuffer physical address (BIOS VBE)
    pitch: u32,             // Bytes per scanline
    width: u32,             // Screen width in pixels
    height: u32,            // Screen height in pixels
    bpp: u32,               // Bits per pixel
    memory_map_addr: u64,   // ← E820 MAP ADDRESS (from BIOS)
) -> ! {
```

**Register calling convention (System V AMD64 ABI):**
- RDI = fb_addr (32-bit)
- RSI = pitch
- RDX = width
- RCX = height
- R8 = bpp
- R9 = memory_map_addr ← **E820 map address**
- [RSP] = memory_map_entry_count (7th argument on stack)

**Boot sequence (Lines 852-980+):**

1. **Initialize COM1 serial port (Lines 853-861)** - for debug output
2. **Initialize GDT/IDT/PIC (Lines 864-870)** - CPU setup
3. **Initialize video (Lines 873-895)** - framebuffer or VGA text
4. **Initialize Memory subsystem (Lines 896-905):**
   ```rust
   video::print("Initializing Memory... \0");
   memory::init(memory_map_addr);  // Parse E820 from BIOS
   video::print("OK\n\0");
   ```

5. **Initialize Paging (Lines 906-955):**
   ```rust
   let cr3 = memory::paging_read_cr3();  // From bootloader
   let paging_result = memory::paging_init_c(cr3);
   ```

6. **Initialize PAT (Page Attribute Table) (Lines 957-975)** - memory types
7. **Initialize Copy-on-Write (Line 979)**
8. **Initialize PMM (Physical Memory Manager) (Lines 983-1002):**
   ```rust
   let pmm_result = memory::pmm_init_c(memory_map_addr);
   ```

---

## 5. UEFI BOOTLOADER: bootloader/uefi/src/main.c

### 5.1 Complete UEFI Bootloader (Lines 1-272)

**Entry point (Lines 14-18):**
```c
EFI_STATUS EFIAPI efi_main(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable)
{
    InitializeLib(ImageHandle, SystemTable);
    Print(L"Nafuraito UEFI Bootloader\n");
```

**Step 1: Get UEFI Memory Map (Lines 22-51)**
```c
UINTN MemoryMapSize = 0;
EFI_MEMORY_DESCRIPTOR *MemoryMap = NULL;
UINTN MapKey = 0;
UINTN DescriptorSize = 0;
UINT32 DescriptorVersion = 0;

// GetMemoryMap called twice: first to size, second to fill
uefi_call_wrapper(
    SystemTable->BootServices->GetMemoryMap, 5,
    &MemoryMapSize, MemoryMap, &MapKey, &DescriptorSize, &DescriptorVersion
);
// Allocate buffer...
Status = uefi_call_wrapper(
    SystemTable->BootServices->GetMemoryMap, 5,
    &MemoryMapSize, MemoryMap, &MapKey, &DescriptorSize, &DescriptorVersion
);
```

**Step 2: Query GOP (Graphics Output Protocol) for framebuffer (Lines 57-80)**
```c
EFI_GRAPHICS_OUTPUT_PROTOCOL *Gop = NULL;
Status = uefi_call_wrapper(
    SystemTable->BootServices->LocateProtocol, 3,
    &GopGuid, NULL, (VOID **)&Gop
);

if (!EFI_ERROR(Status) && Gop != NULL) {
    FbBase   = Gop->Mode->FrameBufferBase;
    FbPitch  = Gop->Mode->Info->PixelsPerScanLine * 4;
    FbWidth  = Gop->Mode->Info->HorizontalResolution;
    FbHeight = Gop->Mode->Info->VerticalResolution;
}
```

**Step 3: Load kernel from FAT32 ESP (Lines 89-174)**
```c
// Get LoadedImage protocol to find boot device
EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *FileSystem = NULL;
Status = uefi_call_wrapper(
    SystemTable->BootServices->HandleProtocol, 3,
    LoadedImage->DeviceHandle, &SfsGuid, (VOID **)&FileSystem
);

// Open file: \nafuraito\kernel.bin
EFI_FILE *KernelFile = NULL;
Status = uefi_call_wrapper(
    Root->Open, 5,
    Root, &KernelFile,
    L"\\nafuraito\\kernel.bin",
    EFI_FILE_MODE_READ, 0
);

// Allocate pages at 0x100000 (KERNEL_LOAD_ADDR)
EFI_PHYSICAL_ADDRESS KernelPhys = (EFI_PHYSICAL_ADDRESS)KERNEL_LOAD_ADDR;
Status = uefi_call_wrapper(
    SystemTable->BootServices->AllocatePages, 4,
    AllocateAddress, EfiLoaderData,
    (UINTN)KERNEL_RESERVE_PAGES, &KernelPhys
);

// Read kernel into memory
Status = uefi_call_wrapper(
    KernelFile->Read, 3,
    KernelFile, &KernelFileSize, (VOID *)(UINTN)KERNEL_LOAD_ADDR
);
```

**Step 4: Exit Boot Services (Lines 211-226)**
```c
Status = uefi_call_wrapper(
    SystemTable->BootServices->ExitBootServices, 2,
    ImageHandle, MapKey
);
if (EFI_ERROR(Status)) {
    // MapKey may have changed; re-read and retry
}
```

**Step 5: Jump to kernel (Lines 248-271)**
```c
// Set up argument registers (System V AMD64 ABI)
register UINT64 r_rdi __asm__("rdi") = (UINT64)(UINT32)FbBase;  // fb_addr
register UINT64 r_rsi __asm__("rsi") = (UINT64)FbPitch;         // pitch
register UINT64 r_rdx __asm__("rdx") = (UINT64)FbWidth;         // width
register UINT64 r_rcx __asm__("rcx") = (UINT64)FbHeight;        // height
register UINT64 r_r8  __asm__("r8")  = (UINT64)4;               // bpp (bytes)
register UINT64 r_r9  __asm__("r9")  = (UINT64)0;               // NO E820 MAP!

__asm__ volatile (
    "movq $0x1FF0000, %%rsp\n\t"  // 31MB kernel stack
    "andq $-16,       %%rsp\n\t"  // 16-byte align
    "xorq %%rax, %%rax\n\t"
    "pushq %%rax\n\t"             // 7th arg: entry count = 0
    "subq $8, %%rsp\n\t"
    "movq $0x100000, %%rax\n\t"
    "jmpq *%%rax\n\t"             // Jump to kernel at 0x100000
    :: "r"(r_rdi), "r"(r_rsi), "r"(r_rdx), "r"(r_rcx),
       "r"(r_r8),  "r"(r_r9)
    :  "rax", "memory"
);
```

**CRITICAL DIFFERENCE:** UEFI bootloader passes **r9 = 0** (no E820 map) and **[rsp] = 0** (entry count = 0).

The kernel then falls back to VGA text mode since **no framebuffer info** is passed either!

---

## 6. BIOS BOOT INFO PASSING: bootloader/src/stage2/bridge.asm

### 6.1 How BIOS Passes Boot Info to Kernel (Lines 700-792)

**Function load_kernel_c() is called first (Lines 744-745):**
```nasm
; Call C kernel loader (which loads the kernel binary from disk)
call load_kernel_c

; Kernel entry point returned in rax
mov rbx, rax
```

**Then boot info is passed via System V AMD64 ABI (Lines 769-792):**

```nasm
; Set up fresh stack for kernel (16-byte aligned)
mov rsp, 0x2000000
and rsp, ~0xF

; Pass boot info via registers (System V AMD64 ABI):
; rdi = framebuffer address (32-bit, zero-extended)
; rsi = pitch (bytes per scanline)
; rdx = width (pixels)
; rcx = height (pixels)
; r8  = bpp (bytes per pixel, AFTER VBE to bytes conversion)
; r9  = memory_map_addr ← E820 MAP ADDRESS
; [rsp] = memory_map_entry_count (7th argument on stack)

mov edi, [rel vbe_framebuffer_addr]
movzx rsi, word [rel vbe_pitch]
movzx rdx, word [rel vbe_width]
movzx rcx, word [rel vbe_height]
movzx r8, byte [rel vbe_bpp]
mov r9d, [rel memory_map_addr]  ; ← E820 map address (0x500 or 0x6000)

; Push memory map entry count as 7th argument
movzx rax, word [rel memory_map_entries]
push rax                         ; 7th argument: entry count
sub rsp, 8                       ; Align to 16 bytes

; Jump to kernel
call rbx
```

### 6.2 Where Boot Info Is Collected (Lines 60-304)

**E820 Memory Map Collection (Lines 277-304):**
```nasm
get_memory_map:
    mov di, 0x500               ; Memory map destination
    mov [memory_map_addr], di
    xor ebx, ebx
    xor bp, bp                  ; Entry counter

.e820_loop:
    mov eax, 0xE820             ; E820 BIOS call
    mov ecx, 24                 ; Entry size (24 bytes)
    mov edx, 0x534D4150         ; 'SMAP' signature
    int 0x15                    ; Call BIOS

    jc .e820_done               ; Error or done
    cmp eax, 0x534D4150
    jne .e820_done

    inc bp                      ; Increment entry counter
    add di, 24                  ; Next entry position
    test ebx, ebx
    jz .e820_done               ; ebx=0 means done

    jmp .e820_loop

.e820_done:
    mov [memory_map_entries], bp
    ret
```

**VESA/VBE Framebuffer Collection (Lines 160-271):**
```nasm
; Scans available video modes via int 0x10 / 0x4F01
; Selects mode with highest resolution and 32-bit color
; Stores in vbe_framebuffer_addr, vbe_pitch, vbe_width, vbe_height, vbe_bpp
```

**Data Section Globals (Lines 814-845):**
```nasm
[SECTION .data]

global vbe_framebuffer_addr      ; dd (4 bytes)
global vbe_pitch                 ; dw (2 bytes)
global vbe_width                 ; dw (2 bytes)
global vbe_height                ; dw (2 bytes)
global vbe_bpp                   ; db (1 byte)
global memory_map_addr           ; dd (4 bytes)
global memory_map_entries        ; dw (2 bytes)

vbe_framebuffer_addr:   dd 0
vbe_pitch:              dw 0
vbe_width:              dw 0
vbe_height:             dw 0
vbe_bpp:                db 0
                        db 0    ; Padding
memory_map_addr:        dd 0
memory_map_entries:     dw 0
```

---

## 7. BOOTLOADER STRUCTURES: bootloader/src/stage2/bootloader.h (Complete)

```c
#ifndef BOOTLOADER_H
#define BOOTLOADER_H

// Type definitions (Lines 4-13)
typedef unsigned char      uint8_t;
typedef unsigned short     uint16_t;
typedef unsigned int       uint32_t;
typedef unsigned long long uint64_t;
typedef signed char        int8_t;
typedef signed short       int16_t;
typedef signed int         int32_t;
typedef signed long long   int64_t;

#define NULL ((void*)0)

// Boot Information Structures (Lines 22-33)

// VBE/VESA framebuffer info (11 bytes + 1 byte padding = 12 bytes)
typedef struct {
    uint32_t framebuffer_addr;      // Physical address
    uint16_t pitch;                 // Bytes per scanline
    uint16_t width;                 // Pixels
    uint16_t height;                // Pixels
    uint8_t  bpp;                   // Bytes per pixel (NOT bits!)
} vbe_info_t;

// E820 memory map info (4 bytes + 2 bytes + 2 padding = 8 bytes)
typedef struct {
    uint32_t memory_map_addr;       // Physical address of E820 entries
    uint16_t memory_map_entries;    // Number of entries
} memory_info_t;

// ATA/Kernel Loading Constants (Lines 71-99)
#define KERNEL_LOAD_ADDR    0x100000       // 1MB
#define KERNEL_START_SECTOR 64
#define KERNEL_SECTORS      128

#define EXT4_PARTITION_LBA  133120U        // 65 MiB

#define ATA_PRIMARY_DATA     0x1F0
#define ATA_PRIMARY_CMD      0x1F7
#define ATA_CMD_READ_SECTORS 0x20

#define SERIAL_PORT 0x3F8                  // COM1

// Function Declarations (Lines 135-140)
uint64_t load_kernel_c(vbe_info_t* vbe, memory_info_t* mem);  // C bootloader
int ata_wait_ready(void);
int ata_wait_drq(void);
int ata_read_sectors(void* dest, uint64_t lba, uint32_t count);

#endif
```

---

## 8. MEMORY MANAGEMENT IN lib.rs AND mod.rs

### 8.1 kernel/src/memory/lib.rs (Complete - Lines 1-277)

**Purpose:** Standalone memory library for Rust compilation.

**Key components:**
- E820 memory map storage and initialization (lines 28-50)
- Global MemoryMapStorage static
- memory_init() entry point
- PMM re-exports
- Paging re-exports
- PAT (Page Attribute Table) support
- CoW (Copy-on-Write) support
- Demand paging / mmap support

**Critical initialization sequence:**
```rust
pub(crate) static mut MEMORY_MAP_STORAGE: MemoryMapStorage = MemoryMapStorage {
    map: MemoryMap { entries_ptr: core::ptr::null(), count: 0 },
    initialized: false,
};

pub unsafe fn memory_init(memory_map_addr: u64) {
    MEMORY_MAP_STORAGE.map = MemoryMap::parse(memory_map_addr);
    MEMORY_MAP_STORAGE.initialized = true;
}
```

### 8.2 kernel/src/memory/mod.rs (Complete - Lines 1-180)

**Purpose:** Main memory module for kernel.

**Key re-exports:**
- E820 types and constants
- PMM functions
- Paging functions (including paging_init_c)
- PAT functions
- CoW functions
- Demand paging functions

**All functions are declared as extern "C" linkage to be callable from C/bootloader code.**

---

## 9. ACPI/RSDP CODE

### 9.1 Search Results in Kernel

Only **stub references** to ACPI:

**From kernel/src/memory/e820.rs (Lines 119-122, 162, 175):**
```rust
AcpiReclaimable = 3,  // E820 type: can reclaim after reading ACPI tables
AcpiNvs = 4,          // E820 type: ACPI Non-Volatile Storage
acpi_extended: u32,   // ACPI 3.0+ extended attributes field
```

**From kernel/src/memory/pmm.rs (Line 1040):**
```rust
// TODO: Implement NUMA-aware allocation when ACPI SRAT is available
```

**From kernel/src/drivers/pic/pic.rs:**
```rust
// MADT (ACPI Multiple APIC Description Table) STRUCTURES
// (Comment only, no actual implementation)
```

**Conclusion:** **NO ACTUAL ACPI PARSING** - the kernel only handles E820 memory type 3/4 by NOT freeing them.

---

## 10. CRITICAL FLOW DIAGRAM

```
BIOS Boot Sequence
==================

1. Stage 1 Bootloader (512 bytes, MBR)
   └─ Loads Stage 2 to 0x7E00

2. Stage 2 (bridge.asm) - 16-bit REAL MODE
   └─ Calls BIOS int 0x15 / 0xE820 to get memory map
      └─ Stores 24-byte entries at 0x500 (default E820 address)
      └─ Stores entry count
   └─ Calls BIOS int 0x10 / 0x4F01 to get video modes
      └─ Stores framebuffer info
   └─ Transitions to 64-bit LONG MODE
   └─ Loads kernel binary from ATA drive
   └─ Calls C function load_kernel_c(vbe_info_t*, memory_info_t*)
   └─ Jumps to kernel with boot parameters in registers:
      ├─ RDI = framebuffer address (from VESA)
      ├─ RSI = pitch
      ├─ RDX = width
      ├─ RCX = height
      ├─ R8  = bpp
      ├─ R9  = memory_map_addr (0x500 or 0x6000)  ← E820 MAP
      └─ [RSP] = memory_map_entry_count           ← ENTRY COUNT

3. Kernel enter() in kernel/src/main.rs
   └─ Line 898: memory::init(memory_map_addr)
      └─ Calls memory_init(u64) from lib.rs
         └─ Parses E820 memory map from address in R9
            └─ Stores in global MEMORY_MAP_STORAGE
   └─ Line 908: paging_read_cr3()
      └─ Gets page table root from CR3 (set by bootloader)
   └─ Line 913: paging_init_c(cr3)
      └─ Initializes paging manager with bootloader's page table
   └─ Line 990: pmm_init_c(memory_map_addr)
      └─ Uses GLOBAL MEMORY_MAP_STORAGE (NOT parameter!)
      └─ Creates bitmap allocator based on E820 map
      └─ Marks reserved regions as used


UEFI Boot Sequence (bootloader/uefi/src/main.c)
===============================================

1. efi_main() Entry Point
   └─ Calls GetMemoryMap (twice for size negotiation)
      └─ Gets UEFI memory descriptors (different format from E820!)
   └─ Calls LocateProtocol(GOP_GUID)
      └─ Gets GOP framebuffer info
   └─ Calls HandleProtocol(LoadedImage) + HandleProtocol(FileSystem)
      └─ Gets access to boot device FAT32 filesystem
   └─ Loads kernel binary from \nafuraito\kernel.bin
      └─ Allocates pages at 0x100000
      └─ Reads file into memory
   └─ Calls ExitBootServices()
      └─ Point of no return! No UEFI services available
   └─ Jumps to kernel with boot parameters:
      ├─ RDI = framebuffer address (from GOP)
      ├─ RSI = pitch
      ├─ RDX = width
      ├─ RCX = height
      ├─ R8  = 4 (bytes per pixel)
      ├─ R9  = 0 (NO E820 MAP!)  ← PROBLEM!
      └─ [RSP] = 0 (NO ENTRY COUNT!)  ← PROBLEM!

2. Kernel enter() receives R9=0, [RSP]=0
   └─ Calls memory::init(0)  ← ZERO ADDRESS!
      └─ Tries to parse E820 from NULL pointer
      └─ Gets garbage or crashes
```

---

## UEFI COMPATIBILITY CHANGES REQUIRED

### Change 1: Replace E820 Parser
**Current:** kernel/src/memory/e820.rs reads fixed memory at 0x6000
**UEFI:** Must convert UEFI EFI_MEMORY_DESCRIPTOR[] to E820Entry[] format

**Location:** kernel/src/memory/e820.rs - add new function:
```rust
pub unsafe fn parse_uefi(
    descriptors: *const EFI_MEMORY_DESCRIPTOR,
    count: usize,
    descriptor_size: usize,
) -> MemoryMap {
    // Convert UEFI descriptors to E820 format
    // EFI_MEMORY_TYPE -> E820Type mapping
    // Allocate E820Entry array and populate
}
```

### Change 2: Update Memory Init Entry Point
**Current:** kernel/src/memory/lib.rs memory_init() expects BIOS address
**UEFI:** Must accept pre-converted E820 entries

```rust
pub unsafe fn memory_init_uefi(entries: *const E820Entry, count: usize) {
    MEMORY_MAP_STORAGE.map = MemoryMap {
        entries_ptr: entries,
        count,
    };
    MEMORY_MAP_STORAGE.initialized = true;
}
```

### Change 3: Update Bootloader to Pass E820 to Kernel
**Current UEFI bootloader:** Passes R9=0
**Required:** Convert UEFI memory map to E820 and pass in R9

```c
// In bootloader/uefi/src/main.c:
// - Allocate memory for E820Entry[] array
// - Convert EFI_MEMORY_DESCRIPTOR[] to E820Entry[]
// - Pass pointer in R9 register
register UINT64 r_r9 __asm__("r9") = (UINT64)(UINT32)E820MapAddr;
```

### Change 4: Update Kernel Entry Signature (Optional)
Could keep current signature but interpret parameters differently in UEFI path

### Change 5: Handle Paging Setup
**Current:** Assumes bootloader sets up page tables and CR3
**UEFI:** ExitBootServices disables bootloader page tables

Required: Set up kernel's own page tables BEFORE kernel code runs
- Add entry.asm code to:
  1. Detect if running under UEFI (no identity mapping for 0x500)
  2. Set up minimal page tables
  3. Jump to kernel with correct paging

---

## SUMMARY TABLE: BIOS vs UEFI

| Component | BIOS | UEFI |
|-----------|------|------|
| **Memory Map** | E820 BIOS call, stored at 0x500 | EFI_MEMORY_DESCRIPTOR[], from GetMemoryMap |
| **Map Format** | 24-byte entries (base, length, type, flags) | 40+ byte descriptors (phys start, virt start, pages, type, attrs) |
| **Framebuffer** | VESA/VBE BIOS call, int 0x10 | GOP protocol, LocateProtocol |
| **Kernel Loading** | ATA disk read via int 0x13 | FAT32 simple file system via UEFI |
| **Boot Services** | BIOS available until jump to kernel | Disabled after ExitBootServices |
| **Paging** | Bootloader sets up PML4 in CR3 | ExitBootServices invalidates bootloader paging |
| **Memory Map Passed** | R9 register = address | R9 register = 0 (must change) |
| **Entry Count Passed** | [RSP] = count | [RSP] = 0 (must change) |

---

## FILE LOCATIONS SUMMARY

| File | Key Components | UEFI Changes |
|------|----------------|-------------|
| kernel/src/memory/e820.rs | E820Entry struct, MemoryMap parser | Add UEFI descriptor converter |
| kernel/src/memory/lib.rs | memory_init(addr), global storage | Add memory_init_uefi(entries, count) |
| kernel/src/memory/pmm.rs | PhysicalMemoryManager::init() | No changes (uses global storage) |
| kernel/src/memory/paging.rs | paging_init(cr3) | May need paging setup in entry.asm |
| kernel/src/main.rs | enter() function | Detect UEFI path (R9=0) vs BIOS path |
| bootloader/src/stage2/bridge.asm | E820 collection, boot param setup | Keep for BIOS compatibility |
| bootloader/uefi/src/main.c | UEFI bootloader | Convert UEFI map to E820, pass in R9 |
| bootloader/src/stage2/bootloader.h | vbe_info_t, memory_info_t | Already compatible |

---

## FINAL NOTES

1. **No ACPI parsing exists** - the kernel only recognizes E820 type 3/4 but doesn't process them.

2. **PMM depends entirely on E820** - it uses memory map entry count to calculate bitmap size.

3. **Paging setup is minimal** - kernel trusts bootloader's CR3 register value.

4. **UEFI bootloader exists but incomplete** - it doesn't convert UEFI descriptors to E820 format for kernel.

5. **Hardcoded memory addresses** - kernel assumes:
   - First 1MB reserved
   - Kernel at 0x100000 - 0xA00000
   - Kernel stack at 0x1F00000 - 0x2000000
   - Bitmap placed after kernel

6. **Entry point compatibility** - System V AMD64 ABI used, so same function signature works for both BIOS and UEFI if we just change how memory info is passed.

