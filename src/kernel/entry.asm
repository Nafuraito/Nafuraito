; Nafuraito Kernel Entry Point (Assembly)
;
; This is the initial entry point that the bootloader jumps to.
; It immediately transfers control to the Rust kernel entry function.

extern enter

global _start

section .text
_start:
    jmp enter       ; Transfer to Rust kernel
    ; Fallback halt (should never reach here)
    cli
    hlt
