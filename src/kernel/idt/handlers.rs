//! Interrupt and exception handlers

/// External VGA print function from C
extern "C" {
    fn writestring_vga(data: *const i8);
}

/// Print a string using VGA (simple version without buffer allocation)
#[allow(static_mut_refs)]
fn print(s: &str) {
    unsafe {
        // Create a static buffer for null-terminated strings
        static mut PRINT_BUFFER: [u8; 256] = [0; 256];
        
        let bytes = s.as_bytes();
        let len = bytes.len().min(255);
        
        // Manual copy using raw pointers to avoid memcpy
        let src = bytes.as_ptr();
        let dst = (&raw mut PRINT_BUFFER).cast::<u8>();
        let mut i = 0;
        while i < len {
            core::ptr::write_volatile(dst.add(i), core::ptr::read_volatile(src.add(i)));
            i += 1;
        }
        core::ptr::write_volatile(dst.add(len), 0); // Null terminator
        
        writestring_vga((&raw const PRINT_BUFFER).cast::<i8>());
    }
}

/// Print a hexadecimal number
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
        HEX_BUFFER[18] = 0; // Null terminator
        
        writestring_vga((&raw const HEX_BUFFER).cast::<i8>());
    }
}

/// Exception handler called from assembly
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

    // Halt the CPU on exceptions for now
    unsafe {
        loop {
            core::arch::asm!("hlt");
        }
    }
}

/// IRQ handler called from assembly
#[no_mangle]
pub extern "C" fn irq_handler(irq: u64) {
    match irq {
        32 => {
            // Timer IRQ
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
            // Keyboard IRQ
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
            // Other IRQs - just acknowledge
            print("IRQ received: ");
            print_hex(irq);
            print("\n\0");
        }
    }

    // Send End of Interrupt (EOI) to PIC
    send_eoi(irq);
}

/// Send End of Interrupt signal to PIC
fn send_eoi(irq: u64) {
    const PIC1_COMMAND: u16 = 0x20;
    const PIC2_COMMAND: u16 = 0xA0;
    const PIC_EOI: u8 = 0x20;

    unsafe {
        if irq >= 40 {
            // IRQ from slave PIC
            outb(PIC2_COMMAND, PIC_EOI);
        }
        // Always send to master PIC
        outb(PIC1_COMMAND, PIC_EOI);
    }
}

/// Output byte to I/O port
#[inline]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}
