//! Hardware Drivers Module
//!
//! This module contains all hardware driver implementations.

/// VGA Graphics and Text Mode Driver
pub mod video {
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

    extern "C" {
        fn initialize_vga_video();
        fn initialize_vga_graphics();
        fn writestring_vga(data: *const i8);
        fn set_framebuffer_info(addr: u32, pitch: u32, width: u32, height: u32, bpp: u32);
        fn get_framebuffer_address() -> u32;
        fn get_screen_width() -> u32;
        fn get_screen_height() -> u32;
        fn put_pixel_vga(x: u16, y: u16, color: u32);
        fn clear_screen_vga(color: u32);
        fn draw_rect_vga(x: u16, y: u16, width: u16, height: u16, color: u32);
        fn draw_line_vga(x0: i16, y0: i16, x1: i16, y1: i16, color: u32);
        fn write_pixel(x: u16, y: u16, r: u8, g: u8, b: u8);
        fn write_pixel_rgb(x: u16, y: u16, color: u32);
        fn write_pixel_struct(pixel: *const Pixel);
        fn read_pixel(x: u16, y: u16) -> u32;
        fn is_valid_pixel_position(x: u16, y: u16) -> i32;
        fn write_pixels(pixels: *const Pixel, count: u32);
        fn write_pixels_rgb(pixels: *const PixelRgb);
        fn write_pixels_arrays(x_coords: *const u16, y_coords: *const u16, colors: *const u32);
    }

    // =========================================================================
    // Framebuffer Configuration
    // =========================================================================

    /// Set the framebuffer info received from bootloader
    /// 
    /// # Safety
    /// Must be called before init() with valid framebuffer info
    pub unsafe fn set_framebuffer(addr: u32, pitch: u32, width: u32, height: u32, bpp: u32) {
        set_framebuffer_info(addr, pitch, width, height, bpp);
    }

    /// Get the framebuffer physical address
    pub fn get_framebuffer() -> u32 {
        unsafe { get_framebuffer_address() }
    }

    /// Get the screen width in pixels
    pub fn width() -> u32 {
        unsafe { get_screen_width() }
    }

    /// Get the screen height in pixels
    pub fn height() -> u32 {
        unsafe { get_screen_height() }
    }

    // =========================================================================
    // Initialization
    // =========================================================================

    /// Initialize the VGA text mode display (text rendering on graphics mode)
    pub unsafe fn init() {
        initialize_vga_video();
    }

    /// Initialize graphics mode without text rendering
    pub unsafe fn init_graphics() {
        initialize_vga_graphics();
    }

    // =========================================================================
    // Text Rendering
    // =========================================================================

    /// Print a null-terminated string to the VGA display
    /// 
    /// # Safety
    /// The string must be null-terminated (end with \0)
    pub unsafe fn print(s: &str) {
        writestring_vga(s.as_ptr() as *const i8);
    }

    // =========================================================================
    // Graphics Primitives
    // =========================================================================

    /// Draw a single pixel at (x, y) with the specified color
    /// 
    /// Color format: 0x00RRGGBB (24-bit RGB)
    pub fn put_pixel(x: u16, y: u16, color: u32) {
        unsafe { put_pixel_vga(x, y, color) }
    }

    /// Clear the entire screen with the specified color
    /// 
    /// Color format: 0x00RRGGBB (24-bit RGB)
    pub fn clear(color: u32) {
        unsafe { clear_screen_vga(color) }
    }

    /// Draw a filled rectangle
    /// 
    /// Color format: 0x00RRGGBB (24-bit RGB)
    pub fn draw_rect(x: u16, y: u16, width: u16, height: u16, color: u32) {
        unsafe { draw_rect_vga(x, y, width, height, color) }
    }

    /// Draw a line from (x0, y0) to (x1, y1)
    /// 
    /// Color format: 0x00RRGGBB (24-bit RGB)
    pub fn draw_line(x0: i16, y0: i16, x1: i16, y1: i16, color: u32) {
        unsafe { draw_line_vga(x0, y0, x1, y1, color) }
    }

    // =========================================================================
    // Pixel Operations
    // =========================================================================

    /// Write a single pixel with separate RGB components
    pub fn write_pixel_rgb_components(x: u16, y: u16, r: u8, g: u8, b: u8) {
        unsafe { write_pixel(x, y, r, g, b) }
    }

    /// Write a single pixel with packed RGB color (alias for put_pixel)
    pub fn write_pixel_packed(x: u16, y: u16, color: u32) {
        unsafe { write_pixel_rgb(x, y, color) }
    }

    /// Write a pixel using a Pixel structure
    pub fn write_from_struct(pixel: &Pixel) {
        unsafe { write_pixel_struct(pixel as *const Pixel) }
    }

    /// Read the color of a pixel at the specified position
    /// 
    /// Returns packed RGB color (0x00RRGGBB format), or 0 if out of bounds
    pub fn read_pixel_color(x: u16, y: u16) -> u32 {
        unsafe { read_pixel(x, y) }
    }

    /// Check if a coordinate is within screen bounds
    pub fn is_valid_position(x: u16, y: u16) -> bool {
        unsafe { is_valid_pixel_position(x, y) != 0 }
    }

    /// Write multiple pixels from a slice of Pixel structures
    pub fn write_pixels_batch(pixels: &[Pixel]) {
        if !pixels.is_empty() {
            unsafe { write_pixels(pixels.as_ptr(), pixels.len() as u32) }
        }
    }

    /// Write multiple pixels from a slice of PixelRgb structures
    /// 
    /// Note: The slice must be terminated with PixelRgb::SENTINEL
    pub fn write_pixels_rgb_batch(pixels: &[PixelRgb]) {
        if !pixels.is_empty() {
            unsafe { write_pixels_rgb(pixels.as_ptr()) }
        }
    }

    /// Write pixels from separate coordinate and color slices
    /// 
    /// All slices must have the same length.
    /// The x_coords slice must be terminated with 0xFFFF.
    pub fn write_pixels_from_arrays(x_coords: &[u16], y_coords: &[u16], colors: &[u32]) {
        if !x_coords.is_empty() && x_coords.len() == y_coords.len() && y_coords.len() == colors.len() {
            unsafe {
                write_pixels_arrays(x_coords.as_ptr(), y_coords.as_ptr(), colors.as_ptr())
            }
        }
    }

    // =========================================================================
    // Color Constants
    // =========================================================================

    pub const COLOR_BLACK: u32 = 0x000000;
    pub const COLOR_WHITE: u32 = 0xFFFFFF;
    pub const COLOR_RED: u32 = 0xFF0000;
    pub const COLOR_GREEN: u32 = 0x00FF00;
    pub const COLOR_BLUE: u32 = 0x0000FF;
    pub const COLOR_YELLOW: u32 = 0xFFFF00;
    pub const COLOR_CYAN: u32 = 0x00FFFF;
    pub const COLOR_MAGENTA: u32 = 0xFF00FF;
    pub const COLOR_GRAY: u32 = 0x808080;
    pub const COLOR_DARK_GRAY: u32 = 0x404040;
    pub const COLOR_LIGHT_GRAY: u32 = 0xC0C0C0;
    pub const COLOR_ORANGE: u32 = 0xFF8000;
    pub const COLOR_PURPLE: u32 = 0x800080;
}
