#include <efi.h>
#include <efilib.h>
#include "boot_info.h"

/* Physical address where the kernel binary is loaded.
 * Must match KERNEL_LOAD_ADDR in the BIOS bootloader (bootloader.h).
 * The linker script places _start (entry.asm) at exactly this address. */
#define KERNEL_LOAD_ADDR      0x100000ULL

/* Pages reserved for kernel binary + BSS (4 MiB; well above any expected size). */
#define KERNEL_RESERVE_PAGES  1024

/* Maximum E820 entries to produce from UEFI memory map conversion. */
#define MAX_E820_ENTRIES      256

/* ============================================================================
 * ACPI RSDP GUIDs
 *
 * These are the vendor GUIDs stored in the UEFI configuration table that
 * point to the ACPI Root System Description Pointer.
 *   - acpi_10: ACPI 1.0 RSDP (8-byte signature "RSD PTR ")
 *   - acpi_20: ACPI 2.0+ RSDP (extended, 36 bytes, preferred)
 * ============================================================================ */
static EFI_GUID acpi_10_guid = {
    0xeb9d2d30, 0x2d88, 0x11d3,
    {0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d}
};
static EFI_GUID acpi_20_guid = {
    0x8868e871, 0xe4f1, 0x11d3,
    {0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81}
};

/* ============================================================================
 * E820 entry (24 bytes) — internal format understood by the kernel's e820.rs.
 * Matches the E820Entry struct in kernel/src/memory/e820.rs exactly.
 * ============================================================================ */
typedef struct __attribute__((packed)) {
    UINT64 base;
    UINT64 length;
    UINT32 region_type; /* 1=usable 2=reserved 3=acpi-reclaim 4=acpi-nvs 5=bad */
    UINT32 acpi_flags;  /* ACPI 3.0 extended attributes; set to 1 (entry valid) */
} e820_entry_t;

/**
 * efi_type_to_e820 — map a UEFI EFI_MEMORY_TYPE to an E820 region type.
 *
 * After ExitBootServices the EfiBootServicesCode/Data regions are reclaimable
 * by the OS, so we mark them as usable (type 1).
 *
 * EfiRuntimeServicesCode/Data must remain mapped (UEFI runtime services need
 * them), so they stay reserved (type 2).
 */
static UINT32 efi_type_to_e820(UINT32 efi_type)
{
    switch (efi_type) {
    /* Memory the OS owns outright */
    case EfiConventionalMemory:
    case EfiLoaderCode:
    case EfiLoaderData:
        return 1;
    /* Boot-services memory is reclaimable once ExitBootServices returns */
    case EfiBootServicesCode:
    case EfiBootServicesData:
        return 1;
    case EfiACPIReclaimMemory:
        return 3;
    case EfiACPIMemoryNVS:
        return 4;
    case EfiUnusableMemory:
        return 5;
    /* EfiPersistentMemory (12) — treat as usable conventional RAM */
    case 14: /* EfiPersistentMemory */
        return 1;
    default:
        /* Reserved: EfiReservedMemoryType, EfiRuntimeServicesCode/Data,
         * EfiMemoryMappedIO, EfiMemoryMappedIOPortSpace, EfiPalCode, etc. */
        return 2;
    }
}

/**
 * find_rsdp — search the UEFI configuration table for the ACPI RSDP.
 *
 * Prefers the ACPI 2.0 RSDP (acpi_20_guid) over 1.0 because ACPI 2.0
 * supports 64-bit physical addresses in the XSDT.
 *
 * Returns the physical address of the RSDP, or 0 if not found.
 */
static UINT64 find_rsdp(EFI_SYSTEM_TABLE *SystemTable)
{
    UINT64 rsdp = 0;
    for (UINTN i = 0; i < SystemTable->NumberOfTableEntries; i++) {
        EFI_GUID *g = &SystemTable->ConfigurationTable[i].VendorGuid;
        if (CompareGuid(g, &acpi_20_guid) == 0) {
            /* ACPI 2.0 found — highest priority, stop searching. */
            return (UINT64)(UINTN)SystemTable->ConfigurationTable[i].VendorTable;
        }
        if (CompareGuid(g, &acpi_10_guid) == 0) {
            /* ACPI 1.0 found — remember it but keep looking for 2.0. */
            rsdp = (UINT64)(UINTN)SystemTable->ConfigurationTable[i].VendorTable;
        }
    }
    return rsdp;
}

/**
 * convert_uefi_map_to_e820 — convert the UEFI memory map to E820 format.
 *
 * Iterates the EFI_MEMORY_DESCRIPTOR array and writes E820 entries to the
 * fixed physical address UEFI_E820_MAP_ADDR (0x7000).  A sentinel entry
 * with region_type == 0 is appended so the kernel's scan loop can detect
 * the end without a separate count.
 *
 * SAFETY: UEFI identity-maps all conventional RAM, including 0x7000, so
 * writing to that physical address is valid before ExitBootServices.
 *
 * Returns the number of entries written (not counting the sentinel).
 */
static UINT32 convert_uefi_map_to_e820(VOID *MemoryMap,
                                        UINTN MemoryMapSize,
                                        UINTN DescriptorSize)
{
    e820_entry_t *dest = (e820_entry_t *)(UINTN)UEFI_E820_MAP_ADDR;

    /* Zero the destination buffer first so the sentinel is implicit. */
    UINT8 *buf = (UINT8 *)(UINTN)UEFI_E820_MAP_ADDR;
    UINTN buf_size = (MAX_E820_ENTRIES + 1) * sizeof(e820_entry_t);
    for (UINTN i = 0; i < buf_size; i++) buf[i] = 0;

    UINTN num_descs = MemoryMapSize / DescriptorSize;
    UINT32 count    = 0;

    for (UINTN i = 0; i < num_descs && count < MAX_E820_ENTRIES; i++) {
        EFI_MEMORY_DESCRIPTOR *d =
            (EFI_MEMORY_DESCRIPTOR *)((UINT8 *)MemoryMap + i * DescriptorSize);

        /* Skip zero-length regions. */
        if (d->NumberOfPages == 0)
            continue;

        dest[count].base        = d->PhysicalStart;
        dest[count].length      = d->NumberOfPages * 4096ULL;
        dest[count].region_type = efi_type_to_e820(d->Type);
        /* ACPI 3.0 extended attribute: bit 0 = entry valid, bit 1 = non-volatile. */
        dest[count].acpi_flags  = 1;
        count++;
    }

    /* Sentinel: type 0 terminates the kernel's scan loop. */
    dest[count].base        = 0;
    dest[count].length      = 0;
    dest[count].region_type = 0;
    dest[count].acpi_flags  = 0;

    return count;
}

EFI_STATUS
EFIAPI
efi_main(EFI_HANDLE ImageHandle, EFI_SYSTEM_TABLE *SystemTable)
{
    EFI_STATUS Status;

    InitializeLib(ImageHandle, SystemTable);
    Print(L"Nafuraito UEFI Bootloader\n");

    /* -------------------------------------------------------------------
     * Step 1: Get UEFI memory map.
     * GetMemoryMap must be called twice: once to size the buffer, once to
     * fill it.  The MapKey from the second call is needed by ExitBootServices.
     * ------------------------------------------------------------------- */
    UINTN MemoryMapSize      = 0;
    EFI_MEMORY_DESCRIPTOR *MemoryMap = NULL;
    UINTN MapKey             = 0;
    UINTN DescriptorSize     = 0;
    UINT32 DescriptorVersion = 0;

    /* First call: probe for required size (returns EFI_BUFFER_TOO_SMALL). */
    uefi_call_wrapper(
        SystemTable->BootServices->GetMemoryMap, 5,
        &MemoryMapSize, MemoryMap, &MapKey, &DescriptorSize, &DescriptorVersion
    );
    MemoryMapSize += 4 * DescriptorSize; /* pad — later allocs add descriptors */
    Status = uefi_call_wrapper(
        SystemTable->BootServices->AllocatePool, 3,
        EfiLoaderData, MemoryMapSize, (VOID **)&MemoryMap
    );
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: AllocatePool for memory map failed\n"); for (;;) {}
    }
    Status = uefi_call_wrapper(
        SystemTable->BootServices->GetMemoryMap, 5,
        &MemoryMapSize, MemoryMap, &MapKey, &DescriptorSize, &DescriptorVersion
    );
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: GetMemoryMap failed\n"); for (;;) {}
    }

    /* -------------------------------------------------------------------
     * Step 2: Query GOP (Graphics Output Protocol) for the linear
     * framebuffer.  The address remains valid after ExitBootServices.
     * ------------------------------------------------------------------- */
    EFI_GUID GopGuid = EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID;
    EFI_GRAPHICS_OUTPUT_PROTOCOL *Gop = NULL;
    Status = uefi_call_wrapper(
        SystemTable->BootServices->LocateProtocol, 3,
        &GopGuid, NULL, (VOID **)&Gop
    );

    /* Defaults — kernel falls back to VGA text mode when fb_addr == 0. */
    UINT64 FbBase   = 0;
    UINT32 FbPitch  = 0;
    UINT32 FbWidth  = 0;
    UINT32 FbHeight = 0;

    if (!EFI_ERROR(Status) && Gop != NULL) {
        FbBase   = Gop->Mode->FrameBufferBase;
        /* PixelsPerScanLine is the stride in pixels; ×4 gives bytes for
         * 32-bit (BGRX/RGBX) linear framebuffer. */
        FbPitch  = Gop->Mode->Info->PixelsPerScanLine * 4;
        FbWidth  = Gop->Mode->Info->HorizontalResolution;
        FbHeight = Gop->Mode->Info->VerticalResolution;
        Print(L"GOP framebuffer found\n");
    } else {
        Print(L"WARNING: GOP not found — kernel will use VGA text mode\n");
    }

    /* -------------------------------------------------------------------
     * Step 3: Load the kernel binary from the ESP.
     *
     * The kernel binary is at \nafuraito\kernel.bin on the FAT32 ESP
     * partition.  UEFI Simple File System reads it without any custom
     * filesystem driver.
     * ------------------------------------------------------------------- */
    EFI_LOADED_IMAGE *LoadedImage = NULL;
    EFI_GUID LiGuid = EFI_LOADED_IMAGE_PROTOCOL_GUID;
    Status = uefi_call_wrapper(
        SystemTable->BootServices->HandleProtocol, 3,
        ImageHandle, &LiGuid, (VOID **)&LoadedImage
    );
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: HandleProtocol(LoadedImage) failed\n"); for (;;) {}
    }

    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *FileSystem = NULL;
    EFI_GUID SfsGuid = EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID;
    Status = uefi_call_wrapper(
        SystemTable->BootServices->HandleProtocol, 3,
        LoadedImage->DeviceHandle, &SfsGuid, (VOID **)&FileSystem
    );
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: HandleProtocol(FileSystem) failed\n"); for (;;) {}
    }

    EFI_FILE *Root = NULL;
    Status = uefi_call_wrapper(FileSystem->OpenVolume, 2, FileSystem, &Root);
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: OpenVolume failed\n"); for (;;) {}
    }

    EFI_FILE *KernelFile = NULL;
    Status = uefi_call_wrapper(
        Root->Open, 5,
        Root, &KernelFile,
        L"\\nafuraito\\kernel.bin",
        EFI_FILE_MODE_READ, 0
    );
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: Cannot open \\nafuraito\\kernel.bin\n"); for (;;) {}
    }

    /* Query file size via GetInfo so we can verify the read later. */
    EFI_FILE_INFO *FileInfo = NULL;
    UINTN InfoSize = SIZE_OF_EFI_FILE_INFO + 256;
    Status = uefi_call_wrapper(
        SystemTable->BootServices->AllocatePool, 3,
        EfiLoaderData, InfoSize, (VOID **)&FileInfo
    );
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: AllocatePool for FileInfo failed\n"); for (;;) {}
    }
    Status = uefi_call_wrapper(
        KernelFile->GetInfo, 4,
        KernelFile, &GenericFileInfo, &InfoSize, (VOID *)FileInfo
    );
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: GetInfo on kernel file failed\n"); for (;;) {}
    }
    UINTN KernelFileSize = (UINTN)FileInfo->FileSize;
    uefi_call_wrapper(SystemTable->BootServices->FreePool, 1, FileInfo);

    /* Allocate KERNEL_RESERVE_PAGES pages at exactly KERNEL_LOAD_ADDR.
     * Extra pages beyond the file size cover the kernel's BSS section
     * (which is not stored in the binary but is zeroed by entry.asm). */
    EFI_PHYSICAL_ADDRESS KernelPhys = (EFI_PHYSICAL_ADDRESS)KERNEL_LOAD_ADDR;
    Status = uefi_call_wrapper(
        SystemTable->BootServices->AllocatePages, 4,
        AllocateAddress, EfiLoaderData,
        (UINTN)KERNEL_RESERVE_PAGES, &KernelPhys
    );
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: AllocatePages at 0x100000 failed\n"); for (;;) {}
    }

    /* Pre-zero the entire reserved region so the BSS is clean before entry.asm
     * runs its own zeroing pass (belt-and-suspenders). */
    UINT8 *KernelMem = (UINT8 *)(UINTN)KERNEL_LOAD_ADDR;
    for (UINTN i = 0; i < (UINTN)KERNEL_RESERVE_PAGES * 4096; i++)
        KernelMem[i] = 0;

    /* Read the kernel binary into memory. */
    Status = uefi_call_wrapper(
        KernelFile->Read, 3,
        KernelFile, &KernelFileSize, (VOID *)(UINTN)KERNEL_LOAD_ADDR
    );
    if (EFI_ERROR(Status)) {
        Print(L"ERROR: Read kernel file failed\n"); for (;;) {}
    }
    uefi_call_wrapper(KernelFile->Close, 1, KernelFile);
    uefi_call_wrapper(Root->Close, 1, Root);

    Print(L"Kernel loaded at 0x100000 — preparing to boot\n");

    /* -------------------------------------------------------------------
     * Step 4: Re-acquire a fresh memory map and MapKey.
     *
     * Every AllocatePool / AllocatePages call above invalidated the old
     * MapKey.  ExitBootServices will fail unless we provide the most
     * recent key.
     * ------------------------------------------------------------------- */
    uefi_call_wrapper(SystemTable->BootServices->FreePool, 1, MemoryMap);
    MemoryMap     = NULL;
    MemoryMapSize = 0;

    uefi_call_wrapper(
        SystemTable->BootServices->GetMemoryMap, 5,
        &MemoryMapSize, MemoryMap, &MapKey, &DescriptorSize, &DescriptorVersion
    );
    MemoryMapSize += 4 * DescriptorSize;
    uefi_call_wrapper(
        SystemTable->BootServices->AllocatePool, 3,
        EfiLoaderData, MemoryMapSize, (VOID **)&MemoryMap
    );
    uefi_call_wrapper(
        SystemTable->BootServices->GetMemoryMap, 5,
        &MemoryMapSize, MemoryMap, &MapKey, &DescriptorSize, &DescriptorVersion
    );

    /* -------------------------------------------------------------------
     * Step 4b: Convert UEFI memory map to E820 format and write BootInfo.
     *
     * This must happen AFTER the final GetMemoryMap (so the map is current)
     * and BEFORE ExitBootServices (the only call that invalidates the MapKey).
     * We do not call any UEFI Boot Service between here and ExitBootServices,
     * so the MapKey remains valid.
     *
     * Two fixed physical addresses are used (both in the first megabyte,
     * which the PMM always marks reserved):
     *   0x7000 — E820 entries converted from UEFI descriptors
     *   0x6000 — BootInfo struct for boot-path detection by the kernel
     * ------------------------------------------------------------------- */

    /* Convert the UEFI memory map to E820 format at UEFI_E820_MAP_ADDR. */
    UINT32 e820_count = convert_uefi_map_to_e820(MemoryMap, MemoryMapSize, DescriptorSize);
    Print(L"Memory map: %d E820 entries written to 0x7000\n", (UINTN)e820_count);

    /* Locate the ACPI RSDP via UEFI configuration tables.
     * This is the authoritative source when UEFI-booted (more reliable than
     * scanning the BIOS ROM area as legacy OSes do). */
    UINT64 rsdp_addr = find_rsdp(SystemTable);
    if (rsdp_addr != 0)
        Print(L"ACPI RSDP found at 0x%lx\n", rsdp_addr);
    else
        Print(L"WARNING: ACPI RSDP not found in UEFI configuration table\n");

    /* Write the BootInfo struct to physical 0x6000.
     * The kernel reads this on startup to detect the boot path. */
    boot_info_t *bi = (boot_info_t *)(UINTN)BOOT_INFO_ADDR;
    bi->magic              = BOOT_INFO_MAGIC;
    bi->boot_type          = BOOT_TYPE_UEFI;
    bi->rsdp_addr          = rsdp_addr;
    bi->mem_map_entry_count = e820_count;
    bi->_reserved0         = 0;
    bi->_reserved1         = 0;

    /* -------------------------------------------------------------------
     * Step 5: Exit Boot Services — point of no return.
     *
     * After this, no UEFI Boot Services are available (no Print, no Alloc).
     * The GOP framebuffer address remains valid.
     * If ExitBootServices returns EFI_INVALID_PARAMETER the memory map
     * changed again; re-read the key and retry once.
     * ------------------------------------------------------------------- */
    Status = uefi_call_wrapper(
        SystemTable->BootServices->ExitBootServices, 2,
        ImageHandle, MapKey
    );
    if (EFI_ERROR(Status)) {
        MemoryMapSize = 0;
        uefi_call_wrapper(
            SystemTable->BootServices->GetMemoryMap, 5,
            &MemoryMapSize, MemoryMap, &MapKey, &DescriptorSize, &DescriptorVersion
        );
        /* Ignore return — best effort. */
        uefi_call_wrapper(
            SystemTable->BootServices->ExitBootServices, 2,
            ImageHandle, MapKey
        );
    }

    /* -------------------------------------------------------------------
     * Step 6: Jump to the kernel entry point at 0x100000.
     *
     * The kernel binary's first byte is _start (entry.asm) which uses
     * System V AMD64 calling convention:
     *   RDI = fb_addr  (framebuffer physical address, 32-bit fits QEMU VGA)
     *   RSI = pitch    (bytes per scanline)
     *   RDX = width    (pixels)
     *   RCX = height   (pixels)
     *   R8  = bpp      (BYTES per pixel — 4 for 32-bit GOP)
     *   R9  = UEFI_E820_MAP_ADDR (0x7000 — E820-format memory map)
     *   [RSP] = e820_count (memory map entry count, 7th argument)
     *
     * We pin values into specific argument registers using GCC named
     * register variables, set up a fresh stack at 31 MiB (0x1FF0000),
     * then jump directly to 0x100000.
     *
     * NOTE: FbBase is truncated to 32 bits; for QEMU's virtual VGA the
     * GOP framebuffer is always in the low 4 GiB (typically 0xFD000000).
     * ------------------------------------------------------------------- */
    register UINT64 r_rdi __asm__("rdi") = (UINT64)(UINT32)FbBase;
    register UINT64 r_rsi __asm__("rsi") = (UINT64)FbPitch;
    register UINT64 r_rdx __asm__("rdx") = (UINT64)FbWidth;
    register UINT64 r_rcx __asm__("rcx") = (UINT64)FbHeight;
    register UINT64 r_r8  __asm__("r8")  = (UINT64)4;                  /* bytes/pixel */
    register UINT64 r_r9  __asm__("r9")  = (UINT64)UEFI_E820_MAP_ADDR; /* E820 map    */
    register UINT64 r_cnt __asm__("rbx") = (UINT64)e820_count;         /* entry count */

    __asm__ volatile (
        /* Set up a fresh kernel stack at 31 MiB */
        "movq $0x1FF0000, %%rsp\n\t"
        "andq $-16,       %%rsp\n\t"
        /* Push 7th argument: memory-map entry count */
        "pushq %[cnt]\n\t"
        "subq $8, %%rsp\n\t"    /* re-align stack to 16 bytes before call */
        /* Jump to kernel _start */
        "movq $0x100000, %%rax\n\t"
        "jmpq *%%rax\n\t"
        /* Input operands ensure the register variables are live here */
        :: "r"(r_rdi), "r"(r_rsi), "r"(r_rdx), "r"(r_rcx),
           "r"(r_r8),  "r"(r_r9),  [cnt] "r"(r_cnt)
        :  "rax", "memory"
    );
    __builtin_unreachable();
}
