/**
 * boot_info.h — Unified Boot Information Structure
 *
 * This header defines the BootInfo structure that both the BIOS (stage2/bridge.asm)
 * and UEFI (uefi/src/main.c) bootloaders fill before jumping to the kernel.
 *
 * The kernel reads from BOOT_INFO_ADDR (physical 0x6000) on startup to detect
 * the boot path and locate system resources that differ between BIOS and UEFI.
 *
 * Memory layout around the boot info region:
 *   0x0500 – 0x5FFF : BIOS E820 memory map  (stage2 stores E820 here)
 *   0x6000 – 0x601F : BootInfo struct        (this header)
 *   0x7000 – 0x8BFF : UEFI-converted E820    (up to 256 entries × 24 bytes)
 *   0x10000 – 0x15FFF: Page tables           (bridge.asm 4-level paging setup)
 */

#ifndef BOOT_INFO_H
#define BOOT_INFO_H

/* Physical address of the BootInfo struct. */
#define BOOT_INFO_ADDR         0x6000

/* Magic number stored in BootInfo.magic to confirm the struct is valid.
 * ASCII "NAFI" (Nafuraito). */
#define BOOT_INFO_MAGIC        0x4E414649U

/* Values for BootInfo.boot_type */
#define BOOT_TYPE_BIOS         0x42494F53U  /* ASCII "BIOS" */
#define BOOT_TYPE_UEFI         0x55454649U  /* ASCII "UEFI" */

/**
 * Physical address where the UEFI bootloader writes the E820-format memory
 * map converted from EFI_MEMORY_DESCRIPTOR entries.
 *
 * NOTE: The BIOS bootloader writes E820 entries to 0x0500.
 *       Both addresses are in the first megabyte, which the PMM marks as
 *       reserved — so no risk of the data being overwritten by the allocator.
 */
#define UEFI_E820_MAP_ADDR     0x7000

#ifndef __ASSEMBLER__

#include <stdint.h>

/**
 * boot_info_t — written by the bootloader to BOOT_INFO_ADDR.
 *
 * Total size: 32 bytes.  All fields are little-endian.
 *
 * Layout:
 *   +0x00  uint32  magic               — must equal BOOT_INFO_MAGIC
 *   +0x04  uint32  boot_type           — BOOT_TYPE_BIOS or BOOT_TYPE_UEFI
 *   +0x08  uint64  rsdp_addr           — physical address of ACPI RSDP (0 = not found)
 *   +0x10  uint32  mem_map_entry_count — number of E820 entries at R9
 *   +0x14  uint32  _reserved0
 *   +0x18  uint64  _reserved1
 */
typedef struct __attribute__((packed)) {
    uint32_t magic;
    uint32_t boot_type;
    uint64_t rsdp_addr;
    uint32_t mem_map_entry_count;
    uint32_t _reserved0;
    uint64_t _reserved1;
} boot_info_t;

#endif /* __ASSEMBLER__ */

#endif /* BOOT_INFO_H */
