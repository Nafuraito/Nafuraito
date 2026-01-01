//! VGA Video Driver FFI Declarations
//!
//! This file contains the foreign function interface declarations
//! for the C VGA driver. The actual implementation is in vga.c.
//!
//! Note: The main video API is exposed through drivers/drivers.rs

extern "C" {
    /// Initialize VGA text mode display
    pub fn initialize_vga_video();
    
    /// Write a null-terminated string to the display
    pub fn writestring_vga(data: *const u8);
}
