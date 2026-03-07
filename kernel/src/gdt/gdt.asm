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
; After loading a new GDT with lgdt, the CPU's segment shadow registers (the
; hidden descriptor cache for CS, DS, etc.) still hold values from the
; PREVIOUS GDT.  The kernel's GDT only defines descriptors at indices 0–6
; (bytes 0x00–0x37).  UEFI firmware commonly hands control with CS=0x38
; (its own 64-bit code segment), which falls one slot beyond the kernel GDT
; limit.  Code continues to execute fine because the shadow cache for CS is
; valid, but ANY instruction that forces the CPU to re-validate CS against the
; GDT (e.g. iretq restoring a saved CS=0x38 after an interrupt) will raise
; #GP(0x38) — the exact crash we see after the first timer IRQ fires.
;
; Fix: perform an explicit far return (lretq) which atomically replaces both
; RIP and CS.  This forces the CPU to load the new 0x08 descriptor from the
; kernel GDT and update the shadow register.  Data/stack segments are then
; reloaded with a simple mov.
;
; Segment selectors used:
;   0x08 - Kernel code segment (CS)
;   0x10 - Kernel data segment (DS, ES, FS, GS, SS)
; =============================================================================
global reload_segments
reload_segments:
    ; Jump over the landing pad so the far-return has a backward (already-seen)
    ; label to reference — NASM's multi-pass optimizer can't handle a forward
    ; RIP-relative reference as the target of `o64 retf`.
    jmp .do_reload

.cs_reloaded:
    ; -----------------------------------------------------------------------
    ; We arrive here via `o64 retf` with CS freshly loaded to 0x08.
    ; Now update the data/stack segments so they also reflect the kernel GDT
    ; rather than whatever the UEFI firmware left behind.
    ; -----------------------------------------------------------------------
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    ret

.do_reload:
    ; -----------------------------------------------------------------------
    ; Reload CS via a 64-bit far return (o64 retf).
    ;
    ; `o64 retf` pops:  [RSP+0] → RIP,  [RSP+8] → CS
    ;
    ; We push CS (0x08) first (lands at higher address), then RIP (.cs_reloaded
    ; — now a backward reference, always safe in NASM) second (lower address).
    ; The far return atomically loads both, forcing the CPU to re-validate CS
    ; against the kernel's GDT and update the segment shadow register.
    ; -----------------------------------------------------------------------
    lea  rax, [rel .cs_reloaded]   ; RIP to resume at (backward ref → safe)
    push qword 0x08                ; New CS selector  (popped second by o64 retf)
    push rax                       ; New RIP          (popped first  by o64 retf)
    o64 retf                       ; 64-bit far return: atomically loads RIP + CS

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
