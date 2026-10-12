
pub(crate) trait Zero<T> {
    const ZERO: T;
}

impl Zero<u8> for u8 {
    const ZERO: u8 = 0;
}

impl Zero<i8> for i8 {
    const ZERO: i8 = 0;
}

impl Zero<u16> for u16 {
    const ZERO: u16 = 0;
}

impl Zero<i16> for i16 {
    const ZERO: i16 = 0;
}

impl Zero<u32> for u32 {
    const ZERO: u32 = 0;
}

impl Zero<i32> for i32 {
    const ZERO: i32 = 0;
}

impl Zero<u64> for u64 {
    const ZERO: u64 = 0;
}

impl Zero<i64> for i64 {
    const ZERO: i64 = 0;
}

impl Zero<u128> for u128 {
    const ZERO: u128 = 0;
}

impl Zero<i128> for i128 {
    const ZERO: i128 = 0;
}

impl Zero<f32> for f32 {
    const ZERO: f32 = 0.0;
}

impl Zero<f64> for f64 {
    const ZERO: f64 = 0.0;
}


pub(crate) trait One<T> {
    const ONE: T;
}

impl One<u8> for u8 {
    const ONE: u8 = 1;
}

impl One<i8> for i8 {
    const ONE: i8 = 1;
}

impl One<u16> for u16 {
    const ONE: u16 = 1;
}

impl One<i16> for i16 {
    const ONE: i16 = 1;
}

impl One<u32> for u32 {
    const ONE: u32 = 1;
}

impl One<i32> for i32 {
    const ONE: i32 = 1;
}

impl One<u64> for u64 {
    const ONE: u64 = 1;
}

impl One<i64> for i64 {
    const ONE: i64 = 1;
}

impl One<u128> for u128 {
    const ONE: u128 = 1;
}

impl One<i128> for i128 {
    const ONE: i128 = 1;
}

impl One<f32> for f32 {
    const ONE: f32 = 1.0;
}

impl One<f64> for f64 {
    const ONE: f64 = 1.0;
}
