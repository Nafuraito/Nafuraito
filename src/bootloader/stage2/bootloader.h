#ifndef BOOTLOADER_H
#define BOOTLOADER_H

// Bootloader types - use specific-width types
typedef unsigned char      uint8_t;
typedef unsigned short     uint16_t;
typedef unsigned int       uint32_t;
typedef unsigned long long uint64_t;

typedef signed char        int8_t;
typedef signed short       int16_t;
typedef signed int         int32_t;
typedef signed long long   int64_t;

// NULL pointer
#define NULL ((void*)0)

//=============================================================================
// Boot Information Structure
//=============================================================================

typedef struct {
    uint32_t framebuffer_addr;
    uint16_t pitch;
    uint16_t width;
    uint16_t height;
    uint8_t  bpp;
} vbe_info_t;

typedef struct {
    uint32_t memory_map_addr;
    uint16_t memory_map_entries;
} memory_info_t;

//=============================================================================
// Port I/O Functions (Inline Assembly)
//=============================================================================

static inline uint8_t inb(uint16_t port) {
    uint8_t value;
    __asm__ volatile ("inb %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

static inline void outb(uint16_t port, uint8_t value) {
    __asm__ volatile ("outb %0, %1" : : "a"(value), "Nd"(port));
}

static inline uint16_t inw(uint16_t port) {
    uint16_t value;
    __asm__ volatile ("inw %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

static inline void outw(uint16_t port, uint16_t value) {
    __asm__ volatile ("outw %0, %1" : : "a"(value), "Nd"(port));
}

// Read multiple 16-bit words from a port into memory
static inline void insw(uint16_t port, void* addr, uint32_t count) {
    __asm__ volatile ("rep insw" 
                     : "+D"(addr), "+c"(count) 
                     : "d"(port) 
                     : "memory");
}

//=============================================================================
// ATA PIO Constants
//=============================================================================

#define KERNEL_LOAD_ADDR    0x100000
#define KERNEL_START_SECTOR 64
#define KERNEL_SECTORS      128

// ATA Primary Bus Ports
#define ATA_PRIMARY_DATA     0x1F0
#define ATA_PRIMARY_ERROR    0x1F1
#define ATA_PRIMARY_SECCOUNT 0x1F2
#define ATA_PRIMARY_LBA_LO   0x1F3
#define ATA_PRIMARY_LBA_MID  0x1F4
#define ATA_PRIMARY_LBA_HI   0x1F5
#define ATA_PRIMARY_DRIVE    0x1F6
#define ATA_PRIMARY_STATUS   0x1F7
#define ATA_PRIMARY_CMD      0x1F7

// ATA Status Register Bits
#define ATA_STATUS_ERR  0x01
#define ATA_STATUS_DRQ  0x08
#define ATA_STATUS_RDY  0x48
#define ATA_STATUS_BSY  0x80

// ATA Commands
#define ATA_CMD_READ_SECTORS 0x20

// Serial port (COM1) for debugging
#define SERIAL_PORT 0x3F8

//=============================================================================
// Serial Debug Functions
//=============================================================================

static inline void serial_char(char c) {
    outb(SERIAL_PORT, c);
}

static inline void serial_str(const char* s) {
    while (*s) {
        serial_char(*s++);
    }
}

static inline void serial_hex8(uint8_t val) {
    const char hex[] = "0123456789ABCDEF";
    serial_char(hex[(val >> 4) & 0x0F]);
    serial_char(hex[val & 0x0F]);
}

static inline void serial_hex32(uint32_t val) {
    serial_hex8((val >> 24) & 0xFF);
    serial_hex8((val >> 16) & 0xFF);
    serial_hex8((val >> 8) & 0xFF);
    serial_hex8(val & 0xFF);
}

//=============================================================================
// Function Declarations
//=============================================================================

// Main kernel loader function (called from assembly)
uint64_t load_kernel_c(vbe_info_t* vbe, memory_info_t* mem);

// ATA driver functions
int ata_wait_ready(void);
int ata_wait_drq(void);
int ata_read_sectors(void* dest, uint64_t lba, uint32_t count);

#endif // BOOTLOADER_H
