; Nafuraito Stage 2 Bootloader
;
; Handles CPU mode transitions (Real -> Protected -> Long Mode)
; Sets up paging, GDT, VESA graphics, and transfers control to the kernel loader.

bits 16

section .text.entry

global entry
global vbe_framebuffer_addr
global vbe_pitch
global vbe_width
global vbe_height
global vbe_bpp
extern load_kernel

;=============================================================================
; VESA/VBE Constants
;=============================================================================

VBE_MODE_WIDTH      equ 1920
VBE_MODE_HEIGHT     equ 1080
VBE_MODE_BPP        equ 24
VBE_INFO_BLOCK      equ 0x5000      ; VBE Controller Info
VBE_MODE_INFO       equ 0x5200      ; VBE Mode Info Block

;=============================================================================
; Entry Point (16-bit Real Mode)
;=============================================================================

entry:
    ; Set up segments
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    ; Set video mode 3 temporarily (80x25 text mode)
    mov ax, 0x0003
    int 0x10

    ;=========================================================================
    ; VESA Graphics Mode Setup
    ;=========================================================================
    
    ; Get VBE Controller Info
    mov ax, 0x4F00
    mov di, VBE_INFO_BLOCK
    mov dword [di], 'VBE2'      ; Request VBE 2.0+ info
    int 0x10
    
    cmp ax, 0x004F
    jne .vesa_failed
    
    ; Search for 1920x1080x24 mode in mode list
    mov si, [VBE_INFO_BLOCK + 14]   ; Video mode pointer (offset)
    mov ax, [VBE_INFO_BLOCK + 16]   ; Video mode pointer (segment)
    mov fs, ax
    
.search_modes:
    mov cx, [fs:si]             ; Get mode number
    cmp cx, 0xFFFF              ; End of list?
    je .try_fallback_mode
    
    ; Get mode info
    push si
    push cx
    mov ax, 0x4F01
    mov di, VBE_MODE_INFO
    int 0x10
    pop cx
    pop si
    
    cmp ax, 0x004F
    jne .next_mode
    
    ; Check if mode matches our requirements
    ; Check width
    mov ax, [VBE_MODE_INFO + 18]    ; XResolution
    cmp ax, VBE_MODE_WIDTH
    jne .next_mode
    
    ; Check height
    mov ax, [VBE_MODE_INFO + 20]    ; YResolution
    cmp ax, VBE_MODE_HEIGHT
    jne .next_mode
    
    ; Check bits per pixel
    mov al, [VBE_MODE_INFO + 25]    ; BitsPerPixel
    cmp al, VBE_MODE_BPP
    jne .next_mode
    
    ; Check if linear framebuffer is supported (bit 7 of mode attributes)
    mov ax, [VBE_MODE_INFO]         ; Mode attributes
    test ax, 0x80
    jz .next_mode
    
    ; Found matching mode - set it
    jmp .set_mode

.next_mode:
    add si, 2
    jmp .search_modes

.try_fallback_mode:
    ; Try common VESA mode for 1920x1080x24 (0x118 variant or 0x143)
    ; First try to get info for mode 0x143 (common 1920x1080x24)
    mov cx, 0x143
    mov ax, 0x4F01
    mov di, VBE_MODE_INFO
    int 0x10
    cmp ax, 0x004F
    jne .try_mode_118
    
    ; Verify it's the right resolution
    mov ax, [VBE_MODE_INFO + 18]
    cmp ax, VBE_MODE_WIDTH
    jne .try_mode_118
    mov ax, [VBE_MODE_INFO + 20]
    cmp ax, VBE_MODE_HEIGHT
    jne .try_mode_118
    jmp .set_mode

.try_mode_118:
    ; Try mode 0x118 (1024x768x24) as a fallback
    mov cx, 0x118
    mov ax, 0x4F01
    mov di, VBE_MODE_INFO
    int 0x10
    cmp ax, 0x004F
    jne .vesa_failed
    jmp .set_mode

.set_mode:
    ; Save framebuffer info before setting mode
    mov eax, [VBE_MODE_INFO + 40]   ; PhysBasePtr (framebuffer address)
    mov [vbe_framebuffer_addr], eax
    
    mov ax, [VBE_MODE_INFO + 16]    ; BytesPerScanLine (pitch)
    mov [vbe_pitch], ax
    
    mov ax, [VBE_MODE_INFO + 18]    ; XResolution
    mov [vbe_width], ax
    
    mov ax, [VBE_MODE_INFO + 20]    ; YResolution
    mov [vbe_height], ax
    
    mov al, [VBE_MODE_INFO + 25]    ; BitsPerPixel
    mov [vbe_bpp], al
    
    ; Set the video mode with linear framebuffer (bit 14 = use LFB)
    mov ax, 0x4F02
    mov bx, cx                      ; Mode number
    or bx, 0x4000                   ; Enable linear framebuffer
    int 0x10
    
    cmp ax, 0x004F
    jne .vesa_failed
    
    jmp .vesa_done

.vesa_failed:
    ; VESA failed - fall back to text mode
    mov ax, 0x0003
    int 0x10
    ; Store zeros to indicate no graphics mode
    mov dword [vbe_framebuffer_addr], 0
    mov word [vbe_pitch], 0
    mov word [vbe_width], 0
    mov word [vbe_height], 0
    mov byte [vbe_bpp], 0

.vesa_done:

    ; Check for long mode support
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no_long_mode
    
    mov eax, 0x80000001
    cpuid
    test edx, (1 << 29)
    jz .no_long_mode
    
    ; Prepare for long mode
    cli
    
    ; Enable A20 line
    in al, 0x92
    or al, 2
    out 0x92, al
    
    ; Set up page tables for identity mapping
    ; Page table structure at 0x1000:
    ;   0x1000 - PML4T (1 page = 4KB)
    ;   0x2000 - PDPT  (1 page = 4KB)
    ;   0x3000 - PDT for 0-1GB (1 page = 4KB)
    ;   0x4000 - PDT for 3GB-4GB (1 page = 4KB)
    ; Total: 4 pages = 16KB
    mov edi, 0x1000
    mov cr3, edi
    xor eax, eax
    mov ecx, 4096               ; Clear 16KB (4 pages)
    rep stosd
    mov edi, cr3
    
    ; PML4T[0] -> PDPT at 0x2000
    mov dword [edi], 0x2003
    add edi, 0x1000
    
    ; PDPT[0] -> PDT at 0x3000 (for 0-1GB)
    mov dword [edi], 0x3003
    ; PDPT[3] -> PDT at 0x4000 (for 3GB-4GB, where VESA framebuffer typically is)
    mov dword [edi + 24], 0x4003
    add edi, 0x1000
    
    ; PDT entries for first 1GB (2MB pages) - map first 16MB (8 entries)
    ; This covers: code at 0x7E00, kernel at 0x100000, stack at 0x900000
    mov dword [edi],      0x00000083  ; 0MB - 2MB
    mov dword [edi + 8],  0x00200083  ; 2MB - 4MB
    mov dword [edi + 16], 0x00400083  ; 4MB - 6MB
    mov dword [edi + 24], 0x00600083  ; 6MB - 8MB
    mov dword [edi + 32], 0x00800083  ; 8MB - 10MB (covers stack at 0x900000)
    mov dword [edi + 40], 0x00A00083  ; 10MB - 12MB
    mov dword [edi + 48], 0x00C00083  ; 12MB - 14MB
    mov dword [edi + 56], 0x00E00083  ; 14MB - 16MB
    add edi, 0x1000
    
    ; PDT entries for 3GB-4GB range (for VESA framebuffer)
    ; Map the entire 1GB range with 2MB pages (512 entries)
    ; Physical addresses: 0xC0000000 - 0xFFFFFFFF
    mov ecx, 512
    mov eax, 0xC0000083         ; Start at 3GB, present + writable + huge page
.map_fb_loop:
    mov [edi], eax
    mov dword [edi + 4], 0      ; High 32 bits = 0
    add eax, 0x200000           ; Next 2MB page
    add edi, 8
    dec ecx
    jnz .map_fb_loop
    
    ; Enable PAE
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax
    
    ; Enable long mode (LME)
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
    ; Fallback to 32-bit protected mode
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

;=============================================================================
; 32-bit Protected Mode
;=============================================================================

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

;=============================================================================
; 64-bit Long Mode
;=============================================================================

bits 64
long_mode_start:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    
    ;=========================================================================
    ; GRAPHICS MODE - Framebuffer is already set up by VESA in real mode
    ; The kernel will use the framebuffer address stored in vbe_framebuffer_addr
    ;=========================================================================
    
    ; Jump to 64-bit Kernel loader
    jmp load_kernel

;=============================================================================
; Global Descriptor Table
;=============================================================================
; GDT must be in the same section to be accessible in real mode
align 8
gdt:
    dq 0x0000000000000000   ; Null descriptor
    dq 0x00AF9A000000FFFF   ; 64-bit code segment
    dq 0x00CF92000000FFFF   ; Data segment

gdt_descriptor:
    dw gdt_descriptor - gdt - 1
    dd gdt

;=============================================================================
; VBE Information Storage (accessible from kernel)
;=============================================================================
section .data
align 4
vbe_framebuffer_addr:   dd 0    ; Physical address of framebuffer
vbe_pitch:              dw 0    ; Bytes per scanline
vbe_width:              dw 0    ; Screen width in pixels
vbe_height:             dw 0    ; Screen height in pixels
vbe_bpp:                db 0    ; Bits per pixel
