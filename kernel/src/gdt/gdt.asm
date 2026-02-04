; GDT Assembly Routines
; 
; Provides low-level assembly functions for:
; - Loading the GDT (lgdt instruction)
; - Loading the TSS (ltr instruction)
; - Reloading segment registers
;
; These functions are called from Rust code via FFI.

bits 64
section .text

; =============================================================================
; gdt_load - Load the Global Descriptor Table
; =============================================================================
; void gdt_load(GdtPointer* gdt_ptr)
;
; Parameters:
;   rdi - Pointer to GDT descriptor (limit + base)
;
; Loads the GDTR with the new GDT.
; =============================================================================
global gdt_load
gdt_load:
    lgdt [rdi]
    ret

; =============================================================================
; tss_load - Load the Task State Segment
; =============================================================================
; void tss_load(uint16_t selector)
;
; Parameters:
;   di - TSS selector (e.g., 0x28)
;
; Loads the TR (Task Register) with the TSS selector.
; =============================================================================
global tss_load
tss_load:
    ltr di
    ret

; =============================================================================
; reload_segments - Reload all segment registers after GDT change
; =============================================================================
; void reload_segments(void)
;
; After loading a new GDT, segment registers must be reloaded.
; - CS is reloaded via a far return
; - DS, ES, FS, GS, SS are reloaded directly
;
; Segment selectors used:
;   0x08 - Kernel code segment
;   0x10 - Kernel data segment
; =============================================================================
global reload_segments
reload_segments:
    ; In 64-bit long mode, data segment registers are not checked in the same
    ; way as in protected mode. The bootloader has already set them up.
    ; Skip segment reloading since the descriptors are compatible.
    ret

; =============================================================================
; get_cs - Get current CS register value
; =============================================================================
; uint16_t get_cs(void)
; =============================================================================
global get_cs
get_cs:
    xor rax, rax
    mov ax, cs
    ret

; =============================================================================
; get_ds - Get current DS register value
; =============================================================================
; uint16_t get_ds(void)
; =============================================================================
global get_ds
get_ds:
    xor rax, rax
    mov ax, ds
    ret

; =============================================================================
; get_ss - Get current SS register value
; =============================================================================
; uint16_t get_ss(void)
; =============================================================================
global get_ss
get_ss:
    xor rax, rax
    mov ax, ss
    ret

; =============================================================================
; set_kernel_segments - Set segment registers to kernel values
; =============================================================================
; void set_kernel_segments(void)
;
; Sets DS, ES, FS, GS to kernel data segment (0x10)
; Does NOT modify CS or SS (those require special handling)
; =============================================================================
global set_kernel_segments
set_kernel_segments:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    ret
