#![no_std]
#![no_main]
#[allow(unused)]

// PIC ports
const PIC1: u16 = 0x20;
const PIC2: u16 = 0xA0;
const PIC1_COMMAND: u16 = PIC1;
const PIC1_DATA: u16 = PIC1 + 1;
const PIC2_COMMAND: u16 = PIC2;
const PIC2_DATA: u16 = PIC2 + 1;

// Initialization Control Words
const ICW1_ICW4: u8 = 0x01;
const ICW1_INIT: u8 = 0x10;
const ICW4_8086: u8 = 0x01;
const CASCADE_IRQ: u8 = 2;
const PIC_EOI: u8 = 0x20;


#[inline]
unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let ret: u8;
    core::arch::asm!("in al, dx", out("al") ret, in("dx") port, options(nomem, nostack));
    ret
}

#[inline]
unsafe fn io_wait() {
    outb(0x80, 0);
}

pub unsafe fn remap(offset1: u8, offset2: u8) {
    // Typically called as: remap(0x20, 0x28)
    // offset1: 0x20 (32) - Master PIC IRQs 0-7 -> interrupts 32-39
    // offset2: 0x28 (40) - Slave PIC IRQs 8-15 -> interrupts 40-47
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

pub unsafe fn send_eoi(irq: u8) {
    if irq >= 8 {
        outb(PIC2_COMMAND, PIC_EOI);
    }
    outb(PIC1_COMMAND, PIC_EOI);
}

pub unsafe fn disable() {
    outb(PIC1_DATA, 0xff);
    outb(PIC2_DATA, 0xff);
}

pub unsafe fn set_mask(irq_line: u8) {
    let port = if irq_line < 8 {
        PIC1_DATA
    } else {
        PIC2_DATA
    };
    let irq = irq_line % 8;
    let value = inb(port) | (1 << irq);
    outb(port, value);
}

pub unsafe fn clear_mask(irq_line: u8) {
    let port = if irq_line < 8 {
        PIC1_DATA
    } else {
        PIC2_DATA
    };
    let irq = irq_line % 8;
    let value = inb(port) & !(1 << irq);
    outb(port, value);
}
