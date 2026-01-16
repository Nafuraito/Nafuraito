; Nafuraito Assembly Bridge (Stage 2 Bootloader)
;
; This is the Stage 2 bootloader that transitions from 16-bit real mode
; to 64-bit long mode, then loads and jumps to the kernel.
;
; Memory Layout:
;   0x7E00 - Stage 2 entry (this code)
;   0x8000 - E820 memory map
;   0x9000 - Page tables (PML4, PDPT, PD, PT)
;   0x100000 - Kernel load address (1MB)

; Use default section for entry
[SECTION .text.entry]
[BITS 16]

global entry
entry:
    ; Set up segments
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov sp, 0x7C00              ; Stack below Stage 1

    ; Get memory map via E820
    call get_memory_map

    ; Get VESA VBE mode info
    call setup_vesa

    ; Enable A20 line
    call enable_a20

    ; Load GDT for protected mode
    lgdt [gdt_descriptor]

    ; Disable interrupts before mode switch
    cli

    ; Enable protected mode (set PE bit in CR0)
    mov eax, cr0
    or eax, 1
    mov cr0, eax

    ; Far jump to 32-bit code segment
    jmp dword 0x08:protected_mode_entry

; ============================================================================
; Get Memory Map (E820)
; ============================================================================
[BITS 16]
get_memory_map:
    ; Store E820 map at 0x500 (safe area in low memory, after BDA)
    mov di, 0x504              ; Start after count field (count at 0x500)
    xor ebx, ebx                ; Continuation value (start with 0)
    xor si, si                  ; Entry counter
    mov edx, 0x534D4150         ; "SMAP" signature

.e820_loop:
    mov eax, 0xE820
    mov ecx, 24                 ; Entry size
    int 0x15
    jc .e820_done               ; Carry set = error or done
    
    cmp eax, 0x534D4150         ; Check SMAP signature
    jne .e820_done

    inc si                      ; Increment entry count
    add di, 24                  ; Move to next entry

    test ebx, ebx               ; ebx = 0 means done
    jz .e820_done
    jmp .e820_loop

.e820_done:
    mov [0x500], si             ; Store entry count at 0x500
    mov word [memory_map_addr], 0x500
    mov [memory_map_entries], si
    ret

; ============================================================================
; Setup VESA VBE Graphics Mode
; ============================================================================
[BITS 16]
setup_vesa:
    ; Get VBE Controller Info
    mov ax, 0x4F00
    mov di, vbe_info_block
    int 0x10
    cmp ax, 0x004F
    jne .vesa_fail

    ; Try to set 1024x768x32 mode (commonly mode 0x118)
    ; First, try getting mode info for 0x118
    mov ax, 0x4F01
    mov cx, 0x118               ; Mode 0x118 = 1024x768x32
    mov di, mode_info_block
    int 0x10
    cmp ax, 0x004F
    jne .try_fallback

    ; Check if mode supports LFB
    mov ax, [mode_info_block]   ; Mode attributes
    test ax, 0x80               ; Bit 7 = LFB available
    jz .try_fallback

    ; Set VBE mode 0x118 with LFB
    mov ax, 0x4F02
    mov bx, 0x4118              ; 0x4000 | mode = enable LFB
    int 0x10
    cmp ax, 0x004F
    jne .try_fallback

    ; Store framebuffer info
    mov eax, [mode_info_block + 40]  ; PhysBasePtr
    mov [vbe_framebuffer_addr], eax
    mov ax, [mode_info_block + 16]   ; BytesPerScanLine
    mov [vbe_pitch], ax
    mov ax, [mode_info_block + 18]   ; XResolution
    mov [vbe_width], ax
    mov ax, [mode_info_block + 20]   ; YResolution
    mov [vbe_height], ax
    mov al, [mode_info_block + 25]   ; BitsPerPixel
    mov [vbe_bpp], al
    ret

.try_fallback:
    ; Fallback to mode 0x112 (640x480x32)
    mov ax, 0x4F01
    mov cx, 0x112
    mov di, mode_info_block
    int 0x10
    cmp ax, 0x004F
    jne .vesa_fail

    mov ax, 0x4F02
    mov bx, 0x4112              ; 0x4000 | mode = enable LFB
    int 0x10
    cmp ax, 0x004F
    jne .vesa_fail

    ; Store framebuffer info
    mov eax, [mode_info_block + 40]
    mov [vbe_framebuffer_addr], eax
    mov ax, [mode_info_block + 16]
    mov [vbe_pitch], ax
    mov ax, [mode_info_block + 18]
    mov [vbe_width], ax
    mov ax, [mode_info_block + 20]
    mov [vbe_height], ax
    mov al, [mode_info_block + 25]
    mov [vbe_bpp], al
    ret

.vesa_fail:
    ; Set defaults if VESA fails (no framebuffer)
    mov dword [vbe_framebuffer_addr], 0
    mov word [vbe_pitch], 0
    mov word [vbe_width], 0
    mov word [vbe_height], 0
    mov byte [vbe_bpp], 0
    ret

; ============================================================================
; Enable A20 Line
; ============================================================================
[BITS 16]
enable_a20:
    ; Try BIOS method first
    mov ax, 0x2401
    int 0x15
    jnc .a20_done

    ; Try keyboard controller method
    call .wait_kbd
    mov al, 0xAD
    out 0x64, al
    call .wait_kbd
    mov al, 0xD0
    out 0x64, al
    call .wait_kbd2
    in al, 0x60
    push ax
    call .wait_kbd
    mov al, 0xD1
    out 0x64, al
    call .wait_kbd
    pop ax
    or al, 2
    out 0x60, al
    call .wait_kbd
    mov al, 0xAE
    out 0x64, al
    call .wait_kbd

.a20_done:
    ret

.wait_kbd:
    in al, 0x64
    test al, 2
    jnz .wait_kbd
    ret

.wait_kbd2:
    in al, 0x64
    test al, 1
    jz .wait_kbd2
    ret

; ============================================================================
; 32-bit Protected Mode Entry
; ============================================================================
[SECTION .text]
[BITS 32]

protected_mode_entry:
    ; Set up segment registers for 32-bit mode
    mov ax, 0x10                ; Data segment selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x90000

    ; Set up paging for long mode
    call setup_paging

    ; Enable PAE (Physical Address Extension)
    mov eax, cr4
    or eax, (1 << 5)            ; Set PAE bit
    mov cr4, eax

    ; Load PML4 address into CR3
    mov eax, 0x9000             ; PML4 table at 0x9000
    mov cr3, eax

    ; Enable long mode via EFER MSR
    mov ecx, 0xC0000080         ; EFER MSR
    rdmsr
    or eax, (1 << 8)            ; Set LME (Long Mode Enable)
    wrmsr

    ; Enable paging (this activates long mode)
    mov eax, cr0
    or eax, (1 << 31)           ; Set PG bit
    mov cr0, eax

    ; Load 64-bit GDT
    lgdt [gdt64_descriptor]

    ; Far jump to 64-bit code
    jmp 0x08:long_mode_entry

; ============================================================================
; Setup Paging (Identity map first 4GB)
; ============================================================================
[BITS 32]
setup_paging:
    ; Clear page table area (0x9000 - 0xF000)
    mov edi, 0x9000
    xor eax, eax
    mov ecx, 0x6000 / 4         ; 24KB / 4 bytes = 6144 dwords
    rep stosd

    ; PML4[0] -> PDPT at 0xA000
    mov dword [0x9000], 0xA003  ; Present + Writable + PDPT address

    ; PDPT[0] -> PD at 0xB000 (first 1GB)
    mov dword [0xA000], 0xB003  ; Present + Writable + PD address

    ; PDPT[1] -> PD at 0xC000 (second 1GB)
    mov dword [0xA008], 0xC003

    ; PDPT[2] -> PD at 0xD000 (third 1GB)
    mov dword [0xA010], 0xD003

    ; PDPT[3] -> PD at 0xE000 (fourth 1GB)
    mov dword [0xA018], 0xE003

    ; Fill Page Directories with 2MB pages (identity mapping)
    ; PD at 0xB000: maps 0x00000000 - 0x3FFFFFFF
    mov edi, 0xB000
    mov eax, 0x83               ; Present + Writable + 2MB page
    mov ecx, 512
.fill_pd1:
    mov [edi], eax
    add eax, 0x200000           ; 2MB per entry
    add edi, 8
    loop .fill_pd1

    ; PD at 0xC000: maps 0x40000000 - 0x7FFFFFFF
    mov edi, 0xC000
    mov eax, 0x40000083
    mov ecx, 512
.fill_pd2:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .fill_pd2

    ; PD at 0xD000: maps 0x80000000 - 0xBFFFFFFF
    mov edi, 0xD000
    mov eax, 0x80000083
    mov ecx, 512
.fill_pd3:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .fill_pd3

    ; PD at 0xE000: maps 0xC0000000 - 0xFFFFFFFF
    mov edi, 0xE000
    mov eax, 0xC0000083
    mov ecx, 512
.fill_pd4:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .fill_pd4

    ret

; ============================================================================
; 64-bit Long Mode Entry
; ============================================================================
[BITS 64]

extern load_kernel_c

long_mode_entry:
    ; Set up 64-bit segment registers
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Set up 64-bit stack
    mov rsp, 0x90000

    ; Call kernel loader
    call load_kernel

    ; Should not return, but halt if it does
    cli
.halt:
    hlt
    jmp .halt

; ============================================================================
; Load Kernel (64-bit)
; ============================================================================
[BITS 64]
load_kernel:
    ; Set up stack for C code
    mov rsp, 0x900000
    
    ; Prepare VBE info structure on stack (16 bytes aligned)
    sub rsp, 16
    mov rdi, rsp                        ; First argument: pointer to vbe_info_t
    
    ; Fill VBE structure
    mov eax, [rel vbe_framebuffer_addr]
    mov [rdi], eax                      ; framebuffer_addr (4 bytes)
    mov ax, [rel vbe_pitch]
    mov [rdi + 4], ax                   ; pitch (2 bytes)
    mov ax, [rel vbe_width]
    mov [rdi + 6], ax                   ; width (2 bytes)
    mov ax, [rel vbe_height]
    mov [rdi + 8], ax                   ; height (2 bytes)
    mov al, [rel vbe_bpp]
    mov [rdi + 10], al                  ; bpp (1 byte)
    
    ; Prepare memory info structure on stack (8 bytes aligned)
    sub rsp, 16
    mov rsi, rsp                        ; Second argument: pointer to memory_info_t
    
    ; Fill memory info structure
    mov eax, [rel memory_map_addr]
    mov [rsi], eax                      ; memory_map_addr (4 bytes)
    mov ax, [rel memory_map_entries]
    mov [rsi + 4], ax                   ; memory_map_entries (2 bytes)
    
    ; Call C kernel loader
    call load_kernel_c
    
    ; Check if kernel loading succeeded
    test rax, rax
    jz .load_failed
    
    ; Save kernel entry point
    mov rbx, rax
    
    ; Restore stack
    mov rsp, 0x900000
    
    ; Pass boot info to kernel via registers (System V AMD64 ABI):
    ; rdi = framebuffer address (32-bit, zero-extended)
    ; rsi = pitch
    ; rdx = width
    ; rcx = height
    ; r8  = bpp
    ; r9  = memory map address
    mov edi, [rel vbe_framebuffer_addr]
    movzx rsi, word [rel vbe_pitch]
    movzx rdx, word [rel vbe_width]
    movzx rcx, word [rel vbe_height]
    movzx r8, byte [rel vbe_bpp]
    mov r9d, [rel memory_map_addr]
    
    ; Jump to kernel entry point
    jmp rbx

.load_failed:
    ret

; ============================================================================
; Data Section
; ============================================================================
[SECTION .data]

; VBE/Memory info (globals for C code)
global vbe_framebuffer_addr
global vbe_pitch
global vbe_width
global vbe_height
global vbe_bpp
global memory_map_addr
global memory_map_entries

vbe_framebuffer_addr:   dd 0
vbe_pitch:              dw 0
vbe_width:              dw 0
vbe_height:             dw 0
vbe_bpp:                db 0
                        db 0    ; Padding
memory_map_addr:        dd 0
memory_map_entries:     dw 0

; ============================================================================
; GDT for Protected Mode (32-bit)
; ============================================================================
align 16
gdt_start:
    ; Null descriptor
    dq 0

    ; Code segment (0x08) - 32-bit
    dw 0xFFFF       ; Limit low
    dw 0            ; Base low
    db 0            ; Base middle
    db 0x9A         ; Access: present, ring 0, code, readable
    db 0xCF         ; Granularity: 4K, 32-bit
    db 0            ; Base high

    ; Data segment (0x10) - 32-bit
    dw 0xFFFF       ; Limit low
    dw 0            ; Base low
    db 0            ; Base middle
    db 0x92         ; Access: present, ring 0, data, writable
    db 0xCF         ; Granularity: 4K, 32-bit
    db 0            ; Base high
gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd gdt_start

; ============================================================================
; GDT for Long Mode (64-bit)
; ============================================================================
align 16
gdt64_start:
    ; Null descriptor
    dq 0

    ; Code segment (0x08) - 64-bit
    dw 0            ; Limit (ignored in 64-bit)
    dw 0            ; Base low
    db 0            ; Base middle
    db 0x9A         ; Access: present, ring 0, code
    db 0x20         ; Long mode flag
    db 0            ; Base high

    ; Data segment (0x10) - 64-bit
    dw 0            ; Limit
    dw 0            ; Base low
    db 0            ; Base middle
    db 0x92         ; Access: present, ring 0, data
    db 0x00         ; Flags
    db 0            ; Base high
gdt64_end:

gdt64_descriptor:
    dw gdt64_end - gdt64_start - 1
    dq gdt64_start

; ============================================================================
; VBE Info Buffers (in BSS or zeroed data)
; ============================================================================
[SECTION .bss]

vbe_info_block:     resb 512
mode_info_block:    resb 256
