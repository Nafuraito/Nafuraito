//! VGA Video Driver FFI Declarations
//!
//! This file contains the foreign function interface declarations
//! for the C VGA driver. The actual implementation is in vga.c.
//!
//! Note: The main video API is exposed through drivers/drivers.rs

/// Pixel structure with separate RGB components
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Pixel {
    pub x: u16,
    pub y: u16,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Pixel structure with packed RGB color
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PixelRgb {
    pub x: u16,
    pub y: u16,
    pub color: u32,  // 0xRRGGBB format
}

extern "C" {
    // Framebuffer configuration
    pub fn set_framebuffer_info(addr: u32, pitch: u32, width: u32, height: u32, bpp: u32);
    pub fn get_framebuffer_address() -> u32;
    pub fn get_screen_width() -> u32;
    pub fn get_screen_height() -> u32;
    
    // Graphics initialization
    pub fn initialize_vga_graphics();
    
    // Graphics primitives
    pub fn put_pixel_vga(x: u16, y: u16, color: u32);
    pub fn clear_screen_vga(color: u32);
    pub fn draw_rect_vga(x: u16, y: u16, width: u16, height: u16, color: u32);
    pub fn draw_line_vga(x0: i16, y0: i16, x1: i16, y1: i16, color: u32);
    
    // Text mode compatibility
    pub fn initialize_vga_video();
    pub fn writestring_vga(data: *const u8);
    
    // Pixel operations
    pub fn write_pixel(x: u16, y: u16, r: u8, g: u8, b: u8);
    pub fn write_pixel_rgb(x: u16, y: u16, color: u32);
    pub fn write_pixel_struct(pixel: *const Pixel);
    pub fn read_pixel(x: u16, y: u16) -> u32;
    pub fn is_valid_pixel_position(x: u16, y: u16) -> i32;
    
    // Batch pixel operations
    pub fn write_pixels(pixels: *const Pixel, count: u32);
    pub fn write_pixels_rgb(pixels: *const PixelRgb);
    pub fn write_pixels_arrays(x_coords: *const u16, y_coords: *const u16, colors: *const u32);
}
