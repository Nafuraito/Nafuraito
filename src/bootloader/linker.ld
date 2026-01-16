ENTRY(entry)
OUTPUT_FORMAT(binary)

SECTIONS
{
    . = 0x7E00;

    .text : {
        /* 16-bit entry point must be at the very beginning */
        *(.text.entry)
        *(.text)
    }

    .rodata : {
        *(.rodata)
        *(.rodata.*)
    }

    .data : {
        *(.data)
        *(.data.*)
    }

    .bss : {
        *(.bss)
        *(.bss.*)
        *(COMMON)
    }

    /DISCARD/ : {
        *(.note*)
        *(.comment)
        *(.eh_frame)
        *(.eh_frame_hdr)
    }
}
