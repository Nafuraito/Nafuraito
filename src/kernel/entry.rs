#![crate_type = "staticlib"]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[path = "drivers/drivers.rs"]
mod drivers;
use drivers::drivers::video;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { 
        unsafe { core::arch::asm!("cli"); }
        unsafe { core::arch::asm!("hlt"); } 
    }
}

#[no_mangle]
pub extern "C" fn enter() -> ! {
    unsafe { 
        // Initialize VGA terminal
        video::init_vga();
        
        // Write welcome message (use \0 for null termination)
        video::write_string("Welcome to Nafuraito!\0");
    }
    
    // Halt the CPU - disable interrupts and halt in a loop
    // This prevents any further execution and stops flickering
    loop { 
        unsafe { 
            core::arch::asm!("cli");  // Disable interrupts
            core::arch::asm!("hlt");  // Halt until next interrupt (none will come)
        } 
    }
}
