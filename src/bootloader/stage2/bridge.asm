; Nafuraito Assembly Bridge
;
; Bridges between the assembly bootloader (main.asm) and the C kernel loader.
; Handles the transition to C code and then to the kernel.

bits 64

extern load_kernel_c
extern vbe_framebuffer_addr
extern vbe_pitch
extern vbe_width
extern vbe_height
extern vbe_bpp
extern memory_map_addr
extern memory_map_entries

global load_kernel

section .text

;=============================================================================
; Kernel Loader Bridge
;=============================================================================

load_kernel:
    ; Set up stack for C code
    mov rsp, 0x900000
    
    ; Prepare VBE info structure on stack (16 bytes aligned)
    sub rsp, 16
    mov rdi, rsp                        ; First argument: pointer to vbe_info_t
    
    ; Fill VBE structure
    mov eax, [vbe_framebuffer_addr]
    mov [rdi], eax                      ; framebuffer_addr (4 bytes)
    mov ax, [vbe_pitch]
    mov [rdi + 4], ax                   ; pitch (2 bytes)
    mov ax, [vbe_width]
    mov [rdi + 6], ax                   ; width (2 bytes)
    mov ax, [vbe_height]
    mov [rdi + 8], ax                   ; height (2 bytes)
    mov al, [vbe_bpp]
    mov [rdi + 10], al                  ; bpp (1 byte)
    
    ; Prepare memory info structure on stack (8 bytes aligned)
    sub rsp, 16
    mov rsi, rsp                        ; Second argument: pointer to memory_info_t
    
    ; Fill memory info structure
    mov eax, [memory_map_addr]
    mov [rsi], eax                      ; memory_map_addr (4 bytes)
    mov ax, [memory_map_entries]
    mov [rsi + 4], ax                   ; memory_map_entries (2 bytes)
    
    ; Call C kernel loader
    ; Arguments are already in rdi (vbe_info_t*) and rsi (memory_info_t*)
    call load_kernel_c
    
    ; Check if kernel loading succeeded
    test rax, rax
    jz .halt
    
    ; Kernel entry point is in rax
    ; Now set up registers for kernel according to System V AMD64 calling convention
    ; Restore stack pointer
    mov rsp, 0x900000
    
    ; Pass boot info to kernel via registers:
    ; rdi = framebuffer address
    ; rsi = pitch
    ; rdx = width
    ; rcx = height
    ; r8  = bpp
    ; r9  = memory map address
    mov edi, [vbe_framebuffer_addr]
    movzx rsi, word [vbe_pitch]
    movzx rdx, word [vbe_width]
    movzx rcx, word [vbe_height]
    movzx r8, byte [vbe_bpp]
    mov r9d, [memory_map_addr]
    
    ; Jump to kernel (entry point is in rax)
    jmp rax

.halt:
    cli
    hlt
    jmp .halt
