//! Nafuraito Kernel - Video Driver Library
//!
//! This library provides VGA graphics functionality for the kernel.

#![crate_type = "rlib"]
#![crate_name = "video"]
#![no_std]
#![allow(unused)]

// =========================================================================
// Pixel Structures
// =========================================================================

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

impl Pixel {
    /// Create a new pixel with position and RGB components
    pub const fn new(x: u16, y: u16, r: u8, g: u8, b: u8) -> Self {
        Self { x, y, r, g, b }
    }

    /// Create a pixel from packed RGB color
    pub const fn from_rgb(x: u16, y: u16, color: u32) -> Self {
        Self {
            x,
            y,
            r: ((color >> 16) & 0xFF) as u8,
            g: ((color >> 8) & 0xFF) as u8,
            b: (color & 0xFF) as u8,
        }
    }

    /// Convert to packed RGB color
    pub const fn to_rgb(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

impl PixelRgb {
    /// Create a new pixel with position and packed color
    pub const fn new(x: u16, y: u16, color: u32) -> Self {
        Self { x, y, color }
    }

    /// Sentinel value for terminating pixel arrays
    pub const SENTINEL: Self = Self { x: 0xFFFF, y: 0, color: 0 };
}

// =========================================================================
// FFI Declarations
// =========================================================================

extern "C" {
    // Framebuffer configuration
    fn set_framebuffer_info(addr: u32, pitch: u32, width: u32, height: u32, bpp: u32);
    fn get_framebuffer_address() -> u32;
    fn get_screen_width() -> u32;
    fn get_screen_height() -> u32;
    
    // Graphics initialization
    fn initialize_vga_graphics();
    fn initialize_vga_video();
    
    // Text rendering
    fn writestring_vga(data: *const i8);
    
    // Graphics primitives
    fn put_pixel_vga(x: u16, y: u16, color: u32);
    fn clear_screen_vga(color: u32);
    fn draw_rect_vga(x: u16, y: u16, width: u16, height: u16, color: u32);
    fn draw_line_vga(x0: i16, y0: i16, x1: i16, y1: i16, color: u32);
    
    // Pixel operations
    fn write_pixel(x: u16, y: u16, r: u8, g: u8, b: u8);
    fn write_pixel_rgb(x: u16, y: u16, color: u32);
    fn write_pixel_struct(pixel: *const Pixel);
    fn read_pixel(x: u16, y: u16) -> u32;
    fn is_valid_pixel_position(x: u16, y: u16) -> i32;
    
    // Batch pixel operations
    fn write_pixels(pixels: *const Pixel, count: u32);
    fn write_pixels_rgb(pixels: *const PixelRgb);
}

// =========================================================================
// Color Constants
// =========================================================================

/// Black color (0x000000)
pub const COLOR_BLACK: u32 = 0x000000;
/// White color (0xFFFFFF)
pub const COLOR_WHITE: u32 = 0xFFFFFF;
/// Red color (0xFF0000)
pub const COLOR_RED: u32 = 0xFF0000;
/// Green color (0x00FF00)
pub const COLOR_GREEN: u32 = 0x00FF00;
/// Blue color (0x0000FF)
pub const COLOR_BLUE: u32 = 0x0000FF;
/// Yellow color (0xFFFF00)
pub const COLOR_YELLOW: u32 = 0xFFFF00;
/// Cyan color (0x00FFFF)
pub const COLOR_CYAN: u32 = 0x00FFFF;
/// Magenta color (0xFF00FF)
pub const COLOR_MAGENTA: u32 = 0xFF00FF;
/// Purple color (0x800080)
pub const COLOR_PURPLE: u32 = 0x800080;
/// Orange color (0xFFA500)
pub const COLOR_ORANGE: u32 = 0xFFA500;
/// Gray color (0x808080)
pub const COLOR_GRAY: u32 = 0x808080;
/// Light gray color (0xC0C0C0)
pub const COLOR_LIGHT_GRAY: u32 = 0xC0C0C0;
/// Dark gray color (0x404040)
pub const COLOR_DARK_GRAY: u32 = 0x404040;

// =========================================================================
// Public API - Safe Wrappers
// =========================================================================

/// Set framebuffer information received from bootloader
#[no_mangle]
pub unsafe fn set_framebuffer(addr: u32, pitch: u32, width: u32, height: u32, bpp: u32) {
    set_framebuffer_info(addr, pitch, width, height, bpp);
}

/// Initialize VGA graphics mode
#[no_mangle]
pub unsafe fn init() {
    initialize_vga_graphics();
}

/// Initialize VGA video/text mode
#[no_mangle]
pub unsafe fn init_video() {
    initialize_vga_video();
}

/// Print a null-terminated string
#[no_mangle]
pub unsafe fn print(s: &str) {
    writestring_vga(s.as_ptr() as *const i8);
}

/// Clear the screen with specified color
#[no_mangle]
pub fn clear(color: u32) {
    unsafe { clear_screen_vga(color); }
}

/// Put a single pixel at (x, y) with specified color
#[no_mangle]
pub fn put_pixel(x: u16, y: u16, color: u32) {
    unsafe { put_pixel_vga(x, y, color); }
}

/// Draw a filled rectangle
#[no_mangle]
pub fn draw_rect(x: u16, y: u16, width: u16, height: u16, color: u32) {
    unsafe { draw_rect_vga(x, y, width, height, color); }
}

/// Draw a line from (x0, y0) to (x1, y1)
#[no_mangle]
pub fn draw_line(x0: i16, y0: i16, x1: i16, y1: i16, color: u32) {
    unsafe { draw_line_vga(x0, y0, x1, y1, color); }
}

/// Write a pixel using RGB components
#[no_mangle]
pub fn pixel_rgb(x: u16, y: u16, r: u8, g: u8, b: u8) {
    unsafe { write_pixel(x, y, r, g, b); }
}

/// Write a pixel using packed RGB color
#[no_mangle]
pub fn pixel_packed(x: u16, y: u16, color: u32) {
    unsafe { write_pixel_rgb(x, y, color); }
}

/// Write a pixel from a Pixel struct
#[no_mangle]
pub fn pixel_struct(pixel: &Pixel) {
    unsafe { write_pixel_struct(pixel as *const Pixel); }
}

/// Read the color of a pixel at (x, y)
#[no_mangle]
pub fn get_pixel(x: u16, y: u16) -> u32 {
    unsafe { read_pixel(x, y) }
}

/// Check if pixel position is valid
#[no_mangle]
pub fn is_valid_position(x: u16, y: u16) -> bool {
    unsafe { is_valid_pixel_position(x, y) != 0 }
}

/// Get the framebuffer address
#[no_mangle]
pub fn framebuffer_address() -> u32 {
    unsafe { get_framebuffer_address() }
}

/// Get screen width
#[no_mangle]
pub fn screen_width() -> u32 {
    unsafe { get_screen_width() }
}

/// Get screen height
#[no_mangle]
pub fn screen_height() -> u32 {
    unsafe { get_screen_height() }
}

/// Write multiple pixels from a slice
#[no_mangle]
pub fn write_pixel_batch(pixels: &[Pixel]) {
    unsafe { write_pixels(pixels.as_ptr(), pixels.len() as u32); }
}

/// Write multiple pixels from a sentinel-terminated array
#[no_mangle]
pub fn write_pixel_rgb_batch(pixels: &[PixelRgb]) {
    unsafe { write_pixels_rgb(pixels.as_ptr()); }
}
