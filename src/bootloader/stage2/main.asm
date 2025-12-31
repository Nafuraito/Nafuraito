bits 16

section .text

global entry
extern load_kernel

%macro print_TTY 1
    mov al, %1
    mov bh, 0
    mov ah, 0Eh
    int 10h
%endmacro

entry:
    print_TTY 'S'
    print_TTY '2'
    print_TTY ' '

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
    
    ; PDT entries (2MB pages)
    mov dword [edi], 0x00000083
    mov dword [edi + 8], 0x00200083
    
    ; Enable PAE
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax
    
    ; Set LME bit
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr
    
    ; Load GDT and enable paging
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, (1 << 31)
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

bits 32
protected_mode_start:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    
    ; Debug: 32-bit mode
    mov al, 'P'
    mov dx, 0x3F8
    out dx, al
    mov al, 'M'
    out dx, al
    mov al, 10
    out dx, al
    
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
    ; Jump to kernel loader in 64-bit mode
    jmp load_kernel

gdt:
    dq 0x0000000000000000  ; Null descriptor
    dq 0x00AF9A000000FFFF  ; 64-bit code segment
    dq 0x00CF92000000FFFF  ; Data segment

gdt_descriptor:
    dw gdt_descriptor - gdt - 1
    dd gdt
