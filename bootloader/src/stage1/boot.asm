; =============================================================================
; Nafuraito Stage 1 Bootloader
; =============================================================================
;
; This is the Master Boot Record (MBR).
;
; What it does:
; 1. Set up a basic environment (segments, stack, etc.)
; 2. Load the Stage 2 bootloader from the disk
; 3. Jump to Stage 2 and let it do the heavy lifting
;
; This supports both modern LBA (Logical Block Addressing) and legacy CHS
; (Cylinder-Head-Sector) disk access, because we never know what kind of
; hardware it's running on.
;
; =============================================================================

org 0x7C00              ; Tell the assembler we'll be loaded at 0x7C00
bits 16                 ; We start in 16-bit real mode (thanks, BIOS!)

; -----------------------------------------------------------------------------
; Macro: print
; Prints a single character to the screen using BIOS interrupt 0x10
; This is our "printf" before we have a real display driver
; -----------------------------------------------------------------------------
%macro print 1
    mov al, %1          ; Character to print goes in AL
    mov ah, 0x0E        ; BIOS teletype function (fancy name for "print char")
    xor bh, bh          ; Page number 0 (we don't care about multiple pages)
    int 0x10            ; Call BIOS video services
%endmacro

; -----------------------------------------------------------------------------
; Macro: serial_char
; Sends a character to the serial port (COM1) for debugging
; Very useful when you don't have a screen or things go wrong early
; -----------------------------------------------------------------------------
%macro serial_char 1
    mov dx, 0x3F8       ; COM1 base I/O port address
    mov al, %1          ; Character to send
    out dx, al          ; Send it out the serial port
%endmacro

; -----------------------------------------------------------------------------
; Entry Point: start
; This is where the BIOS jumps to after loading us into memory
; -----------------------------------------------------------------------------
start:
    ; -------------------------------------------------------------------------
    ; Step 1: Set up the environment
    ; The BIOS doesn't guarantee anything about segment registers or the stack,
    ; so we need to set them up ourselves to have a clean, predictable state
    ; -------------------------------------------------------------------------
    cli                 ; Disable interrupts while we mess with segments/stack
                        ; (we don't want an interrupt firing with a broken stack!)
    
    xor ax, ax          ; Zero out AX (faster than "mov ax, 0")
    mov ds, ax          ; Set Data Segment to 0
    mov es, ax          ; Set Extra Segment to 0
    mov ss, ax          ; Set Stack Segment to 0
    mov sp, 0x7C00      ; Set Stack Pointer to 0x7C00 (grows downward from us)
                        ; Stack layout: [free memory] <- SP growing down | 0x7C00: [our code]
    
    ; -------------------------------------------------------------------------
    ; Step 2: Save the boot drive number
    ; The BIOS kindly tells us which drive we booted from by putting the drive
    ; number in DL. We need to save this NOW because we're about to clobber DL
    ; with our debug output, and we'll need it later to read from the same disk
    ; -------------------------------------------------------------------------
    mov [drive_number], dl
    
    sti                 ; Re-enable interrupts now that we're set up

    ; -------------------------------------------------------------------------
    ; Step 3: Send debug messages to serial port
    ; If something goes wrong, at least we can see where we got to by watching
    ; the serial output (connect a serial console to COM1)
    ; -------------------------------------------------------------------------
    serial_char 'S'     ; Output "S1:S\n" meaning "Stage 1: Started"
    serial_char '1'
    serial_char ':'
    serial_char 'S'
    serial_char 10      ; Newline character
    
    ; -------------------------------------------------------------------------
    ; Print the boot drive number in hex for debugging
    ; Helps us verify we saved the right drive number
    ; -------------------------------------------------------------------------
    serial_char 'D'     ; "D:" prefix for "Drive:"
    serial_char ':'
    
    ; High nibble (upper 4 bits) of drive number
    mov al, [drive_number]
    shr al, 4           ; Shift right 4 bits to get high nibble
    add al, '0'         ; Convert to ASCII ('0'-'9')
    cmp al, '9'         ; Is it a letter (A-F)?
    jbe .print_hi       ; If not, we're good
    add al, 7           ; Adjust for letters: 'A' = '9' + 7
.print_hi:
    mov dx, 0x3F8       ; Send to COM1
    out dx, al
    
    ; Low nibble (lower 4 bits) of drive number
    mov al, [drive_number]
    and al, 0x0F        ; Mask to get only low nibble
    add al, '0'         ; Convert to ASCII
    cmp al, '9'         ; Is it a letter (A-F)?
    jbe .print_lo       ; If not, we're good
    add al, 7           ; Adjust for letters
.print_lo:
    mov dx, 0x3F8       ; Send to COM1
    out dx, al
    serial_char 10      ; Newline
    
    ; -------------------------------------------------------------------------
    ; Step 4: Reset the disk system
    ; Sometimes the disk controller is in a weird state; resetting it ensures
    ; we start from a clean slate before trying to read anything
    ; -------------------------------------------------------------------------
    xor ax, ax          ; AH=0: Reset disk system
    mov dl, [drive_number]
    int 0x13            ; Call BIOS disk services

    ; -------------------------------------------------------------------------
    ; Step 5: Check if LBA extensions are available
    ; Modern disks use LBA (Logical Block Addressing) which is much simpler
    ; than the old CHS (Cylinder-Head-Sector) addressing. Let's see if the
    ; BIOS supports it. INT 0x13, AH=0x41 checks for LBA support.
    ; -------------------------------------------------------------------------
    mov ah, 0x41        ; Check Extensions Present function
    mov bx, 0x55AA      ; Magic value required by BIOS
    mov dl, [drive_number]
    int 0x13
    jc .no_lba          ; If carry flag is set, LBA is not supported - fall back to CHS
    cmp bx, 0xAA55      ; BIOS flips the magic value if LBA is supported
    jne .no_lba         ; If it's not flipped, no LBA support
    
    ; -------------------------------------------------------------------------
    ; LBA is available! Use it to read Stage 2 bootloader
    ; -------------------------------------------------------------------------
    serial_char 'L'     ; Output 'L' for "LBA mode"
    
    ; Set up retry counter in case the disk read fails (flaky hardware happens)
    mov byte [retry_count], 3
.retry_lba:
    mov ah, 0x42        ; Extended Read function (LBA mode)
    mov dl, [drive_number]
    mov si, dap         ; DS:SI points to Disk Address Packet (DAP)
                        ; The DAP tells the BIOS what to read and where to put it
    int 0x13            ; Call BIOS to read sectors
    jnc .read_ok        ; If carry flag is clear, read succeeded!
    
    ; -------------------------------------------------------------------------
    ; Read failed - reset the disk and try again
    ; Disks can be temperamental, especially old hardware or VMs
    ; -------------------------------------------------------------------------
    serial_char 'F'     ; Output 'F' for "Failed"
    xor ax, ax          ; AH=0: Reset disk
    mov dl, [drive_number]
    int 0x13
    dec byte [retry_count]
    jnz .retry_lba      ; Try again if we have retries left
    jmp disk_error      ; All retries exhausted - give up

.no_lba:
    ; -------------------------------------------------------------------------
    ; Fallback to CHS (Cylinder-Head-Sector) addressing
    ; This is the old way of addressing disk sectors, used on ancient hardware
    ; or when LBA extensions aren't available. It's more complex but still works.
    ; -------------------------------------------------------------------------
    serial_char 'C'     ; Output 'C' for "CHS mode"
    
    ; Set up where to load Stage 2: segment 0x07E0, offset 0x0000 = address 0x7E00
    ; (This is right after us in memory)
    mov ax, 0x07E0
    mov es, ax          ; ES:BX will be the destination address
    xor bx, bx          ; BX = 0, so ES:BX = 0x07E0:0x0000 = 0x7E00
    
    mov si, 3           ; Retry counter (3 attempts)
.retry_chs:
    mov ah, 0x02        ; BIOS function: Read Sectors
    mov al, 20          ; Read 20 sectors (each sector is 512 bytes, so 10KB total)
    mov ch, 0           ; Cylinder 0 (start of disk)
    mov cl, 2           ; Start at sector 2 (sector 1 is the MBR we're currently in)
    mov dh, 0           ; Head 0
    mov dl, [drive_number]
    int 0x13            ; Call BIOS disk services
    jnc .read_ok        ; Success! Carry flag clear means it worked
    
    ; -------------------------------------------------------------------------
    ; Read failed - reset and try again
    ; -------------------------------------------------------------------------
    serial_char 'F'     ; Output 'F' for "Failed"
    xor ax, ax          ; AH=0: Reset disk
    mov dl, [drive_number]
    int 0x13
    
    dec si              ; Decrement retry counter
    jnz .retry_chs      ; Try again if we have retries left
    jmp disk_error      ; All retries exhausted - give up

.read_ok:
    ; -------------------------------------------------------------------------
    ; Success! Stage 2 is now loaded at 0x7E00
    ; Time to hand off control and let Stage 2 take over
    ; -------------------------------------------------------------------------
    serial_char 'S'     ; Output "S1:J\n" meaning "Stage 1: Jumping"
    serial_char '1'
    serial_char ':'
    serial_char 'J'
    serial_char 10      ; Newline

    ; Clean up segment registers before jumping
    ; (Stage 2 expects DS=ES=0)
    xor ax, ax
    mov es, ax
    mov ds, ax
    
    ; Far jump to Stage 2 at address 0x0000:0x7E00
    ; This is it - we're done! Stage 2, you're up!
    jmp 0x0000:0x7E00

; -----------------------------------------------------------------------------
; Error Handler: disk_error
; If we get here, we couldn't load Stage 2 from disk after all our retries.
; Print "ERR" to the screen and halt the system. Game over.
; -----------------------------------------------------------------------------
disk_error:
    print 'E'           ; Print "ERR" to the screen
    print 'R'
    print 'R'
    cli                 ; Disable interrupts
    hlt                 ; Halt the CPU (nothing more we can do)

; =============================================================================
; Data Section
; =============================================================================

align 4

; -----------------------------------------------------------------------------
; DAP: Disk Address Packet for LBA reads
; This 16-byte structure tells the BIOS exactly what we want to read and where
; to put it. We pass a pointer to this structure when calling INT 0x13, AH=0x42
; -----------------------------------------------------------------------------
dap:
    db 0x10             ; Byte 0: Size of this DAP structure (16 bytes)
    db 0                ; Byte 1: Reserved (must be 0)
    dw 20               ; Bytes 2-3: Number of sectors to read (20 sectors = 10KB)
                        ; Stage 2 bootloader is 10KB, fits in 20 sectors
    dw 0x0000           ; Bytes 4-5: Offset to load to (0x0000)
    dw 0x07E0           ; Bytes 6-7: Segment to load to (0x07E0)
                        ; Together: 0x07E0:0x0000 = physical address 0x7E00
    dq 1                ; Bytes 8-15: Starting LBA (Logical Block Address)
                        ; LBA 0 = this MBR, LBA 1 = first sector of Stage 2

; -----------------------------------------------------------------------------
; Variables
; -----------------------------------------------------------------------------
drive_number: db 0      ; Boot drive number (set by BIOS in DL at startup)
retry_count: db 3       ; Number of retries left for disk operations

; -----------------------------------------------------------------------------
; Boot Signature
; Pad the bootloader to exactly 510 bytes, then add the magic boot signature.
; The BIOS looks for 0xAA55 at bytes 510-511 to identify this as bootable.
; Without this signature, the BIOS won't even try to run our code!
; -----------------------------------------------------------------------------
times 510-($-$$) db 0   ; Pad with zeros to byte 510
dw 0xAA55               ; Magic boot signature (0x55 0xAA in little-endian)
