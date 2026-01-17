/* Kernel Linker Script */
/* The kernel is loaded at 1MB (0x100000) by the bootloader */

ENTRY(_start)
OUTPUT_FORMAT(binary)

SECTIONS
{
    /* Kernel starts at 1MB */
    . = 0x100000;
    
    /* Code section - entry point first */
    .text : {
        /* Force entry.o to be first so _start is at 0x100000 */
        *entry.o(.text)
        *(.text)
        *(.text.*)
    }
    
    /* Read-only data */
    .rodata : {
        *(.rodata)
        *(.rodata.*)
    }
    
    /* Initialized data */
    .data : {
        *(.data)
        *(.data.*)
    }
    
    /* Uninitialized data */
    .bss : {
        *(.bss)
        *(.bss.*)
        *(COMMON)
    }
    
    /* Discard debugging info */
    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
        *(.note.*)
    }
}
