#![no_std]
#![no_main]

#![feature(generic_arg_infer)]
//#![feature(stmt_expr_attributes)]
#![feature(abi_avr_interrupt)]

use core::{
    convert::From,
    iter::zip,
    mem::swap,
    ops::{Add, Sub},
    panic::PanicInfo,
};
use arduino_hal;
use avr_progmem::progmem;
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};

#[cfg(feature = "fps")]
mod fps;

/// Fixed-point type
/// 
/// This project uses a fixed point representation for mesh vertices and their
/// transforms to improve performance. The Atmega328P has no floating point
/// unit so all floating point math would be done in software suffering a
/// performance hit.
/// 
/// The fixed point representation here uses 16-bit signed integers with a
/// 12-bit fractional part. This allows a granularity of ~0.000244 with an
/// integer part in the range [-8, 7].
type IFixed = i16;

/// Private fixed-point intermediate type for multiplication
/// 
/// Use [`IFixed`] instead for general use.
type IFixedMul = i32;

/// How far into the screen to render the mesh
const MESH_DEPTH: IFixed = 0x2a00;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

/// 2D vector type of [`IFixed`]
#[derive(Copy, Clone, Default)]
struct Vec2 {
    x: IFixed,
    y: IFixed,
}

/// Private intermediate 2D vector type for multiplication
/// 
/// Use [`Vec2`] instead for general use.
struct Vec2Mul {
    x: IFixedMul,
    y: IFixedMul,
}

impl From<Vec2> for Vec2Mul {
    fn from(value: Vec2) -> Self {
        Self {
            x: value.x as IFixedMul,
            y: value.y as IFixedMul,
        }
    }
}

impl From<Vec2Mul> for Vec2 {
    fn from(value: Vec2Mul) -> Self {
        Self {
            x: value.x as IFixed,
            y: value.y as IFixed,
        }
    }
}

/// Convenience macro for creating vectors via `vec2!(x, y)`
macro_rules! vec2 {
    ($x:expr, $y:expr) => {
        Vec2 { x: $x, y: $y }
    }
}

impl Vec2 {

    /// Multiply by another vector as a complex number
    #[must_use]
    fn rotate(self, other: Self) -> Self {
        let v1 = Vec2Mul::from(self);
        let v2 = Vec2Mul::from(other);
        Self::from(Vec2Mul {
            x: (((v1.x*v2.x) - (v1.y*v2.y)) >> 12),
            y: (((v1.x*v2.y) + (v1.y*v2.x)) >> 12),
        })
    }

    /// Swap x and y
    #[must_use]
    fn swap(self) -> Self {
        Self {
            x: self.y,
            y: self.x,
        }
    }

    /// Component-wise absolute value
    #[must_use]
    fn component_abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

/// 3D vector type of [`IFixed`]
#[derive(Copy, Clone)]
struct Vec3 {
    x: IFixed,
    y: IFixed,
    z: IFixed,
}

/// Convenience macro for creating vectors via `vec3!(x, y, z)`
macro_rules! vec3 {
    ($x:expr, $y:expr, $z:expr) => {
        Vec3 { x: $x, y: $y, z: $z }
    }
}

const NUM_VERTS: usize = 57;
const NUM_LINES: usize = 68;

progmem! {

    /// Mesh vertices in program memory
    static progmem MESH_VERTS: [Vec3; NUM_VERTS] = [
        // Cube
        vec3!( 0x800,  0x800,  0x800),
        vec3!(-0x800,  0x800,  0x800),
        vec3!(-0x800, -0x800,  0x800),
        vec3!( 0x800, -0x800,  0x800),
        vec3!( 0x800,  0x800, -0x800),
        vec3!(-0x800,  0x800, -0x800),
        vec3!(-0x800, -0x800, -0x800),
        vec3!( 0x800, -0x800, -0x800),

        // Roof
        vec3!( 0x000, -0x1400, 0x000),

        // Door
        vec3!(-0x100,  0x800, -0x800),
        vec3!(-0x600,  0x800, -0x800),
        vec3!(-0x600,  0x200, -0x800),
        vec3!(-0x100,  0x200, -0x800),

        // Front window
        vec3!( 0x500, -0x200, -0x800),
        vec3!( 0x200, -0x200, -0x800),
        vec3!( 0x200, -0x500, -0x800),
        vec3!( 0x500, -0x500, -0x800),

        // Left window
        vec3!(-0x800,  0x500,  0x200),
        vec3!(-0x800,  0x500,  0x500),
        vec3!(-0x800,  0x200,  0x500),
        vec3!(-0x800,  0x200,  0x200),

        // Car
        vec3!(-0x800,  0x800,  0xb00),
        vec3!( 0x800,  0x800,  0xb00),
        vec3!( 0x800,  0x500,  0xb00),
        vec3!( 0x400,  0x500,  0xb00),
        vec3!( 0x200,  0x200,  0xb00),
        vec3!(-0x600,  0x200,  0xb00),
        vec3!(-0x800,  0x500,  0xb00),
        vec3!(-0x800,  0x800,  0x1200),
        vec3!( 0x800,  0x800,  0x1200),
        vec3!( 0x800,  0x500,  0x1200),
        vec3!( 0x400,  0x500,  0x1200),
        vec3!( 0x200,  0x200,  0x1200),
        vec3!(-0x600,  0x200,  0x1200),
        vec3!(-0x800,  0x500,  0x1200),

        // Tree
        vec3!( 0x1000,  0x800,   0x000),
        vec3!( 0x1000, -0x1400,  0x000),
        vec3!( 0x1000,  0x200,   0x000), // Branch base
        vec3!( 0x1400, -0x1000,  0x000),
        vec3!( 0xc00,  -0x1000,  0x000),
        vec3!( 0x1000, -0x1000,  0x400),
        vec3!( 0x1000, -0x1000, -0x400),

        // Fence
        vec3!(-0x800,   0x800,   0x000),
        vec3!(-0x1400,  0x800,   0x000),
        vec3!(-0x1400,  0x200,   0x000),
        vec3!(-0x1200,  0x000,   0x000),
        vec3!(-0x1000,  0x200,   0x000),
        vec3!(-0xe00,   0x000,   0x000),
        vec3!(-0xc00,   0x200,   0x000),
        vec3!(-0xa00,   0x000,   0x000),
        vec3!(-0x800,   0x200,   0x000),
        vec3!(-0x1000,  0x800,   0x000),
        vec3!(-0xc00,   0x800,   0x000),

        // Welcome mat
        vec3!(-0x100,  0x800, -0x900),
        vec3!(-0x600,  0x800, -0x900),
        vec3!(-0x600,  0x800, -0xc00),
        vec3!(-0x100,  0x800, -0xc00),
    ];

    /// Line segments as indices into [`MESH_VERTS`]
    static progmem MESH_INDICES: [(u8, u8); NUM_LINES] = [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7),
        (2, 8), (3, 8), (6, 8), (7, 8),             // Roof
        (10, 11), (11, 12), (12, 9),                // Door
        (13, 14), (14, 15), (15, 16), (16, 13),     // Front window
        (17, 18), (18, 19), (19, 20), (20, 17),     // Left window
        (21, 22), (22, 23), (23, 24), (24, 25), (25, 26), (26, 27), (27, 21),   // Car inner side
        (28, 29), (29, 30), (30, 31), (31, 32), (32, 33), (33, 34), (34, 28),   // Car outer side
        (21, 28), (22, 29), (23, 30), (24, 31), (25, 32), (26, 33), (27, 34),   // Car body
        (35, 36), (37, 38), (37, 39), (37, 40), (37, 41),                       // Tree
        (42, 43), (43, 44), (44, 45), (45, 46), (46, 47), (47, 48), (48, 49), (49, 50), (50, 42), (46, 51), (48, 52),   // Fence
        (53, 54), (54, 55), (55, 56), (56, 53),     // Welcome mat
    ];
}

/// Constant rotation vector of 3 degrees per frame
/// 
/// From the equation `round(4096*exp(3j*pi/180))`
const ROT0: Vec2 = vec2!(0xffa, 0xd6);

/// Constant rotation vector of 1 degree per frame
/// 
/// From the equation `round(4096*exp(1j*pi/180))`
const LOC0: Vec2 = vec2!(0xfff, 0x47);

const SCREEN_WIDTH: IFixed = 128;
const SCREEN_HEIGHT: IFixed = 64;
const SCREEN_CENTER: Vec2 = vec2!(64, 32);

/// Very rudimentary algorithm to discard off-screen geometry
fn point_accept(v: Vec2) -> bool {
    if v.x < 0 {
        false
    } else if v.x >= SCREEN_WIDTH {
        false
    } else if v.y < 0 {
        false
    } else if v.y >= SCREEN_HEIGHT {
        false
    } else {
        true
    }
}

/// Bresenham's line algorithm
fn draw_line<F: FnMut(u32, u32)>(mut put_pixel: F, mut v0: Vec2, mut v1: Vec2) {
    let should_swap = {
        let d = (v1 - v0).component_abs();
        d.y > d.x
    };

    if should_swap {
        swap(&mut v0.x, &mut v0.y);
        swap(&mut v1.x, &mut v1.y);
    }

    if v0.x > v1.x {
        swap(&mut v0, &mut v1);
    }

    let dx = v1.x - v0.x;
    let dy = (v1.y - v0.y).abs();

    let y_step = if v0.y < v1.y { 1 } else { -1 };
    let mut half_diff = -(dx >> 1);

    while v0.x <= v1.x {
        if should_swap {
            if point_accept(v0.swap()) {
                put_pixel(v0.y as u32, v0.x as u32);
            }
        } else {
            if point_accept(v0) {
                put_pixel(v0.x as u32, v0.y as u32);
            }
        }

        half_diff += dy;
        if half_diff > 0 {
            half_diff -= dx;
            v0.y += y_step;
        }
        v0.x += 1;
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();

    #[cfg(feature = "fps")]
    fps::setup_timer1(&dp);

    let pins = arduino_hal::pins!(dp);

    #[cfg(feature = "fps")]
    let mut fps_counter = fps::FpsCounter::new(
        arduino_hal::default_serial!(dp, pins, 57600)
    );

    #[cfg(feature = "fps")]
    unsafe {
        avr_device::interrupt::enable();
    }

    let i2c = arduino_hal::I2c::new(
        dp.TWI,
        pins.a4.into_pull_up_input(),
        pins.a5.into_pull_up_input(),
        400000
    );

    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(
        interface,
        DisplaySize128x64,
        DisplayRotation::Rotate0,
    ).into_buffered_graphics_mode();
    display.init().unwrap();

    display.clear();

    let mut screen_verts: [Vec2; NUM_VERTS] = [Vec2::default(); _];

    // Rotation vector, updated per-frame
    let mut rotation = vec2!(0x1000, 0);

    // Location vector, updated per-frame
    let mut location = vec2!(0x1000, 0);

    let mut rotation_counter: u16 = 0;
    let mut location_counter: u16 = 0;

    //uwriteln!(&mut serial, "Hello!").unwrap();

    loop {
        
        // Rotate the rotation vectors
        rotation = rotation.rotate(ROT0);
        location = location.rotate(LOC0);

        rotation_counter += 1;
        location_counter += 1;

        // Reset the rotation vectors each revolution to avoid precision loss
        if rotation_counter >= 120 {
            rotation_counter = 0;
            rotation = vec2!(0x1000, 0);
        }
        if location_counter >= 360 {
            location_counter = 0;
            location = vec2!(0x1000, 0);
        }

        // Transform vertices from model space into screen space
        for (v, screen) in zip(MESH_VERTS.iter(), &mut screen_verts) {
            
            // Rotate mesh and move up and down
            let moved = vec2!(v.x, v.z).rotate(rotation) + location.swap();
            let Vec3 { x, y, z } = vec3!(
                moved.x,
                v.y + (location.x >> 2),
                moved.y
            );

            let z_prime: IFixed = (z + MESH_DEPTH) >> 6;
            let perspective_divided = vec2!(x/z_prime, y/z_prime);
            
            *screen = perspective_divided + SCREEN_CENTER;   
        }

        display.clear();

        // Faster line algorithm
        for pair in MESH_INDICES.iter() {
            unsafe {
                // SAFETY: Array is hard-coded to index into vertices so there
                // is no chance for an out-of-bounds access
                let v0 = *screen_verts.get_unchecked(pair.0 as usize);
                let v1 = *screen_verts.get_unchecked(pair.1 as usize);
            
                draw_line(|x, y| display.set_pixel(x, y, true), v0, v1);
            }
        }

        // Draw points for each vertex
        /*for v in &screen_verts {
            display.set_pixel(v.x as u32, v.y as u32, true);
        }*/

        display.flush().unwrap();

        #[cfg(feature = "fps")]
        fps_counter.update();
    }
}
