#![crate_type = "staticlib"]
#![no_std]
#![no_main]

extern "C" { fn video_init(); }

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("hlt"); } }
}

#[no_mangle]
pub extern "C" fn enter() -> ! {
    // Add some indication before calling video_init
    unsafe { 
        video_init();
    }
    
    // Add a different halt pattern to distinguish success
    loop { 
        unsafe { 
            core::arch::asm!("nop"); // Different instruction pattern
            core::arch::asm!("hlt"); 
        } 
    }
}
