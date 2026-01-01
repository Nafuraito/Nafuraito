bits 16

; Entry section - must be first in binary
section .text.entry

global entry
extern load_kernel

entry:
    ; Ensure segments are set correctly
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    ; Set video mode 3 (80x25 text mode, 16 colors) - this clears the screen
    ; and resets video hardware to a known state
    mov ax, 0x0003
    int 0x10
    
    ; Set display page to 0
    mov ax, 0x0500
    int 0x10
    
    ; Disable text cursor (hide it)
    mov ah, 0x01
    mov cx, 0x2607      ; Set cursor shape to invisible
    int 0x10

    ; Check if long mode is supported
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no_long_mode
    
    mov eax, 0x80000001
    cpuid
    test edx, (1 << 29)
    jz .no_long_mode
    
    ; Set up for long mode
    cli
    
    ; Enable A20 line
    in al, 0x92
    or al, 2
    out 0x92, al
    
    ; Set up page tables for identity mapping
    mov edi, 0x1000
    mov cr3, edi
    xor eax, eax
    mov ecx, 4096
    rep stosd
    mov edi, cr3
    
    ; PML4T[0] -> PDPT
    mov dword [edi], 0x2003
    add edi, 0x1000
    
    ; PDPT[0] -> PDT
    mov dword [edi], 0x3003
    add edi, 0x1000
    
    ; PDT entries (2MB pages) - map first 16MB (8 entries)
    ; This covers: code at 0x7E00, kernel at 0x100000, stack at 0x900000
    mov dword [edi],      0x00000083  ; 0MB - 2MB
    mov dword [edi + 8],  0x00200083  ; 2MB - 4MB
    mov dword [edi + 16], 0x00400083  ; 4MB - 6MB
    mov dword [edi + 24], 0x00600083  ; 6MB - 8MB
    mov dword [edi + 32], 0x00800083  ; 8MB - 10MB (covers stack at 0x900000)
    mov dword [edi + 40], 0x00A00083  ; 10MB - 12MB
    mov dword [edi + 48], 0x00C00083  ; 12MB - 14MB
    mov dword [edi + 56], 0x00E00083  ; 14MB - 16MB
    
    ; Enable PAE
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax
    
    ; Set LME bit
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr
    
    ; Load GDT and enable paging + protected mode
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, (1 << 31) | 1   ; Enable paging AND protected mode
    mov cr0, eax
    
    ; Jump to 64-bit code
    jmp 0x08:long_mode_start

.no_long_mode:
    ; Enter 32-bit protected mode
    cli
    
    ; Enable A20 line
    in al, 0x92
    or al, 2
    out 0x92, al
    
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    
    jmp 0x08:protected_mode_start

; Regular text section for the rest
section .text

bits 32
protected_mode_start:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    
    ; Jump to kernel loader in 32-bit mode
    jmp load_kernel

bits 64
long_mode_start:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    
    ;=========================================================================
    ; VGA REGISTER INITIALIZATION FOR 64-BIT MODE
    ;=========================================================================
    
    ; Set CRTC Start Address to 0 (critical for fixing flickering)
    mov dx, 0x3D4
    mov al, 0x0C            ; Start Address High
    out dx, al
    inc dx
    xor al, al
    out dx, al
    
    dec dx
    mov al, 0x0D            ; Start Address Low  
    out dx, al
    inc dx
    xor al, al
    out dx, al
    
    ; Disable cursor
    dec dx
    mov al, 0x0A
    out dx, al
    inc dx
    mov al, 0x20
    out dx, al
    
    ; Clear VGA buffer
    mov rdi, 0xB8000
    mov rcx, 2000
    mov rax, 0x0F200F200F200F20
    rep stosq
    
    ; Jump to 64-bit Kernel loader
    jmp load_kernel

; GDT must be in same section to be accessible in real mode
align 8
gdt:
    dq 0x0000000000000000  ; Null descriptor
    dq 0x00AF9A000000FFFF  ; 64-bit code segment
    dq 0x00CF92000000FFFF  ; Data segment

gdt_descriptor:
    dw gdt_descriptor - gdt - 1
    dd gdt
