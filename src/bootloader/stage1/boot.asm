org 0x7C00
bits 16

start:
    ; Set up segments
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00          ; Set stack pointer below stage1
    sti

    mov [drive_number], dl
    
    ; Reset disk system first
    xor ax, ax
    mov dl, [drive_number]
    int 0x13
    
    ; Read stage2 using CHS (legacy BIOS)
    ; CHS sector 2 = LBA sector 1 (immediately after MBR)
    ; Load stage2 at 0x7E00
    mov ax, 0x07E0          ; Segment 0x07E0
    mov es, ax
    xor bx, bx              ; ES:BX = 0x07E0:0x0000 = 0x7E00
    
    mov si, 3               ; Retry counter
.retry:
    mov ah, 0x02            ; Read sectors function
    mov al, 20              ; Number of sectors to read (10KB)
    mov ch, 0               ; Cylinder 0
    mov cl, 2               ; Start at sector 2 (CHS sectors are 1-based)
    mov dh, 0               ; Head 0
    mov dl, [drive_number]
    int 0x13
    jnc .read_ok            ; Jump if no error (CF=0)
    
    ; Reset disk and retry
    xor ax, ax
    mov dl, [drive_number]
    int 0x13
    
    dec si
    jnz .retry
    jmp disk_error          ; All retries failed

.read_ok:
    
    ; Reset segments before jump
    xor ax, ax
    mov es, ax
    mov ds, ax

    jmp 0x0000:0x7E00

disk_error:
    cli
    hlt

drive_number: db 0

times 510-($-$$) db 0
dw 0xAA55
