; =============================================================================
; Nafuraito Assembly Bridge (Stage 2 Bootloader)
; =============================================================================
;
; This must be loaded sperately since the bootloader (stage 1) is restricted to only
; 512 Bytes in size, which is too small for all of this.
;
; What it does:
; 1. Get system information (memory map, video mode)
; 2. Transition from 16-bit real mode -> 32-bit protected mode -> 64-bit long mode
; 3. Set up paging (required for 64-bit mode)
; 4. Load the kernel from the EXT4 filesystem
; 5. Jump to the kernel
;
; Memory Layout we're using:
;   0x0500 - 0x0600: E820 memory map (saved from BIOS)
;   0x7C00 - 0x7E00: Stage 1 bootloader (still in memory)
;   0x7E00 - 0xA600: Stage 2 bootloader (this code)
;   0x9000 - 0xA000: Page tables (PML4, PDPT, PD, PT)
;   0x100000+: Kernel load address (1MB, away from all bootloader stuff)
;
; =============================================================================

; Use default section for entry
[SECTION .text.entry]
[BITS 16]

global entry
entry:
    ; -------------------------------------------------------------------------
    ; Send debug message to serial port
    ; Output "S2:S" meaning "Stage 2: Started"
    ; -------------------------------------------------------------------------
    mov dx, 0x3F8
    mov al, 'S'
    out dx, al
    mov al, '2'
    out dx, al
    mov al, ':'
    out dx, al
    mov al, 'S'
    out dx, al
    mov al, 10
    out dx, al

    ; -------------------------------------------------------------------------
    ; Initialize segment registers and stack
    ; In real mode, we need to set up segments. Zero them for simplicity.
    ; -------------------------------------------------------------------------
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov sp, 0x7C00              ; Stack below Stage 1
                                  ; Stack grows downward from 0x7C00

    ; -------------------------------------------------------------------------
    ; Get the memory map from BIOS via E820 interface
    ; This tells us which regions of RAM are usable and which are reserved
    ; We MUST do this now while we still have BIOS access!
    ; -------------------------------------------------------------------------
    call get_memory_map

    ; -------------------------------------------------------------------------
    ; Set up VESA VBE graphics mode
    ; This queries the BIOS for available video modes and picks the best one
    ; (highest resolution, 32-bit color)
    ; -------------------------------------------------------------------------
    call setup_vesa

    ; -------------------------------------------------------------------------
    ; Enable the A20 line
    ; On ancient PCs, address line 20 (A20) was disabled for compatibility.
    ; If we don't enable it, we can't access memory above 1MB!
    ; This is a historical quirk we have to deal with.
    ; -------------------------------------------------------------------------
    call enable_a20

    ; -------------------------------------------------------------------------
    ; Load the GDT (Global Descriptor Table)
    ; The GDT defines memory segments. Required for protected mode.
    ; -------------------------------------------------------------------------
    lgdt [gdt_descriptor]

    ; -------------------------------------------------------------------------
    ; Disable interrupts before switching modes
    ; Interrupts in real mode point to BIOS handlers. In protected mode,
    ; those addresses are meaningless. Disable them to avoid crashes!
    ; -------------------------------------------------------------------------
    cli

    ; -------------------------------------------------------------------------
    ; Debug message: "S2:P" = "Stage 2: Protected mode"
    ; -------------------------------------------------------------------------
    mov dx, 0x3F8
    mov al, 'S'
    out dx, al
    mov al, '2'
    out dx, al
    mov al, ':'
    out dx, al
    mov al, 'P'
    out dx, al
    mov al, 10
    out dx, al

    ; -------------------------------------------------------------------------
    ; Enable Protected Mode!
    ; CR0 (Control Register 0) has a PE (Protection Enable) bit.
    ; Setting it switches the CPU to 32-bit protected mode.
    ; -------------------------------------------------------------------------
    mov eax, cr0
    or eax, 1               ; Set bit 0 (PE = Protection Enable)
    mov cr0, eax

    ; -------------------------------------------------------------------------
    ; Far jump to flush the pipeline and start executing 32-bit code
    ; The CPU's instruction pipeline still has 16-bit instructions in it.
    ; A far jump clears the pipeline and loads the new code segment selector.
    ; -------------------------------------------------------------------------
    jmp dword 0x08:protected_mode_entry
                            ; 0x08 = segment selector for code segment in GDT

; ============================================================================
; Setup VESA VBE Graphics Mode (Auto-detect max resolution)
; ============================================================================
[BITS 16]
setup_vesa:
    ; Get VBE Controller Info
    mov ax, 0x4F00
    mov di, vbe_info_block
    int 0x10
    cmp ax, 0x004F
    jne .vesa_fail

    ; Initialize best mode tracking
    mov word [best_mode], 0
    mov dword [best_pixels], 0
    mov byte [best_bpp], 0

    ; Get pointer to mode list (far pointer at offset 14)
    mov si, [vbe_info_block + 14]   ; Offset of mode list
    mov ax, [vbe_info_block + 16]   ; Segment of mode list
    mov fs, ax                       ; Use FS for mode list segment

.scan_modes:
    ; Read mode number from list
    mov cx, [fs:si]
    cmp cx, 0xFFFF                  ; End of list?
    je .set_best_mode
    
    ; Save mode number and pointer BEFORE BIOS call (BIOS can clobber CX)
    mov [current_mode], cx
    add si, 2                       ; Next mode in list
    push si                         ; Save mode list pointer
    push fs                         ; Save mode list segment

    ; Get mode info - BIOS may clobber CX, SI, etc.
    mov ax, 0x4F01
    mov cx, [current_mode]          ; Mode number for query
    mov di, mode_info_block
    int 0x10
    
    pop fs                          ; Restore mode list segment
    pop si                          ; Restore mode list pointer
    
    cmp ax, 0x004F
    jne .scan_modes                 ; Skip if failed

    ; Check mode attributes
    mov ax, [mode_info_block]       ; Mode attributes
    test ax, 0x01                   ; Bit 0 = mode supported
    jz .scan_modes
    test ax, 0x08                   ; Bit 3 = color mode
    jz .scan_modes
    test ax, 0x80                   ; Bit 7 = LFB available
    jz .scan_modes

    ; Check bits per pixel (we want 32-bit or 24-bit)
    mov al, [mode_info_block + 25]  ; BitsPerPixel
    cmp al, 32
    je .check_resolution
    cmp al, 24
    je .check_resolution
    jmp .scan_modes

.check_resolution:
    ; Calculate total pixels (width * height) using 16-bit math
    ; to avoid potential issues with 32-bit ops in real mode
    mov ax, [mode_info_block + 18]  ; XResolution
    mov bx, [mode_info_block + 20]  ; YResolution
    
    ; Use 32-bit multiply: eax = ax * bx
    movzx eax, ax
    movzx ebx, bx
    imul eax, ebx                   ; eax = total pixels

    ; Compare with best so far
    cmp eax, [best_pixels]
    jb .scan_modes                  ; Less pixels, skip
    je .check_bpp                   ; Same pixels, check BPP

    ; More pixels - this is the new best
    jmp .new_best

.check_bpp:
    ; Same pixel count - prefer higher BPP
    mov dl, [mode_info_block + 25]  ; Current BPP
    cmp dl, [best_bpp]
    jbe .scan_modes                 ; Same or worse BPP, skip

.new_best:
    ; Save this as best mode
    mov [best_pixels], eax
    mov cx, [current_mode]          ; Get saved mode number
    mov [best_mode], cx
    mov dl, [mode_info_block + 25]  ; BPP
    mov [best_bpp], dl
    jmp .scan_modes

.set_best_mode:
    ; Check if we found any mode
    mov cx, [best_mode]
    test cx, cx
    jz .vesa_fail

    ; Get mode info for best mode (to populate mode_info_block correctly)
    mov ax, 0x4F01
    ; CX already contains best_mode
    mov di, mode_info_block
    int 0x10
    cmp ax, 0x004F
    jne .vesa_fail

    ; Set the mode with LFB enabled
    mov ax, 0x4F02
    mov bx, [best_mode]
    or bx, 0x4000                   ; Enable LFB
    int 0x10
    cmp ax, 0x004F
    jne .vesa_fail

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

    ; Restore FS
    xor ax, ax
    mov fs, ax
    ret

.vesa_fail:
    ; Restore FS and set defaults
    xor ax, ax
    mov fs, ax
    mov dword [vbe_framebuffer_addr], 0
    mov word [vbe_pitch], 0
    mov word [vbe_width], 0
    mov word [vbe_height], 0
    mov byte [vbe_bpp], 0
    ret

; ============================================================================
; Get Memory Map via E820
; ============================================================================
[BITS 16]
get_memory_map:
    mov di, 0x500               ; Memory map destination (safe low memory, after BDA)
    mov [memory_map_addr], di   ; Store address
    xor ebx, ebx                ; Continuation value (0 to start)
    xor bp, bp                  ; Entry counter

.e820_loop:
    mov eax, 0xE820             ; E820 function
    mov ecx, 24                 ; Buffer size (24 bytes per entry)
    mov edx, 0x534D4150         ; 'SMAP' signature
    int 0x15

    jc .e820_done               ; Carry set = error or done
    cmp eax, 0x534D4150         ; Check signature
    jne .e820_done

    ; Valid entry
    inc bp                      ; Increment counter
    add di, 24                  ; Next entry position

    test ebx, ebx               ; If ebx = 0, we're done
    jz .e820_done

    jmp .e820_loop

.e820_done:
    mov [memory_map_entries], bp
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
; Check CPU Features (LA57, NX, etc.)
; ============================================================================
[BITS 32]
check_cpu_features:
    ; Save registers
    push ebx
    push ecx
    push edx

    ; Check for CPUID support (toggle bit 21 in EFLAGS)
    pushfd
    pop eax
    mov ecx, eax
    xor eax, (1 << 21)
    push eax
    popfd
    pushfd
    pop eax
    push ecx
    popfd
    xor eax, ecx
    jz .no_cpuid

    ; Check maximum CPUID leaf
    xor eax, eax
    cpuid
    cmp eax, 7
    jb .no_la57                 ; Need leaf 7 for LA57 check

    ; Check LA57 support: CPUID.07H:ECX.LA57[bit 16]
    mov eax, 7
    xor ecx, ecx
    cpuid
    test ecx, (1 << 16)
    jz .no_la57
    mov byte [cpu_la57_supported], 1
    jmp .check_nx

.no_la57:
    mov byte [cpu_la57_supported], 0

.check_nx:
    ; Check NX support: CPUID.80000001H:EDX.NX[bit 20]
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no_nx

    mov eax, 0x80000001
    cpuid
    test edx, (1 << 20)
    jz .no_nx
    mov byte [cpu_nx_supported], 1
    jmp .done

.no_nx:
    mov byte [cpu_nx_supported], 0
    jmp .done

.no_cpuid:
    mov byte [cpu_la57_supported], 0
    mov byte [cpu_nx_supported], 0

.done:
    ; Restore registers
    pop edx
    pop ecx
    pop ebx
    ret

; ============================================================================
; 32-bit Protected Mode Entry
; ============================================================================
[SECTION .text]
[BITS 32]

protected_mode_entry:
    ; Serial debug: In protected mode
    mov dx, 0x3F8
    mov al, 'P'
    out dx, al
    mov al, 'M'
    out dx, al
    mov al, 10
    out dx, al

    ; Set up segment registers for 32-bit mode
    mov ax, 0x10                ; Data segment selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x90000

    ; Check CPU features (LA57, NX, etc.)
    call check_cpu_features

    ; Set up paging for long mode (will use LA57 if available)
    call setup_paging

    ; Enable PAE (Physical Address Extension)
    mov eax, cr4
    or eax, (1 << 5)            ; Set PAE bit
    
    ; Enable LA57 if supported (must be done before enabling paging!)
    mov bl, [cpu_la57_supported]
    test bl, bl
    jz .no_la57
    or eax, (1 << 12)           ; Set LA57 bit in CR4
    mov byte [cpu_la57_enabled], 1
.no_la57:
    mov cr4, eax

    ; Load PML4 or PML5 address into CR3
    mov eax, [paging_root_addr]
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
; Setup Paging (Identity map first 4GB with LA57 support)
; For LA57: Page tables at 0x10000-0x18000
; For LA48: Page tables at 0x10000-0x16000
; ============================================================================
[BITS 32]
setup_paging:
    ; Check if LA57 is supported
    mov al, [cpu_la57_supported]
    test al, al
    jnz .setup_la57

    ; Setup 4-level paging (LA48)
    call setup_paging_4level
    mov dword [paging_root_addr], 0x10000  ; PML4 at 0x10000
    ret

.setup_la57:
    ; Setup 5-level paging (LA57)
    call setup_paging_5level
    mov dword [paging_root_addr], 0x10000  ; PML5 at 0x10000
    ret

; ============================================================================
; Setup 4-Level Paging (LA48)
; Page tables located at 0x10000-0x16000
; ============================================================================
[BITS 32]
setup_paging_4level:
    ; Clear page table area (0x10000 - 0x16000)
    mov edi, 0x10000
    xor eax, eax
    mov ecx, 0x6000 / 4         ; 24KB / 4 bytes = 6144 dwords
    rep stosd

    ; PML4[0] -> PDPT at 0x11000
    mov dword [0x10000], 0x11003  ; Present + Writable + PDPT address

    ; PDPT[0] -> PD at 0x12000 (first 1GB)
    mov dword [0x11000], 0x12003  ; Present + Writable + PD address

    ; PDPT[1] -> PD at 0x13000 (second 1GB)
    mov dword [0x11008], 0x13003

    ; PDPT[2] -> PD at 0x14000 (third 1GB)
    mov dword [0x11010], 0x14003

    ; PDPT[3] -> PD at 0x15000 (fourth 1GB)
    mov dword [0x11018], 0x15003

    ; Fill Page Directories with 2MB pages (identity mapping)
    ; PD at 0x12000: maps 0x00000000 - 0x3FFFFFFF
    mov edi, 0x12000
    mov eax, 0x83               ; Present + Writable + 2MB page
    mov ecx, 512
.fill_pd1:
    mov [edi], eax
    add eax, 0x200000           ; 2MB per entry
    add edi, 8
    loop .fill_pd1

    ; PD at 0x13000: maps 0x40000000 - 0x7FFFFFFF
    mov edi, 0x13000
    mov eax, 0x40000083
    mov ecx, 512
.fill_pd2:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .fill_pd2

    ; PD at 0x14000: maps 0x80000000 - 0xBFFFFFFF
    mov edi, 0x14000
    mov eax, 0x80000083
    mov ecx, 512
.fill_pd3:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .fill_pd3

    ; PD at 0x15000: maps 0xC0000000 - 0xFFFFFFFF
    mov edi, 0x15000
    mov eax, 0xC0000083
    mov ecx, 512
.fill_pd4:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .fill_pd4

    ret

; ============================================================================
; Setup 5-Level Paging (LA57)
; Page tables located at 0x10000-0x18000
; PML5 -> PML4 -> PDPT -> PD (2MB pages)
; ============================================================================
[BITS 32]
setup_paging_5level:
    ; Clear page table area (0x10000 - 0x18000) - 32KB
    mov edi, 0x10000
    xor eax, eax
    mov ecx, 0x8000 / 4         ; 32KB / 4 bytes = 8192 dwords
    rep stosd

    ; PML5[0] -> PML4 at 0x11000
    mov dword [0x10000], 0x11003  ; Present + Writable + PML4 address

    ; PML4[0] -> PDPT at 0x12000
    mov dword [0x11000], 0x12003  ; Present + Writable + PDPT address

    ; PDPT[0] -> PD at 0x13000 (first 1GB)
    mov dword [0x12000], 0x13003  ; Present + Writable + PD address

    ; PDPT[1] -> PD at 0x14000 (second 1GB)
    mov dword [0x12008], 0x14003

    ; PDPT[2] -> PD at 0x15000 (third 1GB)
    mov dword [0x12010], 0x15003

    ; PDPT[3] -> PD at 0x16000 (fourth 1GB)
    mov dword [0x12018], 0x16003

    ; Fill Page Directories with 2MB pages (identity mapping)
    ; PD at 0x13000: maps 0x00000000 - 0x3FFFFFFF
    mov edi, 0x13000
    mov eax, 0x83               ; Present + Writable + 2MB page
    mov ecx, 512
.fill_pd1:
    mov [edi], eax
    add eax, 0x200000           ; 2MB per entry
    add edi, 8
    loop .fill_pd1

    ; PD at 0x14000: maps 0x40000000 - 0x7FFFFFFF
    mov edi, 0x14000
    mov eax, 0x40000083
    mov ecx, 512
.fill_pd2:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .fill_pd2

    ; PD at 0x15000: maps 0x80000000 - 0xBFFFFFFF
    mov edi, 0x15000
    mov eax, 0x80000083
    mov ecx, 512
.fill_pd3:
    mov [edi], eax
    add eax, 0x200000
    add edi, 8
    loop .fill_pd3

    ; PD at 0x16000: maps 0xC0000000 - 0xFFFFFFFF
    mov edi, 0x16000
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
    ; Serial debug: In long mode
    mov dx, 0x3F8
    mov al, 'L'
    out dx, al
    mov al, 'M'
    out dx, al
    mov al, 10
    out dx, al

    ; Set up 64-bit segment registers
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Set up 64-bit stack (use higher address to avoid conflicts)
    mov rsp, 0x9F000

    ; Call kernel loader (this should not return)
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
    ; Set up stack for C code (16-byte aligned)
    ; Use a safe location that won't conflict with anything
     mov rsp, 0x2000000
    and rsp, ~0xF                       ; Ensure 16-byte alignment
    
    ; Reserve space for structures (64 bytes for alignment + red zone safety)
    sub rsp, 64
    
    ; VBE info at rsp+0 (16 bytes)
    mov rdi, rsp                        ; First argument: pointer to vbe_info_t
    
    ; Fill VBE structure
    mov eax, [rel vbe_framebuffer_addr]
    mov [rdi], eax                      ; framebuffer_addr (4 bytes)
    movzx eax, word [rel vbe_pitch]
    mov [rdi + 4], ax                   ; pitch (2 bytes)
    movzx eax, word [rel vbe_width]
    mov [rdi + 6], ax                   ; width (2 bytes)
    movzx eax, word [rel vbe_height]
    mov [rdi + 8], ax                   ; height (2 bytes)
    movzx eax, byte [rel vbe_bpp]
    mov [rdi + 10], al                  ; bpp (1 byte)
    mov byte [rdi + 11], 0              ; padding
    
    ; Memory info at rsp+16 (16 bytes)
    lea rsi, [rsp + 16]                 ; Second argument: pointer to memory_info_t
    
    ; Fill memory info structure
    mov eax, [rel memory_map_addr]
    mov [rsi], eax                      ; memory_map_addr (4 bytes)
    movzx eax, word [rel memory_map_entries]
    mov [rsi + 4], ax                   ; memory_map_entries (2 bytes)
    mov word [rsi + 6], 0               ; padding
    
    ; Ensure stack is 16-byte aligned before call (sub 8 for alignment since call pushes 8)
    sub rsp, 8
    
    ; Serial debug: About to call C loader
    mov dx, 0x3F8
    mov al, 'L'
    out dx, al
    mov al, 'D'
    out dx, al
    mov al, 10
    out dx, al

    ; Call C kernel loader
    call load_kernel_c
    
    ; Restore stack alignment
    add rsp, 8
    
    ; Check if kernel loading succeeded
    test rax, rax
    jz .load_failed
    
    ; Save kernel entry point in a callee-saved register
    mov rbx, rax
    
    ; Set up fresh stack for kernel (16-byte aligned)
     mov rsp, 0x2000000
    and rsp, ~0xF
    
    ; -------------------------------------------------------------------------
    ; Write BootInfo struct to physical 0x6000.
    ; The kernel reads this to detect the boot path and find system resources.
    ; Layout (32 bytes, matches boot_info_t in bootloader/include/boot_info.h):
    ;   +0x00  magic              = 0x4E414649 ("NAFI")
    ;   +0x04  boot_type          = 0x42494F53 ("BIOS")
    ;   +0x08  rsdp_addr          = 0 (BIOS path: no UEFI config table)
    ;   +0x10  mem_map_entry_count
    ;   +0x14  _reserved0         = 0
    ;   +0x18  _reserved1         = 0
    ; -------------------------------------------------------------------------
    mov dword [0x6000], 0x4E414649      ; magic = "NAFI"
    mov dword [0x6004], 0x42494F53      ; boot_type = "BIOS"
    xor rax, rax
    mov qword [0x6008], rax             ; rsdp_addr = 0
    movzx eax, word [rel memory_map_entries]
    mov dword [0x6010], eax             ; mem_map_entry_count
    mov dword [0x6014], 0               ; _reserved0
    mov qword [0x6018], 0               ; _reserved1

    ; Pass boot info to kernel via registers (System V AMD64 ABI):
    ; rdi = framebuffer address (32-bit, zero-extended)
    ; rsi = pitch
    ; rdx = width
    ; rcx = height
    ; r8  = bpp
    ; r9  = memory map address
    ; stack[0] = memory map entry count
    mov edi, [rel vbe_framebuffer_addr]
    movzx rsi, word [rel vbe_pitch]
    movzx rdx, word [rel vbe_width]
    movzx rcx, word [rel vbe_height]
    movzx r8, byte [rel vbe_bpp]
    mov r9d, [rel memory_map_addr]
    
    ; Serial debug: About to jump to kernel
    mov dx, 0x3F8
    mov al, 'J'
    out dx, al
    mov al, 'K'
    out dx, al
    mov al, 10
    out dx, al

    ; Push memory map entry count as 7th argument (on stack)
    ; Stack is 16-byte aligned, push adds 8 bytes, so we need another 8 for alignment
    movzx rax, word [rel memory_map_entries]
    push rax                            ; 7th argument
    sub rsp, 8                          ; Align to 16 bytes before call
    
    ; Jump to kernel entry point (kernel should never return)
    call rbx
    
    ; If kernel returns, fall through to halt
    
.kernel_returned:
    cli
    hlt
    jmp .kernel_returned

.load_failed:
    ; Kernel load failed - halt the system (don't try to return)
    cli
.halt_failed:
    hlt
    jmp .halt_failed

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
global cpu_la57_supported
global cpu_la57_enabled
global cpu_nx_supported

vbe_framebuffer_addr:   dd 0
vbe_pitch:              dw 0
vbe_width:              dw 0
vbe_height:             dw 0
vbe_bpp:                db 0
                        db 0    ; Padding
memory_map_addr:        dd 0
memory_map_entries:     dw 0

; VESA mode selection variables
best_mode:              dw 0
best_pixels:            dd 0
best_bpp:               db 0
                        db 0    ; Padding
current_mode:           dw 0    ; Temp storage for current mode during scan

; CPU feature flags
cpu_la57_supported:     db 0
cpu_la57_enabled:       db 0
cpu_nx_supported:       db 0
                        db 0    ; Padding

; Paging configuration
paging_root_addr:       dd 0    ; Physical address of root page table (PML4 or PML5)

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
    dd gdt64_start              ; Use 32-bit base (loaded from 32-bit mode)
    dd 0                        ; Upper 32 bits (for 64-bit reload later if needed)

; ============================================================================
; VBE Info Buffers (in data section for real mode access)
; ============================================================================
[SECTION .data]

align 16
vbe_info_block:     times 512 db 0
mode_info_block:    times 256 db 0
