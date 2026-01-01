//! Hardware Drivers Module
//!
//! This module contains all hardware driver implementations.

/// VGA Text Mode Driver
pub mod video {
    extern "C" {
        fn initialize_vga_video();
        fn writestring_vga(data: *const i8);
    }

    /// Initialize the VGA text mode display
    pub unsafe fn init() {
        initialize_vga_video();
    }

    /// Print a null-terminated string to the VGA display
    /// 
    /// # Safety
    /// The string must be null-terminated (end with \0)
    pub unsafe fn print(s: &str) {
        writestring_vga(s.as_ptr() as *const i8);
    }
}
