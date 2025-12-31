ENTRY(entry)
OUTPUT_FORMAT(binary)

SECTIONS
{
    . = 0x7E00;

    .text : {
        /* Ensure entry point is at the very beginning */
        *(.text.entry)
        *(.text)
    }

    .data : {
        *(.data)
    }

    .rodata : {
        *(.rodata)
    }

    .bss : {
        *(.bss)
    }

    /DISCARD/ : {
        *(.note*)
        *(.comment)
        *(.eh_frame)
    }
}
