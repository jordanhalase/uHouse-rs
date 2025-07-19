//! # μHouse-rs
//!
//! This project only supports rendering to an SSD1306 display over I2C.
//! It uses a resolution of 128x64 by default but can be changed by editing
//! the [`Display`] variable.
//!
//! This was made for an Arduino UNO running an Atmega328P.
//!
//! For performance, this project uses a fixed point representation and no
//! matrix math. Rotations are performed using complex number arithmetic and
//! no clipping is performed.
//!
//! Enjoy!

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

#[macro_use]
mod vec;

use arduino_hal::{self, clock::Clock};
use avr_progmem::progmem;
use core::{iter::zip, mem::swap, panic::PanicInfo};
use ssd1306::{I2CDisplayInterface, Ssd1306, prelude::*};

use vec::*;

#[cfg(feature = "fps")]
mod fps;

/// Pick your display size here
type Display = DisplaySize128x64;

/// Pick your clock frequency here
#[allow(unused)]
const CLOCK_FREQ: u32 = arduino_hal::DefaultClock::FREQ;

const SCREEN_WIDTH: IFixed = Display::WIDTH as IFixed;
const SCREEN_HEIGHT: IFixed = Display::HEIGHT as IFixed;
const SCREEN_CENTER: Vec2 = vec2!(SCREEN_WIDTH >> 1, SCREEN_HEIGHT >> 1);

/// How far into the screen to render the mesh
const MESH_DEPTH: IFixed = 0x2a00;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
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

/// Very rudimentary algorithm to discard off-screen geometry
fn point_accept(v: Vec2) -> bool {
    !(v.x < 0 || v.x >= SCREEN_HEIGHT || v.y < 0 || v.y >= SCREEN_HEIGHT)
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
        } else if point_accept(v0) {
            put_pixel(v0.x as u32, v0.y as u32);
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

    let pins = arduino_hal::pins!(dp);

    #[cfg(feature = "fps")]
    let mut fps_counter = unsafe {
        let fps_counter =
            fps::FpsCounter::new(arduino_hal::default_serial!(dp, pins, 57600), dp.TC1);

        // SAFETY: All interrupts and data are configured before calling
        avr_device::interrupt::enable();

        fps_counter
    };

    let i2c = arduino_hal::I2c::new(
        dp.TWI,
        pins.a4.into_pull_up_input(),
        pins.a5.into_pull_up_input(),
        400000,
    );

    let interface = I2CDisplayInterface::new(i2c);
    let mut display =
        Ssd1306::new(interface, Display {}, DisplayRotation::Rotate0).into_buffered_graphics_mode();
    display.init().unwrap();

    display.clear_buffer();

    let mut screen_verts: [Vec2; NUM_VERTS] = [Vec2::default(); _];

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
            let Vec3 { x, y, z } = vec3!(moved.x, v.y + (location.x >> 2), moved.y);

            let z_prime: IFixed = (z + MESH_DEPTH) >> 6;
            let perspective_divided = vec2!(x / z_prime, y / z_prime);

            *screen = perspective_divided + SCREEN_CENTER;
        }

        display.clear_buffer();

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

        display.flush().unwrap();

        #[cfg(feature = "fps")]
        fps_counter.update();
    }
}
