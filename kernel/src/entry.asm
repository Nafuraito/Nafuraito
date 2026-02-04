; Nafuraito Kernel Entry Point (Assembly)
;
; This is the initial entry point that the bootloader jumps to.
; It zeroes the BSS section and transfers control to Rust kernel.

extern enter
extern __bss_start
extern __bss_end

global _start

section .text
_start:
    cli             ; Disable interrupts immediately
    
    ; Mask all PIC interrupts before anything else
    mov al, 0xFF
    out 0x21, al    ; Mask all IRQs on master PIC
    out 0xA1, al    ; Mask all IRQs on slave PIC
    
    ; Save registers before BSS clear (bootloader passed args)
    push rdi
    push rsi
    push rdx
    push rcx
    push r8
    push r9
    
    ; Zero BSS section
    lea rdi, [rel __bss_start]
    lea rcx, [rel __bss_end]
    sub rcx, rdi            ; rcx = size in bytes
    shr rcx, 3              ; rcx = size in qwords
    xor rax, rax            ; zero to store
    rep stosq               ; zero out BSS
    
    ; Handle remaining bytes (if BSS size not multiple of 8)
    lea rdi, [rel __bss_start]
    lea rcx, [rel __bss_end]
    sub rcx, rdi
    and rcx, 7              ; remaining bytes
    rep stosb
    
    ; Restore registers
    pop r9
    pop r8
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    
    ; Call enter - it's an extern function
    call enter
    
    ; If kernel returns (shouldn't happen), halt
    cli
    hlt
