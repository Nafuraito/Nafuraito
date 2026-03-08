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
static GFX_BUFFER: *mut u8 = VGA_GFX_MEMORY as *mut u8;

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

    for y in 0..GFX_HEIGHT {
        let row: *u8 = GFX_BUFFER + y * GFX_PITCH;
        for x in 0..GFX_WIDTH {
            row[x * GFX_BPP + 0] = b;
            row[x * GFX_BPP + 1] = g;
            row[x * GFX_BPP + 2] = r;
            if 4 == GFX_BPP {
                row[x * GFX_BPP + 3] = 0xFF; // Alpha channel
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
    if x < GFX_WIDTH && y < GFX_HEIGHT {
        let pixel: *mut u8 = (GFX_BUFFER + y * GFX_PITCH + x * GFX_BPP) as *mut u8;
        pixel[0] = b;
        pixel[1] = g;
        pixel[2] = r;
        if 4 == GFX_BPP {
            pixel[3] = 0xFF; // Alpha channel
        }
    }
}

