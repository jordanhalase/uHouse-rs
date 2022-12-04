#![no_std]
#![no_main]

#![feature(generic_arg_infer)]

use core::{
    iter::zip,
    convert::From,
    panic::PanicInfo,
};
use arduino_hal;
use avr_progmem::progmem;
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};
use embedded_graphics::{
    Drawable,
    geometry::Point,
    primitives::{line::Line, Primitive, PrimitiveStyle},
    pixelcolor::BinaryColor,
};

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
/// Use [`IFixed'] instead for general use.
type IFixedMul = i32;

/// How far into the screen to render the mesh
const MESH_DEPTH: IFixed = 0x2a00;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[derive(Copy, Clone, Default)]
struct Vec2 {
    x: IFixed,
    y: IFixed,
}

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

impl From<Vec2> for Point {
    fn from(value: Vec2) -> Self {
        Self::new(value.x as i32, value.y as i32)
    }
}

/// Convenience macro for creating vectors via `vec2!(x, y)`
macro_rules! vec2 {
    ($x:expr, $y:expr) => {
        Vec2 { x: $x, y: $y }
    }
}

impl Vec2 {

    #[must_use]
    fn rotate(self, other: Self) -> Self {
        let v1 = Vec2Mul::from(self);
        let v2 = Vec2Mul::from(other);
        Self::from(Vec2Mul {
            x: (((v1.x*v2.x) - (v1.y*v2.y)) >> 12),
            y: (((v1.x*v2.y) + (v1.y*v2.x)) >> 12),
        })
    }

    #[must_use]
    fn swap(self) -> Self {
        Self {
            x: self.y,
            y: self.x,
        }
    }
}

impl core::ops::Add for Vec2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

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

const NUM_VERTS: u8 = 57;
const NUM_LINES: u16 = 68;

progmem! {
    static progmem MESH_VERTS: [Vec3; NUM_VERTS as usize] = [
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

    static progmem MESH_INDICES: [(u8, u8); NUM_LINES as usize] = [
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

/// Constant rotation vector of 2 degrees per frame
/// 
/// From the equation round(4096*exp(3j*pi/180))
const ROT0: Vec2 = vec2!(0xffa, 0xd6);

/// From the equation round(4096*exp(1j*pi/180))
const LOC0: Vec2 = vec2!(0xfff, 0x47);

const SCREEN_CENTER: Vec2 = vec2!(64, 32);

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

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

    arduino_hal::delay_ms(10);

    display.clear();
    display.flush().unwrap();

    let mut screen_verts: [Vec2; NUM_VERTS as usize] = [Vec2::default(); _];

    // Rotation vector, updated per-frame
    let mut rotation = vec2!(0x1000, 0);

    // Location vector, updated per-frame
    let mut location = vec2!(0x1000, 0);

    let mut rotation_counter: u16 = 0;
    let mut location_counter: u16 = 0;

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

        // TODO: Faster line algorithm

        // Draw lines between each vertex
        for pair in MESH_INDICES.iter() {
            // SAFETY: Each index is hard-coded to be in-bounds on the vertices
            unsafe {
                let p1 = Point::from(*screen_verts.get_unchecked(pair.0 as usize));
                let p2 = Point::from(*screen_verts.get_unchecked(pair.1 as usize));
                Line::new(p1, p2)
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(&mut display)
                    .unwrap_unchecked();
            }
        }

        // Draw points for each vertex
        /*for v in &screen_verts {
            display.set_pixel(v.x as u32, v.y as u32, true);
        }*/

        unsafe { display.flush().unwrap_unchecked() };
    }
}
