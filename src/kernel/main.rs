//! Nafuraito Kernel Entry Point
//! 
//! This is the main entry point for the Nafuraito operating system kernel.
//! It initializes all subsystems and halts the CPU.

#![crate_type = "staticlib"]
#![no_std]
#![no_main]
#![allow(unused)]

use core::panic::PanicInfo;

/// Panic handler for the kernel
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}

// =============================================================================
// Required Memory Functions for no_std
// =============================================================================

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = c as u8;
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return (a as i32) - (b as i32);
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if (dest as usize) < (src as usize) {
        memcpy(dest, src, n)
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
        dest
    }
}

// =============================================================================
// External Library Functions (linked from separate libraries)
// =============================================================================

// Video library (libvideo.a)
extern "C" {
    fn set_framebuffer_info(addr: u32, pitch: u32, width: u32, height: u32, bpp: u32);
    fn initialize_vga_graphics();
    fn writestring_vga(data: *const i8);
}

// GDT library (libgdt.a) - calls the Rust init through asm
extern "C" {
    fn gdt_load(gdt_ptr: *const u8);
    fn reload_segments();
    fn tss_load(selector: u16);
}

// IDT library (libidt.a) - calls the Rust init through asm
extern "C" {
    fn idt_load(idt_ptr: *const u8);
}

// =============================================================================
// Library Init Functions (from Rust libraries)
// =============================================================================

extern "Rust" {
    // From libgdt.a
    fn init() -> ();
}

// GDT Constants (must match libgdt.a)
const KERNEL_CODE_SELECTOR: u16 = 0x08;
const KERNEL_DATA_SELECTOR: u16 = 0x10;
const USER_DATA_SELECTOR: u16 = 0x18 | 3;
const USER_CODE_SELECTOR: u16 = 0x20 | 3;
const TSS_SELECTOR: u16 = 0x28;

// =============================================================================
// Helper Functions
// =============================================================================

/// Print a null-terminated string via VGA
unsafe fn print(s: &str) {
    writestring_vga(s.as_ptr() as *const i8);
}

/// Print a single byte as hex
fn print_hex_byte(b: u8) {
    const HEX_CHARS: [u8; 16] = *b"0123456789ABCDEF";
    let hi = (b >> 4) as usize;
    let lo = (b & 0x0F) as usize;
    unsafe {
        let s: [u8; 3] = [HEX_CHARS[hi], HEX_CHARS[lo], 0];
        print(core::str::from_utf8_unchecked(&s));
    }
}

// =============================================================================
// PIC Functions (inline to avoid library call issues)
// =============================================================================

const PIC1: u16 = 0x20;
const PIC2: u16 = 0xA0;
const PIC1_COMMAND: u16 = PIC1;
const PIC1_DATA: u16 = PIC1 + 1;
const PIC2_COMMAND: u16 = PIC2;
const PIC2_DATA: u16 = PIC2 + 1;
const ICW1_ICW4: u8 = 0x01;
const ICW1_INIT: u8 = 0x10;
const ICW4_8086: u8 = 0x01;
const CASCADE_IRQ: u8 = 2;

#[inline]
unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}

#[inline]
unsafe fn io_wait() {
    outb(0x80, 0);
}

unsafe fn pic_remap(offset1: u8, offset2: u8) {
    outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
    io_wait();
    outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
    io_wait();
    outb(PIC1_DATA, offset1);
    io_wait();
    outb(PIC2_DATA, offset2);
    io_wait();
    outb(PIC1_DATA, 1 << CASCADE_IRQ);
    io_wait();
    outb(PIC2_DATA, CASCADE_IRQ);
    io_wait();
    outb(PIC1_DATA, ICW4_8086);
    io_wait();
    outb(PIC2_DATA, ICW4_8086);
    io_wait();
    outb(PIC1_DATA, 0);
    outb(PIC2_DATA, 0);
}

// =============================================================================
// GDT/TSS Structures and Init
// =============================================================================

use core::mem::size_of;

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct TaskStateSegment {
    reserved0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: u64,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    reserved2: u64,
    reserved3: u16,
    iopb_offset: u16,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    flags_limit_high: u8,
    base_high: u8,
}

impl GdtEntry {
    const fn null() -> Self {
        GdtEntry { limit_low: 0, base_low: 0, base_middle: 0, access: 0, flags_limit_high: 0, base_high: 0 }
    }
    
    const fn new(base: u32, limit: u32, access: u8, flags: u8) -> Self {
        GdtEntry {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_middle: ((base >> 16) & 0xFF) as u8,
            access,
            flags_limit_high: ((limit >> 16) & 0x0F) as u8 | ((flags & 0x0F) << 4),
            base_high: ((base >> 24) & 0xFF) as u8,
        }
    }
    
    const fn kernel_code() -> Self { GdtEntry::new(0, 0xFFFFF, 0x9A, 0x0A) }
    const fn kernel_data() -> Self { GdtEntry::new(0, 0xFFFFF, 0x92, 0x0C) }
    const fn user_code() -> Self { GdtEntry::new(0, 0xFFFFF, 0xFA, 0x0A) }
    const fn user_data() -> Self { GdtEntry::new(0, 0xFFFFF, 0xF2, 0x0C) }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct TssEntry {
    low: GdtEntry,
    base_upper: u32,
    reserved: u32,
}

impl TssEntry {
    const fn null() -> Self {
        TssEntry { low: GdtEntry::null(), base_upper: 0, reserved: 0 }
    }
    
    fn new(tss_addr: u64, tss_size: u32) -> Self {
        let base = tss_addr;
        let limit = tss_size - 1;
        TssEntry {
            low: GdtEntry {
                limit_low: (limit & 0xFFFF) as u16,
                base_low: (base & 0xFFFF) as u16,
                base_middle: ((base >> 16) & 0xFF) as u8,
                access: 0x89,
                flags_limit_high: ((limit >> 16) & 0x0F) as u8,
                base_high: ((base >> 24) & 0xFF) as u8,
            },
            base_upper: ((base >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }
}

#[repr(C, packed)]
struct Gdt {
    null: GdtEntry,
    kernel_code: GdtEntry,
    kernel_data: GdtEntry,
    user_data: GdtEntry,
    user_code: GdtEntry,
    tss: TssEntry,
}

#[repr(C, packed)]
struct GdtPointer {
    limit: u16,
    base: u64,
}

static mut GDT: Gdt = Gdt {
    null: GdtEntry::null(),
    kernel_code: GdtEntry::kernel_code(),
    kernel_data: GdtEntry::kernel_data(),
    user_data: GdtEntry::user_data(),
    user_code: GdtEntry::user_code(),
    tss: TssEntry::null(),
};

static mut TSS: TaskStateSegment = TaskStateSegment {
    reserved0: 0, rsp0: 0, rsp1: 0, rsp2: 0, reserved1: 0,
    ist1: 0, ist2: 0, ist3: 0, ist4: 0, ist5: 0, ist6: 0, ist7: 0,
    reserved2: 0, reserved3: 0, iopb_offset: 104,
};

static mut GDT_POINTER: GdtPointer = GdtPointer { limit: 0, base: 0 };

#[allow(static_mut_refs)]
unsafe fn gdt_init() {
    TSS.rsp0 = 0x900000;
    TSS.ist1 = 0x8F0000;
    TSS.ist2 = 0x8E0000;
    TSS.ist3 = 0x8D0000;
    TSS.iopb_offset = size_of::<TaskStateSegment>() as u16;
    
    let tss_addr = &raw const TSS as u64;
    GDT.tss = TssEntry::new(tss_addr, size_of::<TaskStateSegment>() as u32);
    
    GDT_POINTER.limit = (size_of::<Gdt>() - 1) as u16;
    GDT_POINTER.base = &raw const GDT as u64;
    
    gdt_load(&raw const GDT_POINTER as *const u8);
    reload_segments();
    tss_load(TSS_SELECTOR);
}

// =============================================================================
// IDT Structures and Init
// =============================================================================

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn new() -> Self {
        IdtEntry { offset_low: 0, selector: 0, ist: 0, type_attr: 0, offset_mid: 0, offset_high: 0, reserved: 0 }
    }
    
    fn set_gate(&mut self, handler: u64, selector: u16, ist: u8, gate_type: u8, dpl: u8) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFF_FFFF) as u32;
        self.selector = selector;
        self.ist = ist & 0x7;
        self.type_attr = (1 << 7) | ((dpl & 0x3) << 5) | (gate_type & 0xF);
        self.reserved = 0;
    }
}

#[repr(C, align(16))]
struct Idt {
    entries: [IdtEntry; 256],
}

#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

static mut IDT: Idt = Idt { entries: [IdtEntry::new(); 256] };
static mut IDT_POINTER: IdtPointer = IdtPointer { limit: 0, base: 0 };

// External ISR/IRQ handlers from idt.asm
extern "C" {
    fn isr0(); fn isr1(); fn isr2(); fn isr3(); fn isr4(); fn isr5(); fn isr6(); fn isr7();
    fn isr8(); fn isr9(); fn isr10(); fn isr11(); fn isr12(); fn isr13(); fn isr14(); fn isr15();
    fn isr16(); fn isr17(); fn isr18(); fn isr19(); fn isr20(); fn isr21(); fn isr22(); fn isr23();
    fn isr24(); fn isr25(); fn isr26(); fn isr27(); fn isr28(); fn isr29(); fn isr30(); fn isr31();
    fn irq0(); fn irq1(); fn irq2(); fn irq3(); fn irq4(); fn irq5(); fn irq6(); fn irq7();
    fn irq8(); fn irq9(); fn irq10(); fn irq11(); fn irq12(); fn irq13(); fn irq14(); fn irq15();
}

#[allow(static_mut_refs)]
unsafe fn idt_init() {
    let cs = KERNEL_CODE_SELECTOR;
    
    // Exception handlers
    IDT.entries[0].set_gate(isr0 as u64, cs, 0, 0xE, 0);
    IDT.entries[1].set_gate(isr1 as u64, cs, 0, 0xF, 0);
    IDT.entries[2].set_gate(isr2 as u64, cs, 2, 0xE, 0);
    IDT.entries[3].set_gate(isr3 as u64, cs, 0, 0xF, 0);
    IDT.entries[4].set_gate(isr4 as u64, cs, 0, 0xF, 0);
    IDT.entries[5].set_gate(isr5 as u64, cs, 0, 0xE, 0);
    IDT.entries[6].set_gate(isr6 as u64, cs, 0, 0xE, 0);
    IDT.entries[7].set_gate(isr7 as u64, cs, 0, 0xE, 0);
    IDT.entries[8].set_gate(isr8 as u64, cs, 1, 0xE, 0);
    IDT.entries[9].set_gate(isr9 as u64, cs, 0, 0xE, 0);
    IDT.entries[10].set_gate(isr10 as u64, cs, 0, 0xE, 0);
    IDT.entries[11].set_gate(isr11 as u64, cs, 0, 0xE, 0);
    IDT.entries[12].set_gate(isr12 as u64, cs, 0, 0xE, 0);
    IDT.entries[13].set_gate(isr13 as u64, cs, 0, 0xE, 0);
    IDT.entries[14].set_gate(isr14 as u64, cs, 0, 0xE, 0);
    IDT.entries[15].set_gate(isr15 as u64, cs, 0, 0xE, 0);
    IDT.entries[16].set_gate(isr16 as u64, cs, 0, 0xE, 0);
    IDT.entries[17].set_gate(isr17 as u64, cs, 0, 0xE, 0);
    IDT.entries[18].set_gate(isr18 as u64, cs, 3, 0xE, 0);
    IDT.entries[19].set_gate(isr19 as u64, cs, 0, 0xE, 0);
    IDT.entries[20].set_gate(isr20 as u64, cs, 0, 0xE, 0);
    IDT.entries[21].set_gate(isr21 as u64, cs, 0, 0xE, 0);
    for i in 22..32 {
        let isrs: [unsafe extern "C" fn(); 10] = [isr22, isr23, isr24, isr25, isr26, isr27, isr28, isr29, isr30, isr31];
        IDT.entries[i].set_gate(isrs[i - 22] as u64, cs, 0, 0xE, 0);
    }
    
    // IRQ handlers (32-47)
    let irqs: [unsafe extern "C" fn(); 16] = [irq0, irq1, irq2, irq3, irq4, irq5, irq6, irq7, irq8, irq9, irq10, irq11, irq12, irq13, irq14, irq15];
    for i in 0..16 {
        IDT.entries[32 + i].set_gate(irqs[i] as u64, cs, 0, 0xE, 0);
    }
    
    IDT_POINTER.limit = (size_of::<Idt>() - 1) as u16;
    IDT_POINTER.base = &raw const IDT as u64;
    idt_load(&raw const IDT_POINTER as *const u8);
}

// =============================================================================
// Exception/IRQ Handlers (called from assembly)
// =============================================================================

#[allow(static_mut_refs)]
fn handler_print(s: &str) {
    unsafe {
        static mut PRINT_BUFFER: [u8; 256] = [0; 256];
        let bytes = s.as_bytes();
        let len = bytes.len().min(255);
        for i in 0..len {
            PRINT_BUFFER[i] = bytes[i];
        }
        PRINT_BUFFER[len] = 0;
        writestring_vga(PRINT_BUFFER.as_ptr() as *const i8);
    }
}

#[allow(static_mut_refs)]
fn handler_print_hex(num: u64) {
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
        writestring_vga(HEX_BUFFER.as_ptr() as *const i8);
    }
}

#[no_mangle]
pub extern "C" fn exception_handler(vector: u64, error_code: u64) {
    let exception_name = match vector {
        0 => "Divide Error", 1 => "Debug", 2 => "NMI", 3 => "Breakpoint",
        4 => "Overflow", 5 => "Bound Range", 6 => "Invalid Opcode", 7 => "Device Not Available",
        8 => "Double Fault", 10 => "Invalid TSS", 11 => "Segment Not Present",
        12 => "Stack Fault", 13 => "General Protection Fault", 14 => "Page Fault",
        16 => "FPU Error", 17 => "Alignment Check", 18 => "Machine Check",
        19 => "SIMD Exception", 20 => "Virtualization", 21 => "Control Protection",
        _ => "Unknown",
    };
    handler_print("EXCEPTION: ");
    handler_print(exception_name);
    handler_print(" (vector=");
    handler_print_hex(vector);
    handler_print(", error=");
    handler_print_hex(error_code);
    handler_print(")\n\0");
    unsafe { loop { core::arch::asm!("hlt"); } }
}

#[no_mangle]
pub extern "C" fn irq_handler(irq: u64) {
    match irq {
        32 => {
            static mut TICK: u64 = 0;
            unsafe {
                TICK += 1;
                if TICK % 100 == 0 {
                    handler_print("Timer: ");
                    handler_print_hex(TICK);
                    handler_print("\n\0");
                }
            }
        }
        33 => {
            unsafe {
                let scancode: u8;
                core::arch::asm!("in al, dx", in("dx") 0x60u16, out("al") scancode);
                handler_print("Key: ");
                handler_print_hex(scancode as u64);
                handler_print("\n\0");
            }
        }
        _ => {
            handler_print("IRQ: ");
            handler_print_hex(irq);
            handler_print("\n\0");
        }
    }
    
    // Send EOI
    const PIC_EOI: u8 = 0x20;
    unsafe {
        if irq >= 40 { outb(0xA0, PIC_EOI); }
        outb(0x20, PIC_EOI);
    }
}

// =============================================================================
// Kernel Entry Point
// =============================================================================

#[no_mangle]
pub extern "C" fn enter(
    fb_addr: u32,
    pitch: u32,
    width: u32,
    height: u32,
    bpp: u32,
    memory_map_addr: u64
) -> ! {
    unsafe {
        // Set up framebuffer
        if fb_addr != 0 {
            set_framebuffer_info(fb_addr, pitch, width, height, bpp);
        }
        
        initialize_vga_graphics();
        print("Welcome to Nafuraito!\n\0");
        
        // Initialize GDT and TSS
        print("\nInitializing GDT and TSS... \0");
        gdt_init();
        print("OK\n\0");
        
        // Initialize IDT
        print("Initializing IDT... \0");
        idt_init();
        print("OK\n\0");

        // Init PIC
        print("Initializing PIC... \0");
        pic_remap(0x20, 0x28);
        print("OK\n\0");

        // Enable Interrupts
        core::arch::asm!("sti");
        print("Interrupts enabled\n\0");
        
        // Print GDT info
        print("  Kernel Code Selector: 0x\0");
        print_hex_byte(KERNEL_CODE_SELECTOR as u8);
        print("\n  Kernel Data Selector: 0x\0");
        print_hex_byte(KERNEL_DATA_SELECTOR as u8);
        print("\n  User Code Selector:   0x\0");
        print_hex_byte((USER_CODE_SELECTOR & 0xFF) as u8);
        print("\n  User Data Selector:   0x\0");
        print_hex_byte((USER_DATA_SELECTOR & 0xFF) as u8);
        print("\n  TSS Selector:         0x\0");
        print_hex_byte(TSS_SELECTOR as u8);
        print("\n\0");
        
        // Print memory map address
        print("\nMemory map address: 0x\0");
        let hex_chars: &[u8; 16] = b"0123456789ABCDEF";
        let mut shift: i32 = 60;
        while shift >= 0 {
            let nibble = ((memory_map_addr >> (shift as u64)) & 0xF) as usize;
            let ch = hex_chars[nibble];
            let s: [u8; 2] = [ch, 0];
            print(core::str::from_utf8_unchecked(&s));
            shift -= 4;
        }
        print("\n\0");
        print("\nKernel is stable.\n\0");
    }

    loop {
        unsafe {
            core::arch::asm!("cli");
            core::arch::asm!("hlt");
        }
    }
}
