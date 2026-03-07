//! Interrupt Descriptor Table (IDT) implementation for x86-64
//! =============================================================================
//!
//! The IDT (Interrupt Descriptor Table) is like a phone book for interrupts.
//! When something happens (hardware interrupt, exception, software interrupt),
//! the CPU looks up the handler address in the IDT and jumps to it.
//!
//! What are interrupts and exceptions?
//!
//! **Exceptions (vectors 0-31)**: These are CPU-generated errors
//!   - Divide by zero (vector 0)
//!   - Page fault (vector 14): Invalid memory access
//!   - General protection fault (vector 13): Access violation
//!   - Double fault (vector 8): Exception while handling another exception!
//!   Most exceptions are fatal and cause the kernel to panic.
//!
//! **Hardware Interrupts (IRQs, vectors 32-47)**: External device signals
//!   - Timer (IRQ 0 / vector 32): Fires periodically for timekeeping
//!   - Keyboard (IRQ 1 / vector 33): Key pressed or released
//!   - Hard disk (IRQ 14/15): Disk operation completed
//!   These are expected and we handle them then continue execution.
//!
//! **Software Interrupts (vectors 48-255)**: Triggered by INT instruction
//!   - System calls (INT 0x80 on Linux, or SYSCALL instruction)
//!   - Custom interrupts for IPC, debugging, etc.
//!
//! IDT Structure in 64-bit mode:
//! - Each entry is 16 bytes (double the size of 32-bit!)
//! - 256 entries total = 4096 bytes (one page)
//! - Entry format: handler address, segment selector, flags
//!
//! =============================================================================

mod handlers;

use core::mem::size_of;

/// IDT Entry (16 bytes in 64-bit mode)
/// 
/// Each entry tells the CPU:
/// 1. Where to jump when this interrupt fires (handler address)
/// 2. Which code segment to use (must be kernel code)
/// 3. What type of gate this is (interrupt vs trap)
/// 4. Which privilege level can trigger it
/// 5. Whether to use an alternate stack (IST)
#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    /// Offset bits 0-15
    offset_low: u16,
    /// Code segment selector (must point to valid kernel code segment in GDT)
    selector: u16,
    /// Interrupt Stack Table offset (bits 0-2), rest reserved
    ist: u8,
    /// Type and attributes (gate type, DPL, present bit)
    type_attr: u8,
    /// Offset bits 16-31
    offset_mid: u16,
    /// Offset bits 32-63
    offset_high: u32,
    /// Reserved (must be zero)
    reserved: u32,
}

impl IdtEntry {
    /// Create a new null IDT entry
        /// 
        /// A null entry means this interrupt is not handled.
        /// If it fires, the CPU will triple-fault and reset (not good!)
    pub const fn new() -> Self {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Set the handler function address and configure the entry
        ///
        /// This is the low-level function to configure an IDT entry.
        /// Most code should use set_interrupt_gate() or set_trap_gate() instead.
    ///
    /// # Arguments
    /// * `handler` - Address of the interrupt handler function
    /// * `selector` - GDT code segment selector (typically 0x08 for kernel code)
    /// * `ist` - Interrupt Stack Table offset (0 = don't use IST, 1-7 = IST entry)
    /// * `gate_type` - Gate type (0xE = interrupt gate, 0xF = trap gate)
    /// * `dpl` - Descriptor Privilege Level (0 = kernel only, 3 = user accessible)
    pub fn set_handler(&mut self, handler: u64, selector: u16, ist: u8, gate_type: u8, dpl: u8) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFF_FFFF) as u32;
        self.selector = selector;
        self.ist = ist & 0x7; // Only 3 bits used
        // type_attr = P (1 bit) | DPL (2 bits) | 0 (1 bit) | Gate Type (4 bits)
        // P=1, DPL=dpl, 0, Type=gate_type
        self.type_attr = (1 << 7) | ((dpl & 0x3) << 5) | (gate_type & 0xF);
        self.reserved = 0;
    }

    /// Set as interrupt gate (interrupts disabled on entry)
    pub fn set_interrupt_gate(&mut self, handler: u64, selector: u16, ist: u8, dpl: u8) {
        self.set_handler(handler, selector, ist, 0xE, dpl);
    }

    /// Set as trap gate (interrupts remain enabled on entry)
    pub fn set_trap_gate(&mut self, handler: u64, selector: u16, ist: u8, dpl: u8) {
        self.set_handler(handler, selector, ist, 0xF, dpl);
    }
}

// Compile-time assertion to ensure IdtEntry is 16 bytes
const _: () = assert!(size_of::<IdtEntry>() == 16);

/// IDT Pointer structure (passed to lidt instruction)
#[repr(C, packed)]
pub struct IdtPointer {
    /// Size of IDT in bytes minus 1
    limit: u16,
    /// Base address of IDT
    base: u64,
}

/// The Interrupt Descriptor Table (256 entries)
#[repr(C, align(16))]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    /// Create a new IDT with all entries zeroed
    pub const fn new() -> Self {
        Idt {
            entries: [IdtEntry::new(); 256],
        }
    }

    /// Set a specific IDT entry
    pub fn set_entry(&mut self, index: usize, entry: IdtEntry) {
        if index < 256 {
            self.entries[index] = entry;
        }
    }

    /// Set an interrupt gate for a specific vector
    pub fn set_interrupt_gate(&mut self, vector: usize, handler: u64, selector: u16, ist: u8) {
        if vector < 256 {
            self.entries[vector].set_interrupt_gate(handler, selector, ist, 0);
        }
    }

    /// Set a trap gate for a specific vector
    pub fn set_trap_gate(&mut self, vector: usize, handler: u64, selector: u16, ist: u8) {
        if vector < 256 {
            self.entries[vector].set_trap_gate(handler, selector, ist, 0);
        }
    }

    /// Get the IDT pointer for loading with lidt
    pub fn pointer(&self) -> IdtPointer {
        IdtPointer {
            limit: (size_of::<Idt>() - 1) as u16,
            base: self as *const _ as u64,
        }
    }
}

// External assembly functions
extern "C" {
    fn idt_load(idt_ptr: *const IdtPointer);
    
    // Exception handlers (0-31)
    fn isr0();  // Divide Error
    fn isr1();  // Debug
    fn isr2();  // NMI
    fn isr3();  // Breakpoint
    fn isr4();  // Overflow
    fn isr5();  // Bound Range Exceeded
    fn isr6();  // Invalid Opcode
    fn isr7();  // Device Not Available
    fn isr8();  // Double Fault
    fn isr9();  // Coprocessor Segment Overrun
    fn isr10(); // Invalid TSS
    fn isr11(); // Segment Not Present
    fn isr12(); // Stack-Segment Fault
    fn isr13(); // General Protection Fault
    fn isr14(); // Page Fault
    fn isr15(); // Reserved
    fn isr16(); // x87 FPU Error
    fn isr17(); // Alignment Check
    fn isr18(); // Machine Check
    fn isr19(); // SIMD Floating-Point Exception
    fn isr20(); // Virtualization Exception
    fn isr21(); // Control Protection Exception
    fn isr22(); // Reserved
    fn isr23(); // Reserved
    fn isr24(); // Reserved
    fn isr25(); // Reserved
    fn isr26(); // Reserved
    fn isr27(); // Reserved
    fn isr28(); // Reserved
    fn isr29(); // Reserved
    fn isr30(); // Reserved
    fn isr31(); // Reserved
    
    // IRQ handlers (32-47)
    fn irq0();  // Timer
    fn irq1();  // Keyboard
    fn irq2();  // Cascade
    fn irq3();  // COM2
    fn irq4();  // COM1
    fn irq5();  // LPT2
    fn irq6();  // Floppy
    fn irq7();  // LPT1
    fn irq8();  // RTC
    fn irq9();  // Free
    fn irq10(); // Free
    fn irq11(); // Free
    fn irq12(); // PS/2 Mouse
    fn irq13(); // FPU
    fn irq14(); // Primary ATA
    fn irq15(); // Secondary ATA
}

/// Static IDT instance
static mut IDT: Idt = Idt::new();

/// Static IDT pointer
static mut IDT_POINTER: IdtPointer = IdtPointer { limit: 0, base: 0 };

/// Exception vector numbers
pub mod exceptions {
    pub const DIVIDE_ERROR: usize = 0;
    pub const DEBUG: usize = 1;
    pub const NMI: usize = 2;
    pub const BREAKPOINT: usize = 3;
    pub const OVERFLOW: usize = 4;
    pub const BOUND_RANGE: usize = 5;
    pub const INVALID_OPCODE: usize = 6;
    pub const DEVICE_NOT_AVAILABLE: usize = 7;
    pub const DOUBLE_FAULT: usize = 8;
    pub const INVALID_TSS: usize = 10;
    pub const SEGMENT_NOT_PRESENT: usize = 11;
    pub const STACK_SEGMENT_FAULT: usize = 12;
    pub const GENERAL_PROTECTION: usize = 13;
    pub const PAGE_FAULT: usize = 14;
    pub const FPU_ERROR: usize = 16;
    pub const ALIGNMENT_CHECK: usize = 17;
    pub const MACHINE_CHECK: usize = 18;
    pub const SIMD_EXCEPTION: usize = 19;
    pub const VIRTUALIZATION: usize = 20;
    pub const CONTROL_PROTECTION: usize = 21;
}

/// Initialize the IDT
///
/// Sets up all exception handlers (0-31) and IRQ handlers (32-47)
///
/// # Safety
/// Must be called only once during kernel initialization
#[allow(static_mut_refs)]
pub unsafe fn init() {
    // Kernel code segment selector (from GDT)
    let kernel_cs: u16 = 0x08;

    // Set up exception handlers (vectors 0-31)
    // Use IST entries for critical exceptions to prevent stack corruption
    IDT.set_interrupt_gate(0, isr0 as u64, kernel_cs, 0);      // Divide Error
    IDT.set_trap_gate(1, isr1 as u64, kernel_cs, 0);           // Debug
    IDT.set_interrupt_gate(2, isr2 as u64, kernel_cs, 2);      // NMI (IST2)
    IDT.set_trap_gate(3, isr3 as u64, kernel_cs, 0);           // Breakpoint
    IDT.set_trap_gate(4, isr4 as u64, kernel_cs, 0);           // Overflow
    IDT.set_interrupt_gate(5, isr5 as u64, kernel_cs, 0);      // Bound Range
    IDT.set_interrupt_gate(6, isr6 as u64, kernel_cs, 0);      // Invalid Opcode
    IDT.set_interrupt_gate(7, isr7 as u64, kernel_cs, 0);      // Device Not Available
    IDT.set_interrupt_gate(8, isr8 as u64, kernel_cs, 1);      // Double Fault (IST1)
    IDT.set_interrupt_gate(9, isr9 as u64, kernel_cs, 0);      // Coprocessor Segment Overrun
    IDT.set_interrupt_gate(10, isr10 as u64, kernel_cs, 0);    // Invalid TSS
    IDT.set_interrupt_gate(11, isr11 as u64, kernel_cs, 0);    // Segment Not Present
    IDT.set_interrupt_gate(12, isr12 as u64, kernel_cs, 0);    // Stack-Segment Fault
    IDT.set_interrupt_gate(13, isr13 as u64, kernel_cs, 0);    // General Protection Fault
    // Page Fault (vector 14) uses IST5 – a dedicated emergency stack.
    // When a guard page is hit (stack overflow), the kernel RSP is already
    // invalid.  Without IST the CPU would push the exception frame onto the
    // overflowed stack, causing a #DF before our handler runs.  IST5 ensures
    // the handler always has a clean stack regardless of RSP state.
    IDT.set_interrupt_gate(14, isr14 as u64, kernel_cs, 5);    // Page Fault (IST5 = IST_PAGE_FAULT)
    IDT.set_interrupt_gate(15, isr15 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(16, isr16 as u64, kernel_cs, 0);    // x87 FPU Error
    IDT.set_interrupt_gate(17, isr17 as u64, kernel_cs, 0);    // Alignment Check
    IDT.set_interrupt_gate(18, isr18 as u64, kernel_cs, 3);    // Machine Check (IST3)
    IDT.set_interrupt_gate(19, isr19 as u64, kernel_cs, 0);    // SIMD Exception
    IDT.set_interrupt_gate(20, isr20 as u64, kernel_cs, 0);    // Virtualization
    IDT.set_interrupt_gate(21, isr21 as u64, kernel_cs, 0);    // Control Protection
    IDT.set_interrupt_gate(22, isr22 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(23, isr23 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(24, isr24 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(25, isr25 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(26, isr26 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(27, isr27 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(28, isr28 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(29, isr29 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(30, isr30 as u64, kernel_cs, 0);    // Reserved
    IDT.set_interrupt_gate(31, isr31 as u64, kernel_cs, 0);    // Reserved

    // Set up IRQ handlers (vectors 32-47)
    IDT.set_interrupt_gate(32, irq0 as u64, kernel_cs, 0);     // Timer
    IDT.set_interrupt_gate(33, irq1 as u64, kernel_cs, 0);     // Keyboard
    IDT.set_interrupt_gate(34, irq2 as u64, kernel_cs, 0);     // Cascade
    IDT.set_interrupt_gate(35, irq3 as u64, kernel_cs, 0);     // COM2
    IDT.set_interrupt_gate(36, irq4 as u64, kernel_cs, 0);     // COM1
    IDT.set_interrupt_gate(37, irq5 as u64, kernel_cs, 0);     // LPT2
    IDT.set_interrupt_gate(38, irq6 as u64, kernel_cs, 0);     // Floppy
    IDT.set_interrupt_gate(39, irq7 as u64, kernel_cs, 0);     // LPT1
    IDT.set_interrupt_gate(40, irq8 as u64, kernel_cs, 0);     // RTC
    IDT.set_interrupt_gate(41, irq9 as u64, kernel_cs, 0);     // Free
    IDT.set_interrupt_gate(42, irq10 as u64, kernel_cs, 0);    // Free
    IDT.set_interrupt_gate(43, irq11 as u64, kernel_cs, 0);    // Free
    IDT.set_interrupt_gate(44, irq12 as u64, kernel_cs, 0);    // PS/2 Mouse
    IDT.set_interrupt_gate(45, irq13 as u64, kernel_cs, 0);    // FPU
    IDT.set_interrupt_gate(46, irq14 as u64, kernel_cs, 0);    // Primary ATA
    IDT.set_interrupt_gate(47, irq15 as u64, kernel_cs, 0);    // Secondary ATA

    // Create IDT pointer
    IDT_POINTER = IDT.pointer();

    // Load IDT into CPU
    idt_load(&raw const IDT_POINTER);
}
