use core::{
    convert::From,
    ops::{Add, Sub},
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
pub type IFixed = i16;

/// Private fixed-point intermediate type for multiplication
///
/// Use [`IFixed`] instead for general use.
type IFixedMul = i32;

/// 2D vector type of [`IFixed`]
#[derive(Copy, Clone, Default)]
pub struct Vec2 {
    pub x: IFixed,
    pub y: IFixed,
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
    };
}

impl Vec2 {
    /// Multiply by another vector as a complex number
    #[must_use]
    pub fn rotate(self, other: Self) -> Self {
        let v1 = Vec2Mul::from(self);
        let v2 = Vec2Mul::from(other);
        Self::from(Vec2Mul {
            x: (((v1.x * v2.x) - (v1.y * v2.y)) >> 12),
            y: (((v1.x * v2.y) + (v1.y * v2.x)) >> 12),
        })
    }

    /// Swap x and y
    #[must_use]
    pub fn swap(self) -> Self {
        Self {
            x: self.y,
            y: self.x,
        }
    }

    /// Component-wise absolute value
    #[must_use]
    pub fn component_abs(self) -> Self {
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
pub struct Vec3 {
    pub x: IFixed,
    pub y: IFixed,
    pub z: IFixed,
}

/// Convenience macro for creating vectors via `vec3!(x, y, z)`
macro_rules! vec3 {
    ($x:expr, $y:expr, $z:expr) => {
        Vec3 {
            x: $x,
            y: $y,
            z: $z,
        }
    };
}
