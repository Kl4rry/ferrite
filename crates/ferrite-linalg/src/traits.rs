pub trait Zero<T> {
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

impl Zero<usize> for usize {
    const ZERO: usize = 0;
}

impl Zero<isize> for isize {
    const ZERO: isize = 0;
}

impl Zero<f32> for f32 {
    const ZERO: f32 = 0.0;
}

impl Zero<f64> for f64 {
    const ZERO: f64 = 0.0;
}

pub trait One<T> {
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

impl One<usize> for usize {
    const ONE: usize = 1;
}

impl One<isize> for isize {
    const ONE: isize = 1;
}

impl One<f32> for f32 {
    const ONE: f32 = 1.0;
}

impl One<f64> for f64 {
    const ONE: f64 = 1.0;
}

pub trait NegOne<T> {
    const NEG_ONE: T;
}

impl NegOne<i8> for i8 {
    const NEG_ONE: i8 = -1;
}

impl NegOne<i16> for i16 {
    const NEG_ONE: i16 = -1;
}

impl NegOne<i32> for i32 {
    const NEG_ONE: i32 = -1;
}

impl NegOne<i64> for i64 {
    const NEG_ONE: i64 = -1;
}

impl NegOne<i128> for i128 {
    const NEG_ONE: i128 = -1;
}

impl NegOne<usize> for usize {
    const NEG_ONE: usize = 1;
}

impl NegOne<f32> for f32 {
    const NEG_ONE: f32 = 1.0;
}

impl NegOne<f64> for f64 {
    const NEG_ONE: f64 = 1.0;
}

#[expect(unused)]
pub trait Two<T> {
    const TWO: T;
}

impl Two<u8> for u8 {
    const TWO: u8 = 2;
}

impl Two<i8> for i8 {
    const TWO: i8 = 2;
}

impl Two<u16> for u16 {
    const TWO: u16 = 2;
}

impl Two<i16> for i16 {
    const TWO: i16 = 2;
}

impl Two<u32> for u32 {
    const TWO: u32 = 2;
}

impl Two<i32> for i32 {
    const TWO: i32 = 2;
}

impl Two<u64> for u64 {
    const TWO: u64 = 2;
}

impl Two<i64> for i64 {
    const TWO: i64 = 2;
}

impl Two<u128> for u128 {
    const TWO: u128 = 2;
}

impl Two<i128> for i128 {
    const TWO: i128 = 2;
}

impl Two<usize> for usize {
    const TWO: usize = 2;
}

impl Two<isize> for isize {
    const TWO: isize = 2;
}

impl Two<f32> for f32 {
    const TWO: f32 = 2.0;
}

impl Two<f64> for f64 {
    const TWO: f64 = 2.0;
}

pub trait Clamp<T> {
    fn clamp_t(self, min: Self, max: Self) -> Self;
}

impl Clamp<u8> for u8 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<i8> for i8 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<u16> for u16 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<i16> for i16 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<u32> for u32 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<i32> for i32 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<u64> for u64 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<i64> for i64 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<u128> for u128 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<i128> for i128 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<usize> for usize {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<isize> for isize {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<f32> for f32 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

impl Clamp<f64> for f64 {
    fn clamp_t(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

pub trait Infinity<T> {
    const INFINITY: T;
    fn is_infinity(&self) -> bool;
}

impl Infinity<f32> for f32 {
    const INFINITY: f32 = 1.0;
    fn is_infinity(&self) -> bool {
        *self == f32::INFINITY
    }
}

impl Infinity<f64> for f64 {
    const INFINITY: f64 = 1.0;
    fn is_infinity(&self) -> bool {
        *self == f64::INFINITY
    }
}

pub trait NaN<T> {
    const NAN: T;
    fn is_nan_t(&self) -> bool;
}

impl NaN<f32> for f32 {
    const NAN: f32 = f32::NAN;
    fn is_nan_t(&self) -> bool {
        self.is_nan()
    }
}

impl NaN<f64> for f64 {
    const NAN: f64 = f64::NAN;
    fn is_nan_t(&self) -> bool {
        self.is_nan()
    }
}

pub trait Sqrt<T> {
    fn sqrt(self) -> Self;
}

impl Sqrt<f32> for f32 {
    fn sqrt(self) -> Self {
        self.sqrt()
    }
}

impl Sqrt<f64> for f64 {
    fn sqrt(self) -> Self {
        self.sqrt()
    }
}

pub trait DotProduct {
    type Output;
    fn dot(self, other: Self) -> Self::Output;
}

pub trait CrossProduct {
    fn cross(self, other: Self) -> Self;
}
