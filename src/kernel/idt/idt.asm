; IDT Assembly Routines
; Provides low-level interrupt handling and IDT loading

section .text
bits 64

; Load IDT
; void idt_load(const IdtPointer* idt_ptr);
global idt_load
idt_load:
    lidt [rdi]          ; Load IDT from pointer in RDI
    ret

; Common exception handler stub
; Saves registers, calls Rust handler, restores registers
extern exception_handler
isr_common:
    ; Save all registers
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    ; Call Rust exception handler
    ; RDI = interrupt vector (already pushed by ISR stub)
    ; RSI = error code (already pushed by ISR stub or 0)
    mov rdi, [rsp + 15*8]  ; Get vector number
    mov rsi, [rsp + 16*8]  ; Get error code
    call exception_handler

    ; Restore all registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    ; Remove error code and vector number from stack
    add rsp, 16

    ; Return from interrupt
    iretq

; Common IRQ handler stub
extern irq_handler
irq_common:
    ; Save all registers
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    ; Call Rust IRQ handler
    mov rdi, [rsp + 15*8]  ; Get IRQ number
    call irq_handler

    ; Restore all registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    ; Remove IRQ number from stack
    add rsp, 8

    ; Return from interrupt
    iretq

; Macro to create ISR stub without error code
%macro ISR_NOERRCODE 1
global isr%1
isr%1:
    push qword 0        ; Dummy error code
    push qword %1       ; Push vector number
    jmp isr_common
%endmacro

; Macro to create ISR stub with error code
%macro ISR_ERRCODE 1
global isr%1
isr%1:
    ; CPU pushes error code automatically
    push qword %1       ; Push vector number
    jmp isr_common
%endmacro

; Macro to create IRQ stub
%macro IRQ 2
global irq%1
irq%1:
    push qword %2       ; Push IRQ number
    jmp irq_common
%endmacro

; CPU Exceptions (0-31)
ISR_NOERRCODE 0     ; Divide Error
ISR_NOERRCODE 1     ; Debug
ISR_NOERRCODE 2     ; NMI
ISR_NOERRCODE 3     ; Breakpoint
ISR_NOERRCODE 4     ; Overflow
ISR_NOERRCODE 5     ; Bound Range Exceeded
ISR_NOERRCODE 6     ; Invalid Opcode
ISR_NOERRCODE 7     ; Device Not Available
ISR_ERRCODE   8     ; Double Fault (has error code)
ISR_NOERRCODE 9     ; Coprocessor Segment Overrun
ISR_ERRCODE   10    ; Invalid TSS (has error code)
ISR_ERRCODE   11    ; Segment Not Present (has error code)
ISR_ERRCODE   12    ; Stack-Segment Fault (has error code)
ISR_ERRCODE   13    ; General Protection Fault (has error code)
ISR_ERRCODE   14    ; Page Fault (has error code)
ISR_NOERRCODE 15    ; Reserved
ISR_NOERRCODE 16    ; x87 FPU Error
ISR_ERRCODE   17    ; Alignment Check (has error code)
ISR_NOERRCODE 18    ; Machine Check
ISR_NOERRCODE 19    ; SIMD Floating-Point Exception
ISR_NOERRCODE 20    ; Virtualization Exception
ISR_ERRCODE   21    ; Control Protection Exception (has error code)
ISR_NOERRCODE 22    ; Reserved
ISR_NOERRCODE 23    ; Reserved
ISR_NOERRCODE 24    ; Reserved
ISR_NOERRCODE 25    ; Reserved
ISR_NOERRCODE 26    ; Reserved
ISR_NOERRCODE 27    ; Reserved
ISR_NOERRCODE 28    ; Reserved
ISR_NOERRCODE 29    ; Reserved
ISR_NOERRCODE 30    ; Reserved
ISR_NOERRCODE 31    ; Reserved

; IRQs (32-47)
IRQ 0, 32           ; Timer
IRQ 1, 33           ; Keyboard
IRQ 2, 34           ; Cascade
IRQ 3, 35           ; COM2
IRQ 4, 36           ; COM1
IRQ 5, 37           ; LPT2
IRQ 6, 38           ; Floppy
IRQ 7, 39           ; LPT1
IRQ 8, 40           ; RTC
IRQ 9, 41           ; Free
IRQ 10, 42          ; Free
IRQ 11, 43          ; Free
IRQ 12, 44          ; PS/2 Mouse
IRQ 13, 45          ; FPU
IRQ 14, 46          ; Primary ATA
IRQ 15, 47          ; Secondary ATA
