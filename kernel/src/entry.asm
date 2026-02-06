; =============================================================================
; Nafuraito Kernel Entry Point (Assembly)
; =============================================================================
;
; This is THE FIRST kernel code that runs! The bootloader has loaded us into
; memory and jumped here. We're in 64-bit long mode with paging enabled.
;
; Our job is simple but critical:
; 1. Disable interrupts (we're not ready for them yet!)
; 2. Mask all hardware interrupts at the PIC
; 3. Zero out the BSS section (uninitialized global variables)
; 4. Call the Rust kernel's enter() function
;
; The bootloader passes us parameters in registers:
;   RDI = framebuffer address
;   RSI = pitch (bytes per scanline)
;   RDX = width
;   RCX = height  
;   R8  = bits per pixel
;   R9  = memory map address
;
; We need to preserve these while we clean up the BSS!
;
; =============================================================================

extern enter
extern __bss_start
extern __bss_end

global _start

section .text
_start:
    cli             ; Disable interrupts IMMEDIATELY
                    ; We don't have interrupt handlers set up yet, so any
                    ; interrupt would triple-fault and reboot the machine!
    
    ; Mask all PIC interrupts at the hardware level  
    ; The PIC (Programmable Interrupt Controller) fires hardware interrupts.
    ; Even with CLI, certain NMIs can still fire. Better safe than sorry!
    mov al, 0xFF
    out 0x21, al    ; Port 0x21 = master PIC interrupt mask (IRQs 0-7)
    out 0xA1, al    ; Port 0xA1 = slave PIC interrupt mask (IRQs 8-15)
                    ; 0xFF = all bits set = all IRQs masked (disabled)
    
    ; Save boot parameters
    ; The bootloader passed important info in these registers.
    ; We need to preserve them while we zero the BSS section.
    push rdi
    push rsi
    push rdx
    push rcx
    push r8
    push r9
    
    ; Zero out the BSS section
    ; BSS = Block Started by Symbol (weird name from ancient assemblers)
    ; It's where uninitialized global/static variables live.
    ; In the ELF file, BSS takes zero space (since it's all zeros).
    ; But at runtime, we need to actually zero it out!
    ;
    ; Example: `static mut COUNTER: u64 = 0;` lives in BSS
    lea rdi, [rel __bss_start]
    lea rcx, [rel __bss_end]
    sub rcx, rdi            ; RCX = size in bytes
    shr rcx, 3              ; RCX = size in qwords (8-byte chunks)
    xor rax, rax            ; RAX = 0 (the value we'll write)
    rep stosq               ; Repeat: Store qword at [RDI], increment RDI
                            ; This zeros 8 bytes at a time - fast!
    
    ; Handle leftover bytes
    ; If BSS size isn't a multiple of 8, we have a few bytes left over.
    ; Zero them one byte at a time.
    lea rdi, [rel __bss_start]
    lea rcx, [rel __bss_end]
    sub rcx, rdi
    and rcx, 7              ; remaining bytes
    rep stosb
    
    ; Restore boot parameters
    ; Pop them back in reverse order (stack is LIFO)
    pop r9
    pop r8
    pop rcx
    pop rdx
    pop rsi
    pop rdi
    
    ; Jump into Rust!
    ; Call the enter() function (defined in main.rs)
    ; It will never return, so we'll never get past this line
    call enter
    
    ; Uh oh - if we get here, something went horribly wrong!
    ; The kernel returned when it should run forever.
    ; Best we can do is disable interrupts and halt.
    cli
    hlt
