; Nafuraito Stage 1 Bootloader
;
; Master Boot Record (MBR) - First 512 bytes loaded by BIOS
; Loads Stage 2 bootloader from disk and transfers control to it.
; Uses LBA extensions for better compatibility.

org 0x7C00
bits 16

%macro print 1
    mov al, %1
    mov ah, 0x0E
    xor bh, bh
    int 0x10
%endmacro

; Serial output macro (COM1 = 0x3F8)
%macro serial_char 1
    mov dx, 0x3F8
    mov al, %1
    out dx, al
%endmacro

start:
    ; Set up segments and stack
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    
    ; Save boot drive number IMMEDIATELY before any serial output (which clobbers DX)
    mov [drive_number], dl
    
    sti

    ; Serial debug: Stage1 started
    serial_char 'S'
    serial_char '1'
    serial_char ':'
    serial_char 'S'
    serial_char 10
    
    ; Print drive number for debugging (as hex)
    serial_char 'D'
    serial_char ':'
    ; High nibble
    mov al, [drive_number]
    shr al, 4
    add al, '0'
    cmp al, '9'
    jbe .print_hi
    add al, 7               ; Convert A-F
.print_hi:
    mov dx, 0x3F8
    out dx, al
    ; Low nibble
    mov al, [drive_number]
    and al, 0x0F
    add al, '0'
    cmp al, '9'
    jbe .print_lo
    add al, 7
.print_lo:
    mov dx, 0x3F8
    out dx, al
    serial_char 10
    
    ; Reset disk system
    xor ax, ax
    mov dl, [drive_number]
    int 0x13

    ; Check for LBA extensions
    mov ah, 0x41
    mov bx, 0x55AA
    mov dl, [drive_number]
    int 0x13
    jc .no_lba
    cmp bx, 0xAA55
    jne .no_lba
    
    ; Serial debug: LBA available
    serial_char 'L'
    
    ; Use LBA read (int 0x13, AH=0x42)
    mov byte [retry_count], 3
.retry_lba:
    mov ah, 0x42            ; Extended read
    mov dl, [drive_number]
    mov si, dap             ; Disk Address Packet
    int 0x13
    jnc .read_ok
    
    ; Reset and retry
    serial_char 'F'
    xor ax, ax
    mov dl, [drive_number]
    int 0x13
    dec byte [retry_count]
    jnz .retry_lba
    jmp disk_error

.no_lba:
    ; Fallback to CHS (int 0x13, AH=0x02)
    serial_char 'C'
    
    ; Load Stage 2 at 0x7E00
    mov ax, 0x07E0
    mov es, ax
    xor bx, bx
    
    mov si, 3               ; Retry counter
.retry_chs:
    mov ah, 0x02            ; BIOS read sectors
    mov al, 20              ; Read 20 sectors (10KB)
    mov ch, 0               ; Cylinder 0
    mov cl, 2               ; Start at sector 2 (after MBR)
    mov dh, 0               ; Head 0
    mov dl, [drive_number]
    int 0x13
    jnc .read_ok
    
    ; Reset and retry
    serial_char 'F'
    xor ax, ax
    mov dl, [drive_number]
    int 0x13
    
    dec si
    jnz .retry_chs
    jmp disk_error

.read_ok:
    ; Serial debug: Stage2 loaded, jumping
    serial_char 'S'
    serial_char '1'
    serial_char ':'
    serial_char 'J'
    serial_char 10

    ; Reset segments and jump to Stage 2
    xor ax, ax
    mov es, ax
    mov ds, ax
    jmp 0x0000:0x7E00

disk_error:
    print 'E'
    print 'R'
    print 'R'
    cli
    hlt

; Data section
align 4
dap:                        ; Disk Address Packet (16 bytes)
    db 0x10                 ; Size of DAP (16 bytes)
    db 0                    ; Reserved
    dw 20                   ; Number of sectors to read (20 = 10KB)
    dw 0x0000               ; Offset to load to
    dw 0x07E0               ; Segment to load to (0x07E0:0x0000 = 0x7E00)
    dq 1                    ; LBA of first sector to read (sector 1, after MBR)

drive_number: db 0
retry_count: db 3

; Pad to 510 bytes and add boot signature
times 510-($-$$) db 0
dw 0xAA55
