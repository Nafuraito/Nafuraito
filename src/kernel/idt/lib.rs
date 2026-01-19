//! Nafuraito Kernel - IDT Library
//!
//! Interrupt Descriptor Table (IDT) implementation for x86-64.
//! The IDT is used to handle interrupts and exceptions on x86-64 processors.

#![crate_type = "staticlib"]
#![crate_name = "idt"]
#![no_std]
#![no_main]
#![allow(unused)]

use core::mem::size_of;
use core::panic::PanicInfo;

/// Panic handler - halts the CPU
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe { core::arch::asm!("cli; hlt"); }
    }
}

// =============================================================================
// IDT Entry Structure
// =============================================================================

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
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

    pub fn set_handler(&mut self, handler: u64, selector: u16, ist: u8, gate_type: u8, dpl: u8) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFF_FFFF) as u32;
        self.selector = selector;
        self.ist = ist & 0x7;
        self.type_attr = (1 << 7) | ((dpl & 0x3) << 5) | (gate_type & 0xF);
        self.reserved = 0;
    }

    pub fn set_interrupt_gate(&mut self, handler: u64, selector: u16, ist: u8, dpl: u8) {
        self.set_handler(handler, selector, ist, 0xE, dpl);
    }

    pub fn set_trap_gate(&mut self, handler: u64, selector: u16, ist: u8, dpl: u8) {
        self.set_handler(handler, selector, ist, 0xF, dpl);
    }
}

const _: () = assert!(size_of::<IdtEntry>() == 16);

// =============================================================================
// IDT Pointer Structure
// =============================================================================

#[repr(C, packed)]
pub struct IdtPointer {
    limit: u16,
    base: u64,
}

// =============================================================================
// IDT Structure
// =============================================================================

#[repr(C, align(16))]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    pub const fn new() -> Self {
        Idt {
            entries: [IdtEntry::new(); 256],
        }
    }

    pub fn set_entry(&mut self, index: usize, entry: IdtEntry) {
        if index < 256 {
            self.entries[index] = entry;
        }
    }

    pub fn set_interrupt_gate(&mut self, vector: usize, handler: u64, selector: u16, ist: u8) {
        if vector < 256 {
            self.entries[vector].set_interrupt_gate(handler, selector, ist, 0);
        }
    }

    pub fn set_trap_gate(&mut self, vector: usize, handler: u64, selector: u16, ist: u8) {
        if vector < 256 {
            self.entries[vector].set_trap_gate(handler, selector, ist, 0);
        }
    }

    pub fn pointer(&self) -> IdtPointer {
        IdtPointer {
            limit: (size_of::<Idt>() - 1) as u16,
            base: self as *const _ as u64,
        }
    }
}

// =============================================================================
// External Assembly Functions and ISR/IRQ handlers
// =============================================================================

extern "C" {
    fn idt_load(idt_ptr: *const IdtPointer);
    
    fn isr0();
    fn isr1();
    fn isr2();
    fn isr3();
    fn isr4();
    fn isr5();
    fn isr6();
    fn isr7();
    fn isr8();
    fn isr9();
    fn isr10();
    fn isr11();
    fn isr12();
    fn isr13();
    fn isr14();
    fn isr15();
    fn isr16();
    fn isr17();
    fn isr18();
    fn isr19();
    fn isr20();
    fn isr21();
    fn isr22();
    fn isr23();
    fn isr24();
    fn isr25();
    fn isr26();
    fn isr27();
    fn isr28();
    fn isr29();
    fn isr30();
    fn isr31();
    
    fn irq0();
    fn irq1();
    fn irq2();
    fn irq3();
    fn irq4();
    fn irq5();
    fn irq6();
    fn irq7();
    fn irq8();
    fn irq9();
    fn irq10();
    fn irq11();
    fn irq12();
    fn irq13();
    fn irq14();
    fn irq15();
}

// =============================================================================
// Global State
// =============================================================================

static mut IDT: Idt = Idt::new();
static mut IDT_POINTER: IdtPointer = IdtPointer { limit: 0, base: 0 };

// =============================================================================
// Exception Vector Numbers
// =============================================================================

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

// =============================================================================
// VGA Print Function (from C)
// =============================================================================

extern "C" {
    fn writestring_vga(data: *const i8);
}

#[allow(static_mut_refs)]
fn print(s: &str) {
    unsafe {
        static mut PRINT_BUFFER: [u8; 256] = [0; 256];
        
        let bytes = s.as_bytes();
        let len = bytes.len().min(255);
        
        let src = bytes.as_ptr();
        let dst = (&raw mut PRINT_BUFFER).cast::<u8>();
        let mut i = 0;
        while i < len {
            core::ptr::write_volatile(dst.add(i), core::ptr::read_volatile(src.add(i)));
            i += 1;
        }
        core::ptr::write_volatile(dst.add(len), 0);
        
        writestring_vga((&raw const PRINT_BUFFER).cast::<i8>());
    }
}

#[allow(static_mut_refs)]
fn print_hex(num: u64) {
    unsafe {
        static mut HEX_BUFFER: [u8; 19] = [0; 19];
        const HEX_CHARS: [u8; 16] = *b"0123456789ABCDEF";
        
        HEX_BUFFER[0] = b'0';
        HEX_BUFFER[1] = b'x';
        
        for i in 0..16 {
            let digit = ((num >> (60 - i * 4)) & 0xF) as usize;
            HEX_BUFFER[2 + i] = HEX_CHARS[digit];
        }
        HEX_BUFFER[18] = 0;
        
        writestring_vga((&raw const HEX_BUFFER).cast::<i8>());
    }
}

// =============================================================================
// Exception and IRQ Handlers
// =============================================================================

#[no_mangle]
pub extern "C" fn exception_handler(vector: u64, error_code: u64) {
    let exception_name = match vector {
        0 => "Divide Error",
        1 => "Debug",
        2 => "Non-Maskable Interrupt",
        3 => "Breakpoint",
        4 => "Overflow",
        5 => "Bound Range Exceeded",
        6 => "Invalid Opcode",
        7 => "Device Not Available",
        8 => "Double Fault",
        9 => "Coprocessor Segment Overrun",
        10 => "Invalid TSS",
        11 => "Segment Not Present",
        12 => "Stack-Segment Fault",
        13 => "General Protection Fault",
        14 => "Page Fault",
        16 => "x87 FPU Error",
        17 => "Alignment Check",
        18 => "Machine Check",
        19 => "SIMD Floating-Point Exception",
        20 => "Virtualization Exception",
        21 => "Control Protection Exception",
        _ => "Unknown Exception",
    };

    print("EXCEPTION: ");
    print(exception_name);
    print(" (vector=");
    print_hex(vector);
    print(", error=");
    print_hex(error_code);
    print(")\n\0");

    unsafe {
        loop {
            core::arch::asm!("hlt");
        }
    }
}

#[no_mangle]
pub extern "C" fn irq_handler(irq: u64) {
    match irq {
        32 => {
            static mut TICK_COUNT: u64 = 0;
            unsafe {
                TICK_COUNT += 1;
                if TICK_COUNT % 100 == 0 {
                    print("Timer tick: ");
                    print_hex(TICK_COUNT);
                    print("\n\0");
                }
            }
        }
        33 => {
            const KEYBOARD_DATA_PORT: u16 = 0x60;
            unsafe {
                let scancode: u8;
                core::arch::asm!(
                    "in al, dx",
                    in("dx") KEYBOARD_DATA_PORT,
                    out("al") scancode,
                    options(nomem, nostack, preserves_flags)
                );
                print("Keyboard scancode: ");
                print_hex(scancode as u64);
                print("\n\0");
            }
        }
        _ => {
            print("IRQ received: ");
            print_hex(irq);
            print("\n\0");
        }
    }

    send_eoi(irq);
}

fn send_eoi(irq: u64) {
    const PIC1_COMMAND: u16 = 0x20;
    const PIC2_COMMAND: u16 = 0xA0;
    const PIC_EOI: u8 = 0x20;

    unsafe {
        if irq >= 40 {
            outb(PIC2_COMMAND, PIC_EOI);
        }
        outb(PIC1_COMMAND, PIC_EOI);
    }
}

#[inline]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

// =============================================================================
// Public API
// =============================================================================

#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe fn init() {
    let kernel_cs: u16 = 0x08;

    IDT.set_interrupt_gate(0, isr0 as u64, kernel_cs, 0);
    IDT.set_trap_gate(1, isr1 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(2, isr2 as u64, kernel_cs, 2);
    IDT.set_trap_gate(3, isr3 as u64, kernel_cs, 0);
    IDT.set_trap_gate(4, isr4 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(5, isr5 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(6, isr6 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(7, isr7 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(8, isr8 as u64, kernel_cs, 1);
    IDT.set_interrupt_gate(9, isr9 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(10, isr10 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(11, isr11 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(12, isr12 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(13, isr13 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(14, isr14 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(15, isr15 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(16, isr16 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(17, isr17 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(18, isr18 as u64, kernel_cs, 3);
    IDT.set_interrupt_gate(19, isr19 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(20, isr20 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(21, isr21 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(22, isr22 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(23, isr23 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(24, isr24 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(25, isr25 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(26, isr26 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(27, isr27 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(28, isr28 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(29, isr29 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(30, isr30 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(31, isr31 as u64, kernel_cs, 0);

    IDT.set_interrupt_gate(32, irq0 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(33, irq1 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(34, irq2 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(35, irq3 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(36, irq4 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(37, irq5 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(38, irq6 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(39, irq7 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(40, irq8 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(41, irq9 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(42, irq10 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(43, irq11 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(44, irq12 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(45, irq13 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(46, irq14 as u64, kernel_cs, 0);
    IDT.set_interrupt_gate(47, irq15 as u64, kernel_cs, 0);

    IDT_POINTER = IDT.pointer();
    idt_load(&raw const IDT_POINTER);
}
