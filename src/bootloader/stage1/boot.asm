; Nafuraito Stage 1 Bootloader
;
; Master Boot Record (MBR) - First 512 bytes loaded by BIOS
; Loads Stage 2 bootloader from disk and transfers control to it.

org 0x7C00
bits 16

%macro print 1
    mov al, %1
    mov ah, 0x0E
    xor bh, bh
    int 0x10
%endmacro

start:
    ; Set up segments and stack
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    ; Save boot drive number
    mov [drive_number], dl
    
    ; Reset disk system
    xor ax, ax
    mov dl, [drive_number]
    int 0x13
    
    ; Load Stage 2 at 0x7E00 (immediately after MBR in memory)
    mov ax, 0x07E0
    mov es, ax
    xor bx, bx              ; ES:BX = 0x07E0:0x0000 = 0x7E00
    
    mov si, 3               ; Retry counter
.retry:
    mov ah, 0x02            ; BIOS read sectors
    mov al, 20              ; Read 20 sectors (10KB)
    mov ch, 0               ; Cylinder 0
    mov cl, 2               ; Start at sector 2 (after MBR)
    mov dh, 0               ; Head 0
    mov dl, [drive_number]
    int 0x13
    jnc .read_ok            ; Jump if no error (CF=0)
    
    ; Reset disk and retry on error
    xor ax, ax
    mov dl, [drive_number]
    int 0x13
    
    dec si
    jnz .retry
    jmp disk_error          ; All retries failed

.read_ok:
    ; Reset segments and jump to Stage 2
    xor ax, ax
    mov es, ax
    mov ds, ax
    jmp 0x0000:0x7E00

disk_error:
    print 'E'
    print 'R'
    print 'R'
    print 'O'
    print 'R'
    print ' '
    print 'W'
    print 'H'
    print 'I'
    print 'L'
    print 'E'
    print ' '
    print 'R'
    print 'E'
    print 'A'
    print 'D'
    print 'I'
    print 'N'
    print 'G'
    print ' '
    print 'F'
    print 'R'
    print 'O'
    print 'M'
    print ' '
    print 'D'
    print 'I'
    print 'S'
    print 'K'
    print '!'
    cli
    hlt

drive_number: db 0

; Pad to 510 bytes and add boot signature
times 510-($-$$) db 0
dw 0xAA55
