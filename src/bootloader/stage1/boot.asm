org 0x7C00
bits 16

%macro print_TTY 1
    mov al, %1
    mov bh, 0
    mov ah, 0Eh
    int 10h
%endmacro

start:
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00          ; Set stack pointer below stage1

    print_TTY 'M'
    print_TTY 'B'
    print_TTY 'R'
    print_TTY ' '

    mov [drive_number], dl
    
    ; Read stage2 using CHS (legacy BIOS)
    ; LBA 1 = CHS (0, 0, 2) - sector numbering starts at 1
    ; Read sectors to load up to 0x93A0
    mov ax, 0x7E0
    mov es, ax
    xor bx, bx              ; ES:BX = 0x7E0:0x0000 = 0x7E00

    print_TTY 'R'
    print_TTY 'D'
    print_TTY ' '
    
    mov ah, 0x02            ; Read sectors
    mov al, 30              ; Number of sectors (0x93A0-0x7E00)/0x200 = ~30 sectors
    mov ch, 0               ; Cylinder 0
    mov cl, 2               ; Sector 2 (sector 1 is MBR)
    mov dh, 0               ; Head 0
    mov dl, [drive_number]
    int 0x13
    jc disk_error
    
    print_TTY 'J'
    print_TTY 'M'
    print_TTY 'P'
    print_TTY ' '

    jmp 0x7E00:0x0000

    print_TTY 'J'
    print_TTY 'E'
    print_TTY 'R'
    print_TTY 'R'
    print_TTY ' '

disk_error:
    print_TTY 'D'
    print_TTY 'E'
    print_TTY 'R'
    print_TTY 'R'
    print_TTY ' '
    cli
    hlt

drive_number: db 0

times 510-($-$$) db 0
dw 0xAA55

buffer:
