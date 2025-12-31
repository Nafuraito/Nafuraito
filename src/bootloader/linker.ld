ENTRY(entry)

SECTIONS
{
    . = 0x0;

    .text : {
        *(.text)
        *(_ENTRY)
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
