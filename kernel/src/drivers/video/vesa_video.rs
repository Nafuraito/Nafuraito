// VESA Video Driver
// This video driver provides support VESA pixel based text printing and specific pixel
// manipulation.

// CONTANTS and DEFINITIONS
const VGA_GFX_HEIGHT: u32 = 1920;
const VGA_GFX_WIDTH: u32 = 1080;
const VGA_GFX_BPP: u32 = 3; // 3 Bytes per pixel (24 bit color)
const VGA_GFX_PITCH: u32 = GFX_WIDTH * GFX_BPP; // Number of bytes per row of pixels

// Default framebuffer address - will be overwritten by bootloader info
const VGA_GFX_MEMORY: u32 = 0xFD000000;

// STATIC VARIABLES
static gfx_buffer: *mut u8 = VGA_GFX_MEMORY as *mut u8;
static gfx_height = VGA_GFX_HEIGHT;
static gfx_width = VGA_GFX_WIDTH;
static gfx_bpp = VGA_GFX_BPP;
static gfx_pitch = VGA_GFX_PITCH;

// COLOR MANIPULATION

struct rgb24 {
    b: u8,
    g: u8,
    r: u8,
};

#[inline]
static fn make_rgb(r: u8, g: u8, b: u8) -> rgb24 {
    let color: rgb24 = rgb24 {
        b, g, r
    };
    color
}

#[inline]
static fn rgb_to_u32(rgb: rgb24) -> u32 {
    let r = rgb.r;
    let g = rgb.g;
    let b = rgb.b;
    let res: u32 = ((r << 16) as u32) | ((g << 8) as u32) | ((b) as u32);
    res
}

// INTERNAL GRAPHICS FUNCTIONS

// Set every pixel to black (0, 0, 0)
static fn gfx_clear_screen(rgb: rgb24) {
    let mut x: u32;
    let mut y: u32;

    let b = rgb.b;
    let g = rgb.g;
    let r = rgb.r;

    for y in 0..gfx_height {
        let row: *u8 = gfx_buffer + y * gfx_pitch;
        for x in 0..gfx_width {
            row[x * gfx_bpp + 0] = b;
            row[x * gfx_bpp + 1] = g;
            row[x * gfx_bpp + 2] = r;
            if 4 == gfx_bpp {
                row[x * gfx_bpp + 3] = 0xFF; // Alpha channel
            }
        }
    }
}

// Initializes GFX
static fn gfx_initialize() {
    // mode should already be set by bootloader
    gfx_clear_screen(0, 0, 0);
}

// Put a pixel to screen
static fn gfx_put_pixel(x: u16, y: u16, rgb: rgb24) {
    // Extract r, g and b
    let r: u8 = rgb.r;
    let g: u8 = rgb.g;
    let b: u8 = rgb.b;

    // Check if pixel position is in range
    if x < gfx_width && y < gfx_height {
        let pixel: *mut u8 = (gfx_buffer + y * gfx_pitch + x * gfx_bpp) as *mut u8;
        pixel[0] = b;
        pixel[1] = g;
        pixel[2] = r;
        if 4 == gfx_bpp {
            pixel[3] = 0xFF; // Alpha channel
        }
    }
}

// Draw a rectangle to screen
static fn gfx_draw_rectangle(x: u16, y: u16, width: 16, height: u16, rgb: rgb24) {
    let x_end: u16 = if x + width > gfx_width { gfx_width } else { x + width };
    let y_end: u16 = if y + height > gfx_height { gfx_height } else { y + height };

    let b = rgb.b;
    let g = rgb.g;
    let r = rgb.r;

    for j in y..y_end {
        let row: *u8 = gfx_buffer + j * gfx_pitch;
        for i in x..x_end {
            row[i * gfx_bpp + 0] = b;
            row[i * gfx_bpp + 1] = g;
            row[i * gfx_bpp + 2] = r;
            if 4 == gfx_bpp {
                row[i * gfx_bpp + 3] = 0xFF; // Alpha channel
            }
        }
    }
}

// XXX: Find out, what this does
#[inline]
static fn abs_i16(val: i16) -> i16 {
    return if val < 0 { -val } else { val };
}

// TODO: Definitions only

// Draws Line
static fn gfx_draw_line(x0: i16, y0: i16, x1: i16, y1: i16, rgb: rgb24) {
    todo!("Implement gfx_draw_line() in video driver!");
}


//////////////////////////////////////////
/// Public API (called from Kernel etc.)
//////////////////////////////////////////

// Override the framebuffer address (done by bootloader)
fn set_framebuffer_address(addres: u32, pitch: u32) {
    gfx_buffer = address as *mut u8;
    gfx_pitch = pitch;
}

// bpp_bytes: bytes per pixel (e.g. 4 for 32 bpp GOP)
fn set_framebuffer_info(addres: u32, pitch: u32, width: u32, height: u32, bpp_bytes: u32) {
    gfx_buffer = address as *mut u8;
    gfx_pitch = pitch;
    gfx_width = width;
    gfx_height = height;
    gfx_bpp = if bpp_bytes { bpp_bytes } else { VGA_GFX_BPP };
}

fn get_framebuffer_address() -> u32 {
    return *gfx_buffer as usize as u32;
}

fn get_screen_width() -> u32 {
    return gfx_width;
}

fn get_screen_height() -> u32 {
    return gfx_height;
}

// Initialize the graphics
fn initialize_vesa_graphics() {
    gfx_initialize();
}

// Converts a hex-color to the rgb24 struct
fn hex_to_rgb24(hex: u32) -> rgb24 {
    return rgb24 {
        r: (color >> 16) & 0xFF,
        g: (color >> 8) & 0xFF,
        b: color & 0xFF
    };
}

// Puts a pixel to screen
fn put_pixel_vesa(x: u16, y: u16, color: u32) {
    gfx_put_pixel(x, y, hex_to_rgb24(color));
}

fn clear_screen(color: u32) {
    gfx_clear_screen(hex_to_rgb24(color));
}

fn draw_rectangle_vesa(x: u16, y: u16, width: u16, height: u16, color: u32) {
    gfx_draw_rectangle(x, y, width, height, hex_to_rgb24(color));
}

fn draw_line_vesa(x0: u16, y0: u16, x1: u16, y1: u16, color: u32) {
    gfx_draw_line(x0, y0, x1, y1, hex_to_rgb24(color));
}


/////////////////////////////////////////////////////////
/// Text rendering on graphics mode
/////////////////////////////////////////////////////////

use crate::drivers::video::font_8x16;

// TODO
