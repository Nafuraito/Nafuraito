pub mod drivers {
    pub mod video {
        extern "C" {
            fn initialize_vga_video();
            fn writestring_vga(data: *const i8);
        }
        
        pub unsafe fn init_vga() {
            initialize_vga_video();
        }
        
        pub unsafe fn write_string(s: &str) {
            // Pass the string pointer directly to C
            // Note: s must be a static string literal (null-terminated in binary)
            writestring_vga(s.as_ptr() as *const i8);
        }
    }
}
