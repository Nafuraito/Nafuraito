//! Nafuraito Kernel - PIC Driver Library
//!
//! PIC (Programmable Interrupt Controller) and APIC driver for Nafuraito.
//! Provides support for both legacy 8259 PIC and modern APIC interrupt controllers.

#![crate_type = "rlib"]
#![crate_name = "pic"]
#![no_std]
#![allow(unused)]

// ============================================================================
// LEGACY 8259 PIC
// ============================================================================

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

// ============================================================================
// LOCAL APIC (Advanced Programmable Interrupt Controller)
// ============================================================================

// Local APIC register offsets (relative to APIC base address)
const APIC_ID: u32 = 0x020;
const APIC_VERSION: u32 = 0x030;
const APIC_TPR: u32 = 0x080;          // Task Priority Register
const APIC_APR: u32 = 0x090;          // Arbitration Priority Register
const APIC_PPR: u32 = 0x0A0;          // Processor Priority Register
const APIC_EOI: u32 = 0x0B0;          // End of Interrupt
const APIC_RRD: u32 = 0x0C0;          // Remote Read Register
const APIC_LDR: u32 = 0x0D0;          // Logical Destination Register
const APIC_DFR: u32 = 0x0E0;          // Destination Format Register
const APIC_SPURIOUS: u32 = 0x0F0;     // Spurious Interrupt Vector Register
const APIC_ISR_BASE: u32 = 0x100;     // In-Service Register (0x100-0x170)
const APIC_TMR_BASE: u32 = 0x180;     // Trigger Mode Register (0x180-0x1F0)
const APIC_IRR_BASE: u32 = 0x200;     // Interrupt Request Register (0x200-0x270)
const APIC_ESR: u32 = 0x280;          // Error Status Register
const APIC_ICR_LOW: u32 = 0x300;      // Interrupt Command Register (low)
const APIC_ICR_HIGH: u32 = 0x310;     // Interrupt Command Register (high)
const APIC_TIMER_LVT: u32 = 0x320;    // LVT Timer Register
const APIC_THERMAL_LVT: u32 = 0x330;  // LVT Thermal Sensor Register
const APIC_PERF_LVT: u32 = 0x340;     // LVT Performance Counter Register
const APIC_LINT0_LVT: u32 = 0x350;    // LVT LINT0 Register
const APIC_LINT1_LVT: u32 = 0x360;    // LVT LINT1 Register
const APIC_ERROR_LVT: u32 = 0x370;    // LVT Error Register
const APIC_TIMER_INITIAL: u32 = 0x380; // Timer Initial Count Register
const APIC_TIMER_CURRENT: u32 = 0x390; // Timer Current Count Register
const APIC_TIMER_DIVIDE: u32 = 0x3E0;  // Timer Divide Configuration Register

// APIC Spurious Interrupt Vector Register bits
const APIC_SPURIOUS_ENABLE: u32 = 0x100;  // Bit 8: APIC Software Enable/Disable
const APIC_SPURIOUS_VECTOR: u32 = 0xFF;   // Bits 0-7: Spurious Vector

// APIC Timer LVT bits
const APIC_TIMER_PERIODIC: u32 = 0x20000; // Bit 17: Timer Mode (1 = periodic)
const APIC_TIMER_MASKED: u32 = 0x10000;   // Bit 16: Mask

// MSR for APIC base address
const IA32_APIC_BASE_MSR: u32 = 0x1B;
const IA32_APIC_BASE_ENABLE: u64 = 1 << 11;  // APIC Global Enable
const IA32_APIC_BASE_BSP: u64 = 1 << 8;      // Bootstrap Processor

// ============================================================================
// I/O APIC
// ============================================================================

// I/O APIC register offsets
const IOAPIC_REG_ID: u32 = 0x00;
const IOAPIC_REG_VERSION: u32 = 0x01;
const IOAPIC_REG_ARB: u32 = 0x02;
const IOAPIC_REG_REDTBL_BASE: u32 = 0x10;  // Redirection table starts at 0x10

// I/O APIC redirection entry bits
const IOAPIC_MASKED: u64 = 1 << 16;
const IOAPIC_LEVEL_TRIGGERED: u64 = 1 << 15;
const IOAPIC_ACTIVE_LOW: u64 = 1 << 13;
const IOAPIC_LOGICAL_DEST: u64 = 1 << 11;

// Default I/O APIC address (can be overridden by MADT)
const IOAPIC_DEFAULT_BASE: u64 = 0xFEC00000;


// ============================================================================
// LOW-LEVEL I/O AND MSR HELPERS
// ============================================================================

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

// Public wrappers for external use
#[no_mangle]
pub unsafe extern "C" fn pic_outb(port: u16, val: u8) {
    outb(port, val);
}

#[no_mangle]
pub unsafe extern "C" fn pic_inb(port: u16) -> u8 {
    inb(port)
}

/// Initialize the PIC with standard remapping (IRQs 0-15 -> vectors 32-47)
#[no_mangle]
pub unsafe extern "C" fn pic_init() {
    remap(0x20, 0x28);
}

#[inline]
unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack)
    );
    ((high as u64) << 32) | (low as u64)
}

#[inline]
unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    core::arch::asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, nostack)
    );
}

#[inline]
unsafe fn read_volatile_u32(addr: u64) -> u32 {
    core::ptr::read_volatile(addr as *const u32)
}

#[inline]
unsafe fn write_volatile_u32(addr: u64, value: u32) {
    core::ptr::write_volatile(addr as *mut u32, value);
}

// ============================================================================
// CPUID HELPERS
// ============================================================================

#[inline]
unsafe fn cpuid(leaf: u32) -> (u32, u32, u32, u32) {
    let mut eax: u32;
    let mut ebx: u32;
    let mut ecx: u32 = 0;
    let mut edx: u32;
    core::arch::asm!(
        "mov {0:r}, rbx",
        "cpuid",
        "xchg {0:r}, rbx",
        out(reg) ebx,
        inout("eax") leaf => eax,
        inout("ecx") ecx => ecx,
        out("edx") edx,
        options(nomem, nostack)
    );
    (eax, ebx, ecx, edx)
}

/// Check if APIC is supported via CPUID
#[no_mangle]
pub unsafe extern "C" fn has_apic() -> bool {
    let (_, _, _, edx) = cpuid(1);
    (edx & (1 << 9)) != 0  // Bit 9 in EDX indicates APIC support
}

/// Check if x2APIC is supported via CPUID
#[no_mangle]
pub unsafe extern "C" fn has_x2apic() -> bool {
    let (_, _, ecx, _) = cpuid(1);
    (ecx & (1 << 21)) != 0  // Bit 21 in ECX indicates x2APIC support
}

/// Remap PIC interrupt vectors
#[no_mangle]
pub unsafe extern "C" fn remap(offset1: u8, offset2: u8) {
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

/// Send End of Interrupt to PIC
#[no_mangle]
pub unsafe extern "C" fn send_eoi(irq: u8) {
    if irq >= 8 {
        outb(PIC2_COMMAND, PIC_EOI);
    }
    outb(PIC1_COMMAND, PIC_EOI);
}

/// Disable the PIC
#[no_mangle]
pub unsafe extern "C" fn disable() {
    outb(PIC1_DATA, 0xff);
    outb(PIC2_DATA, 0xff);
}

/// Set (mask) an IRQ line
#[no_mangle]
pub unsafe extern "C" fn set_mask(irq_line: u8) {
    let port = if irq_line < 8 {
        PIC1_DATA
    } else {
        PIC2_DATA
    };
    let irq = irq_line % 8;
    let value = inb(port) | (1 << irq);
    outb(port, value);
}

/// Clear (unmask) an IRQ line
#[no_mangle]
pub unsafe extern "C" fn clear_mask(irq_line: u8) {
    let port = if irq_line < 8 {
        PIC1_DATA
    } else {
        PIC2_DATA
    };
    let irq = irq_line % 8;
    let value = inb(port) & !(1 << irq);
    outb(port, value);
}

// ============================================================================
// LOCAL APIC FUNCTIONS
// ============================================================================

/// Get the physical address of the Local APIC
#[no_mangle]
pub unsafe extern "C" fn get_local_apic_base() -> u64 {
    let apic_base_msr = rdmsr(IA32_APIC_BASE_MSR);
    apic_base_msr & 0xFFFF_FFFF_FFFF_F000  // Mask out lower 12 bits
}

/// Enable the Local APIC
#[no_mangle]
pub unsafe extern "C" fn enable_local_apic() {
    // Enable APIC via MSR
    let apic_base_msr = rdmsr(IA32_APIC_BASE_MSR);
    wrmsr(IA32_APIC_BASE_MSR, apic_base_msr | IA32_APIC_BASE_ENABLE);
    
    // Get APIC base address
    let apic_base = get_local_apic_base();
    
    // Set spurious interrupt vector and enable APIC
    // Vector 0xFF (255) is commonly used for spurious interrupts
    let spurious_reg = apic_base + APIC_SPURIOUS as u64;
    let spurious_value = read_volatile_u32(spurious_reg);
    write_volatile_u32(spurious_reg, spurious_value | APIC_SPURIOUS_ENABLE | 0xFF);
}

/// Disable the Local APIC
#[no_mangle]
pub unsafe extern "C" fn disable_local_apic() {
    let apic_base = get_local_apic_base();
    let spurious_reg = apic_base + APIC_SPURIOUS as u64;
    let spurious_value = read_volatile_u32(spurious_reg);
    write_volatile_u32(spurious_reg, spurious_value & !APIC_SPURIOUS_ENABLE);
}

/// Send End of Interrupt to Local APIC
#[no_mangle]
pub unsafe extern "C" fn local_apic_eoi() {
    let apic_base = get_local_apic_base();
    let eoi_reg = apic_base + APIC_EOI as u64;
    write_volatile_u32(eoi_reg, 0);  // Writing any value signals EOI
}

/// Read Local APIC register
#[no_mangle]
pub unsafe extern "C" fn local_apic_read(offset: u32) -> u32 {
    let apic_base = get_local_apic_base();
    read_volatile_u32(apic_base + offset as u64)
}

/// Write Local APIC register
#[no_mangle]
pub unsafe extern "C" fn local_apic_write(offset: u32, value: u32) {
    let apic_base = get_local_apic_base();
    write_volatile_u32(apic_base + offset as u64, value);
}

/// Get Local APIC ID
#[no_mangle]
pub unsafe extern "C" fn get_local_apic_id() -> u32 {
    (local_apic_read(APIC_ID) >> 24) & 0xFF
}

/// Initialize Local APIC timer in periodic mode
#[no_mangle]
pub unsafe extern "C" fn local_apic_timer_init(vector: u8, divider: u32, initial_count: u32) {
    // Set timer divide configuration
    let divide_value = match divider {
        1 => 0b1011,
        2 => 0b0000,
        4 => 0b0001,
        8 => 0b0010,
        16 => 0b0011,
        32 => 0b1000,
        64 => 0b1001,
        128 => 0b1010,
        _ => 0b0000,  // Default to divide by 2
    };
    local_apic_write(APIC_TIMER_DIVIDE, divide_value);
    
    // Set LVT Timer Register (periodic mode, not masked)
    local_apic_write(APIC_TIMER_LVT, APIC_TIMER_PERIODIC | (vector as u32));
    
    // Set initial count
    local_apic_write(APIC_TIMER_INITIAL, initial_count);
}

/// Stop Local APIC timer
#[no_mangle]
pub unsafe extern "C" fn local_apic_timer_stop() {
    local_apic_write(APIC_TIMER_LVT, APIC_TIMER_MASKED);
    local_apic_write(APIC_TIMER_INITIAL, 0);
}
