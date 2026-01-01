; Nafuraito Kernel Loader (64-bit)
;
; Loads the kernel binary from disk using ATA PIO mode
; and transfers control to the kernel entry point.

global load_kernel

extern vbe_framebuffer_addr
extern vbe_pitch
extern vbe_width
extern vbe_height
extern vbe_bpp

bits 64

;=============================================================================
; Constants
;=============================================================================

KERNEL_LOAD_ADDR    equ 0x100000    ; 1MB - kernel load address
KERNEL_START_SECTOR equ 64          ; Disk sector where kernel starts
KERNEL_SECTORS      equ 32          ; Number of sectors (16KB)

; ATA Primary Bus Ports
ATA_PRIMARY_DATA    equ 0x1F0
ATA_PRIMARY_ERROR   equ 0x1F1
ATA_PRIMARY_SECCOUNT equ 0x1F2
ATA_PRIMARY_LBA_LO  equ 0x1F3
ATA_PRIMARY_LBA_MID equ 0x1F4
ATA_PRIMARY_LBA_HI  equ 0x1F5
ATA_PRIMARY_DRIVE   equ 0x1F6
ATA_PRIMARY_STATUS  equ 0x1F7
ATA_PRIMARY_CMD     equ 0x1F7

;=============================================================================
; Kernel Loader Entry Point
;=============================================================================

section .text

load_kernel:
    ;=========================================================================
    ; Graphics mode is already set by VESA - no VGA text clearing needed
    ;=========================================================================

    ;=========================================================================
    ; LOAD KERNEL VIA ATA PIO
    ;=========================================================================
    
    ; Wait for drive to be ready
    call ata_wait_ready
    test rax, rax
    jz .halt

    ; Read kernel from disk
    mov rdi, KERNEL_LOAD_ADDR
    mov rsi, KERNEL_START_SECTOR
    mov rcx, KERNEL_SECTORS
    call ata_read_sectors
    test rax, rax
    jz .halt

    ; Verify kernel was loaded
    mov rax, [KERNEL_LOAD_ADDR]
    test rax, rax
    jz .halt
    
    ; Set up stack
    mov rsp, 0x900000
    
    ; Pass VBE info to kernel via registers (System V AMD64 calling convention)
    ; rdi = framebuffer address
    ; rsi = pitch
    ; rdx = width
    ; rcx = height
    ; r8  = bpp
    mov edi, [vbe_framebuffer_addr]
    movzx rsi, word [vbe_pitch]
    movzx rdx, word [vbe_width]
    movzx rcx, word [vbe_height]
    movzx r8, byte [vbe_bpp]
    
    ; Jump to kernel
    mov rax, KERNEL_LOAD_ADDR
    jmp rax

.halt:
    cli
    hlt
    jmp .halt

;=============================================================================
; ATA PIO Driver
;=============================================================================

; Wait for ATA drive ready
; Returns: rax = 1 if ready, 0 on error/timeout
ata_wait_ready:
    push rcx
    push rdx
    
    mov rcx, 100000         ; Timeout counter
.wait_loop:
    mov dx, ATA_PRIMARY_STATUS
    in al, dx
    test al, 0x01           ; Error?
    jnz .error
    test al, 0x80           ; Busy?
    jnz .continue
    test al, 0x48           ; Ready or DRQ?
    jnz .ready

.continue:
    dec rcx
    jnz .wait_loop
    
    ; Timeout
    xor rax, rax
    jmp .done

.error:
    xor rax, rax
    jmp .done

.ready:
    mov rax, 1

.done:
    pop rdx
    pop rcx
    ret

; Wait for data request
ata_wait_drq:
    push rdx
    mov rcx, 100000

.wait:
    mov dx, ATA_PRIMARY_STATUS
    in al, dx
    test al, 0x80           ; Busy?
    jnz .wait
    test al, 0x01           ; Error?
    jnz .error
    test al, 0x08           ; DRQ?
    jnz .ready
    dec rcx
    jnz .wait

.error:
    xor rax, rax
    pop rdx
    ret

.ready:
    mov rax, 1
    pop rdx
    ret

; Read sectors from ATA drive
; rdi = destination, rsi = LBA, rcx = sector count
; Returns: rax = 1 on success, 0 on failure
ata_read_sectors:
    push rbx
    push rcx
    push rdx
    push rdi
    push rsi
    push r8
    push r9
    
    mov r8, rdi             ; Destination
    mov r9, rcx             ; Sector count

.read_sector:
    call ata_wait_ready
    test rax, rax
    jz .fail
    
    ; Select drive (LBA mode, master)
    mov dx, ATA_PRIMARY_DRIVE
    mov rax, rsi
    shr rax, 24
    and al, 0x0F            ; Bits 24-27 of LBA
    or al, 0xE0             ; LBA mode, master drive
    out dx, al
    
    ; Sector count = 1
    mov dx, ATA_PRIMARY_SECCOUNT
    mov al, 1
    out dx, al
    
    ; Set LBA low byte (bits 0-7)
    mov dx, ATA_PRIMARY_LBA_LO
    mov rax, rsi
    out dx, al
    
    ; Set LBA mid byte (bits 8-15)
    mov dx, ATA_PRIMARY_LBA_MID
    mov rax, rsi
    shr rax, 8
    out dx, al
    
    ; Set LBA high byte (bits 16-23)
    mov dx, ATA_PRIMARY_LBA_HI
    mov rax, rsi
    shr rax, 16
    out dx, al
    
    ; Send read command
    mov dx, ATA_PRIMARY_CMD
    mov al, 0x20            ; READ SECTORS command
    out dx, al
    
    ; Wait for data
    call ata_wait_drq
    test rax, rax
    jz .fail
    
    ; Read 256 words (512 bytes)
    mov dx, ATA_PRIMARY_DATA
    mov rdi, r8
    mov rcx, 256
    rep insw
    
    ; Update pointers
    add r8, 512             ; Next destination
    inc rsi                 ; Next sector
    dec r9                  ; Decrement count
    jnz .read_sector
    
    ; Success
    mov rax, 1
    jmp .done

.fail:
    xor rax, rax

.done:
    pop r9
    pop r8
    pop rsi
    pop rdi
    pop rdx
    pop rcx
    pop rbx
    ret

section .data
kernel_filename: db "kernel.bin", 0
