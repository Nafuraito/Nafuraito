global load_kernel

bits 64

; Constants
KERNEL_LOAD_ADDR    equ 0x100000    ; 1MB - where to load kernel
KERNEL_START_SECTOR equ 64          ; Sector where kernel starts on disk
KERNEL_SECTORS      equ 32          ; Number of sectors to read (16KB)
ATA_PRIMARY_DATA    equ 0x1F0
ATA_PRIMARY_ERROR   equ 0x1F1
ATA_PRIMARY_SECCOUNT equ 0x1F2
ATA_PRIMARY_LBA_LO  equ 0x1F3
ATA_PRIMARY_LBA_MID equ 0x1F4
ATA_PRIMARY_LBA_HI  equ 0x1F5
ATA_PRIMARY_DRIVE   equ 0x1F6
ATA_PRIMARY_STATUS  equ 0x1F7
ATA_PRIMARY_CMD     equ 0x1F7

section .text

load_kernel:
    ;=========================================================================
    ; VGA INITIALIZATION - Clear screen
    ;=========================================================================
    
    ; Clear the entire VGA text buffer
    mov rdi, 0xB8000
    mov rcx, 2000
    mov rax, 0x0F200F200F200F20
    rep stosq

    ;=========================================================================
    ; LOAD KERNEL VIA ATA PIO
    ;=========================================================================
    
    ; Wait for drive to be ready
    call ata_wait_ready
    test rax, rax
    jz .ata_error

    ; Read kernel sectors
    mov rdi, KERNEL_LOAD_ADDR   ; Destination address
    mov rsi, KERNEL_START_SECTOR ; Starting LBA
    mov rcx, KERNEL_SECTORS      ; Number of sectors
    call ata_read_sectors
    test rax, rax
    jz .read_error

    ; Verify we read something (check first bytes aren't all zero)
    mov rax, [KERNEL_LOAD_ADDR]
    test rax, rax
    jz .empty_kernel
    
    ; Set up stack for kernel
    mov rsp, 0x900000
    
    ; Jump to kernel!
    mov rax, KERNEL_LOAD_ADDR
    jmp rax

.ata_error:
    jmp .halt

.read_error:
    jmp .halt

.empty_kernel:
    jmp .halt

.halt:
    cli
    hlt
    jmp .halt

;=============================================================================
; ATA PIO FUNCTIONS
;=============================================================================

; Wait for ATA drive to be ready
; Returns: rax = 1 if ready, 0 if timeout/error
ata_wait_ready:
    push rcx
    push rdx
    
    mov rcx, 100000         ; Timeout counter
.wait_loop:
    mov dx, ATA_PRIMARY_STATUS
    in al, dx
    
    ; Check for error (bit 0)
    test al, 0x01
    jnz .error
    
    ; Check BSY (bit 7) - should be 0
    test al, 0x80
    jnz .continue
    
    ; Check DRQ or DRDY
    test al, 0x48           ; DRQ (bit 3) or DRDY (bit 6)
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

; Wait for data request ready
ata_wait_drq:
    push rdx
    mov rcx, 100000
.wait:
    mov dx, ATA_PRIMARY_STATUS
    in al, dx
    test al, 0x80           ; BSY
    jnz .wait
    test al, 0x01           ; ERR
    jnz .error
    test al, 0x08           ; DRQ
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

; Read sectors from ATA drive using PIO
; rdi = destination buffer
; rsi = starting LBA (sector number)
; rcx = number of sectors to read
; Returns: rax = 1 on success, 0 on failure
ata_read_sectors:
    push rbx
    push rcx
    push rdx
    push rdi
    push rsi
    push r8
    push r9
    
    mov r8, rdi             ; Save destination
    mov r9, rcx             ; Save sector count
    
.read_next_sector:
    ; Wait for drive ready
    call ata_wait_ready
    test rax, rax
    jz .fail
    
    ; Select drive and set high LBA bits (LBA mode)
    mov dx, ATA_PRIMARY_DRIVE
    mov rax, rsi
    shr rax, 24
    and al, 0x0F            ; Bits 24-27 of LBA
    or al, 0xE0             ; LBA mode, master drive
    out dx, al
    
    ; Set sector count = 1
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
    
    ; Read 256 words (512 bytes = 1 sector)
    mov dx, ATA_PRIMARY_DATA
    mov rdi, r8
    mov rcx, 256
    rep insw
    
    ; Update pointers
    add r8, 512             ; Next destination
    inc rsi                 ; Next sector
    dec r9                  ; Decrement count
    jnz .read_next_sector
    
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
