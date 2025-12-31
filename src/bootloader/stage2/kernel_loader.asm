global load_kernel

bits 32

section .text

load_kernel:
    ;=========================================================================
    ; VGA REGISTER INITIALIZATION - CRITICAL FOR FIXING FLICKERING
    ;=========================================================================
    ; The BIOS may have left the CRTC Start Address pointing to a non-zero
    ; offset, causing the display to read from the wrong location in video
    ; memory. We need to explicitly set it to 0 so that what we write at
    ; 0xB8000 is actually displayed.
    ;=========================================================================
    
    ; First, ensure we're in color text mode addressing (port 0x3D4, not 0x3B4)
    ; Read Miscellaneous Output Register and set bit 0
    mov dx, 0x3CC           ; Misc Output Read port
    in al, dx
    or al, 0x01             ; Set I/O Address Select bit (use 0x3Dx ports)
    mov dx, 0x3C2           ; Misc Output Write port
    out dx, al
    
    ; Program the Graphics Controller to map memory at 0xB8000 (text mode)
    ; Port 0x3CE index 0x06 = Miscellaneous Graphics Register
    mov dx, 0x3CE           ; Graphics Controller Address port
    mov al, 0x06            ; Index 6: Miscellaneous Register
    out dx, al
    inc dx                  ; 0x3CF: Graphics Controller Data port
    in al, dx               ; Read current value
    and al, 0xF3            ; Clear bits 3-2 (memory map)
    or al, 0x0C             ; Set to 11b = 0xB8000-0xBFFFF (32K color text)
    out dx, al
    
    ; Set CRTC Start Address to 0 (display from beginning of video buffer)
    ; This is THE critical fix for the flickering issue
    ; The Start Address is a 16-bit word offset, so 0 means display starts
    ; at physical address 0xB8000 + (0 * 2) = 0xB8000
    
    mov dx, 0x3D4           ; CRTC Address port
    
    ; Set Start Address High (index 0x0C) to 0
    mov al, 0x0C            ; Index 0x0C: Start Address High Register
    out dx, al
    inc dx                  ; 0x3D5: CRTC Data port
    mov al, 0x00            ; High byte = 0
    out dx, al
    
    ; Set Start Address Low (index 0x0D) to 0
    dec dx                  ; Back to 0x3D4
    mov al, 0x0D            ; Index 0x0D: Start Address Low Register
    out dx, al
    inc dx                  ; 0x3D5: CRTC Data port
    mov al, 0x00            ; Low byte = 0
    out dx, al
    
    ; Disable the text cursor by setting bit 5 of Cursor Start Register
    dec dx                  ; Back to 0x3D4
    mov al, 0x0A            ; Index 0x0A: Cursor Start Register
    out dx, al
    inc dx                  ; 0x3D5
    mov al, 0x20            ; Bit 5 = disable cursor
    out dx, al
    
    ;=========================================================================
    ; VGA TEXT BUFFER OPERATIONS
    ;=========================================================================
    
    ; Clear the entire VGA text buffer first (80x25 = 2000 characters, 4000 bytes)
    mov edi, 0xB8000
    mov ecx, 2000           ; 80*25 characters
    mov eax, 0x0F200F20     ; Two space characters with white-on-black attribute
    rep stosd               ; Clear screen (writes 4 bytes at a time)
    
    ; Print "KL" to VGA text buffer at 0xB8000
    mov edi, 0xB8000
    mov byte [edi], 'K'
    mov byte [edi+1], 0x0F  ; White on black
    mov byte [edi+2], 'L'
    mov byte [edi+3], 0x0F  ; White on black

    ; Save registers
    push ebp
    mov ebp, esp
    push ebx
    push ecx
    push edx
    push esi
    push edi
    
    ; Initialize filesystem structures
    call init_ext4_filesystem
    test eax, eax
    jz .error
    
    ; Search for kernel.bin in root directory
    mov esi, kernel_filename
    call find_file_in_root
    test eax, eax
    jz .file_not_found
    
    ; eax now contains inode number of kernel.bin
    mov ebx, eax  ; Save inode number
    
    ; Get file size from inode
    mov edi, eax
    call get_file_size
    mov ecx, eax  ; Save file size
    
    ; Allocate memory for kernel (at 0x100000 - 1MB mark)
    mov edi, 0x100000
    mov esi, ecx
    call allocate_memory_region
    test eax, eax
    jz .memory_error
    
    ; Read kernel file data from disk
    mov edi, ebx  ; inode number
    mov esi, 0x100000  ; destination address
    mov edx, ecx  ; file size
    call read_file_data
    test eax, eax
    jz .read_error
    
    ; Verify kernel signature
    mov edi, 0x100000
    call verify_kernel_signature
    test eax, eax
    jz .error
    
    ; Prepare kernel environment
    cli
    
    ; Set up stack for kernel
    mov esp, 0x900000
    
    ; Jump to kernel
    jmp 0x100000

.error:
.file_not_found:
.memory_error:
.read_error:
    mov eax, 0
    pop edi
    pop esi
    pop edx
    pop ecx
    pop ebx
    pop ebp
    ret

; Initialize ext4 filesystem
init_ext4_filesystem:
    ; Placeholder implementation
    mov eax, 1  ; Return success
    ret

; Find file in root directory
find_file_in_root:
    ; esi = filename
    ; Placeholder implementation
    mov eax, 1  ; Return fake inode number
    ret

; Get file size from inode
get_file_size:
    ; edi = inode number
    ; Placeholder implementation
    mov eax, 4096  ; Return fake size
    ret

; Allocate memory region
allocate_memory_region:
    ; edi = address, esi = size
    ; Placeholder implementation
    mov eax, 1  ; Return success
    ret

; Read file data
read_file_data:
    ; edi = inode, esi = destination, edx = size
    ; Placeholder implementation
    mov eax, 1  ; Return success
    ret

; Verify kernel signature
verify_kernel_signature:
    ; edi = kernel address
    ; Placeholder implementation
    mov eax, 1  ; Return success
    ret

section .data
kernel_filename: db "kernel.bin", 0
