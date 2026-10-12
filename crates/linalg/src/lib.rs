pub mod rect_impl;
mod traits;
pub mod vec2_impl;
pub mod vec3_impl;
pub mod vec4_impl;

pub use traits::{CrossProduct, DotProduct};

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Vec4<T> {
    pub x: T,
    pub y: T,
    pub z: T,
    pub w: T,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Vec3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Vec2<T> {
    pub x: T,
    pub y: T,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Mat4<T> {
    pub x: Vec4<T>,
    pub y: Vec4<T>,
    pub z: Vec4<T>,
    pub w: Vec4<T>,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Mat3<T> {
    pub x: Vec3<T>,
    pub y: Vec3<T>,
    pub z: Vec3<T>,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Mat2<T> {
    pub x: Vec2<T>,
    pub y: Vec2<T>,
    pub z: Vec2<T>,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

pub fn dot<T: DotProduct<Output = O>, O>(lhs: T, rhs: T) -> O {
    lhs.dot(rhs)
}

pub fn cross<T: CrossProduct>(lhs: T, rhs: T) -> T {
    lhs.cross(rhs)
}

#[inline]
pub const fn vec4<T>(x: T, y: T, z: T, w: T) -> Vec4<T> {
    Vec4 { x, y, z, w }
}

#[inline]
pub const fn vec3<T>(x: T, y: T, z: T) -> Vec3<T> {
    Vec3 { x, y, z }
}

#[inline]
pub const fn vec2<T>(x: T, y: T) -> Vec2<T> {
    Vec2 { x, y }
}
