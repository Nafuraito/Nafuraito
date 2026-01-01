extern "C" {
    pub fn initialize_vga_video();
    pub fn writestring_vga(data: *const u8);
}
