use super::{Vec2, Vec3, Vec4};
use crate::traits::*;

impl<T> Vec4<T> {
    #[inline]
    pub const fn new(x: T, y: T, z: T, w: T) -> Self {
        Self { x, y, z, w }
    }
}

impl<T> Vec4<T>
where
    T: Copy,
{
    #[inline]
    pub const fn splat(t: T) -> Self {
        Self {
            x: t,
            y: t,
            z: t,
            w: t,
        }
    }

    #[inline]
    pub const fn to_array(self) -> [T; 4] {
        [self.x, self.y, self.z, self.w]
    }

    #[inline]
    pub const fn as_array(&self) -> &[T; 4] {
        unsafe { std::mem::transmute::<_, &[T; 4]>(self) }
    }

    #[inline]
    pub const fn as_array_mut(&mut self) -> &mut [T; 4] {
        unsafe { std::mem::transmute::<_, &mut [T; 4]>(self) }
    }

    pub const fn as_slice(&self) -> &[T] {
        self.as_array()
    }

    #[inline]
    pub fn map<O, F: FnMut(T) -> O>(self, mut f: F) -> Vec4<O> {
        Vec4::new((f)(self.x), (f)(self.y), (f)(self.z), (f)(self.w))
    }

    #[inline]
    pub fn try_map<O, E, F: FnMut(T) -> Result<O, E>>(self, mut f: F) -> Result<Vec4<O>, E> {
        Ok(Vec4::new(
            (f)(self.x)?,
            (f)(self.y)?,
            (f)(self.z)?,
            (f)(self.w)?,
        ))
    }

    #[inline]
    pub fn any<F: FnMut(T) -> bool>(self, mut f: F) -> bool {
        (f)(self.x) || (f)(self.y) || (f)(self.z) || (f)(self.w)
    }

    #[inline]
    pub fn all<F: FnMut(T) -> bool>(self, mut f: F) -> bool {
        (f)(self.x) && (f)(self.y) && (f)(self.z) && (f)(self.w)
    }
}

impl<T> Vec4<T>
where
    T: Clamp<T>,
{
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self::new(
            self.x.clamp_t(min.x, max.x),
            self.y.clamp_t(min.y, max.y),
            self.z.clamp_t(min.z, max.z),
            self.w.clamp_t(min.w, max.w),
        )
    }
}

impl<T> Vec4<T>
where
    T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T>,
{
    #[inline]
    pub fn length2(self) -> T {
        (self.x * self.x) + (self.y * self.y) + (self.z * self.z) + (self.w * self.w)
    }
}

impl<T> Vec4<T>
where
    T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T> + Sqrt<T>,
{
    #[inline]
    pub fn length(self) -> T {
        self.length2().sqrt()
    }
}

impl<T> Vec4<T>
where
    T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T> + std::ops::Sub<Output = T>,
{
    #[inline]
    pub fn distance2(self, other: Self) -> T {
        (self - other).length2()
    }
}

impl<T> Vec4<T>
where
    T: Copy
        + std::ops::Mul<Output = T>
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + Sqrt<T>,
{
    #[inline]
    pub fn distance(self, other: Self) -> T {
        (self - other).length()
    }
}

impl<T> Vec4<T>
where
    T: Copy + std::ops::Mul<Output = T> + std::ops::Add<Output = T> + Sqrt<T>,
{
    #[inline]
    pub fn normalize(self) -> Self {
        self * self.length()
    }
}

impl<T> DotProduct for Vec4<T>
where
    T: std::ops::Mul<Output = T> + std::ops::Add<Output = T>,
{
    type Output = T;
    #[inline]
    fn dot(self, rhs: Self) -> Self::Output {
        (self.x * rhs.x) + (self.y * rhs.y) + (self.z * rhs.z) + (self.w * rhs.w)
    }
}

impl<T> Vec4<T>
where
    T: One<T>,
    T: Zero<T>,
{
    pub const X: Self = Self::new(T::ONE, T::ZERO, T::ZERO, T::ZERO);
    pub const Y: Self = Self::new(T::ZERO, T::ONE, T::ZERO, T::ZERO);
    pub const Z: Self = Self::new(T::ZERO, T::ZERO, T::ONE, T::ZERO);
    pub const W: Self = Self::new(T::ZERO, T::ZERO, T::ZERO, T::ONE);
}

impl<T> Vec4<T>
where
    T: Zero<T>,
{
    pub const ZERO: Self = Self::new(T::ZERO, T::ZERO, T::ZERO, T::ZERO);
}

impl<T> Vec4<T>
where
    T: One<T>,
{
    pub const ONE: Self = Self::new(T::ONE, T::ONE, T::ONE, T::ONE);
}

impl<T> Vec4<T>
where
    T: NegOne<T>,
{
    pub const NEG_ONE: Self = Self::new(T::NEG_ONE, T::NEG_ONE, T::NEG_ONE, T::NEG_ONE);
}

impl<T> Vec4<T>
where
    T: Infinity<T>,
{
    pub const INFINITY: Self = Self::new(T::INFINITY, T::INFINITY, T::INFINITY, T::INFINITY);
}

impl<T> Vec4<T>
where
    T: NaN<T> + Copy,
{
    pub const NAN: Self = Self::new(T::NAN, T::NAN, T::NAN, T::NAN);
    #[inline]
    pub fn any_nan(self) -> bool {
        self.any(|v| v.is_nan_t())
    }
}

impl<T> From<&[T; 4]> for Vec4<T>
where
    T: Copy,
{
    #[inline]
    fn from(value: &[T; 4]) -> Self {
        Self::new(value[0], value[1], value[2], value[3])
    }
}

impl<T> From<&[T; 3]> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: &[T; 3]) -> Self {
        Self::new(value[0], value[1], value[2], T::ZERO)
    }
}

impl<T> From<&[T; 2]> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: &[T; 2]) -> Self {
        Self::new(value[0], value[1], T::ZERO, T::ZERO)
    }
}

impl<T> From<[T; 4]> for Vec4<T>
where
    T: Copy,
{
    #[inline]
    fn from(value: [T; 4]) -> Self {
        Self::new(value[0], value[1], value[2], value[3])
    }
}

impl<T> From<[T; 3]> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: [T; 3]) -> Self {
        Self::new(value[0], value[1], value[2], T::ZERO)
    }
}

impl<T> From<[T; 2]> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: [T; 2]) -> Self {
        Self::new(value[0], value[1], T::ZERO, T::ZERO)
    }
}

impl<T> From<&(T, T, T, T)> for Vec4<T>
where
    T: Copy,
{
    #[inline]
    fn from(value: &(T, T, T, T)) -> Self {
        Self::new(value.0, value.1, value.2, value.3)
    }
}

impl<T> From<&(T, T, T)> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: &(T, T, T)) -> Self {
        Self::new(value.0, value.1, value.2, T::ZERO)
    }
}

impl<T> From<&(T, T)> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: &(T, T)) -> Self {
        Self::new(value.0, value.1, T::ZERO, T::ZERO)
    }
}

impl<T> From<(T, T, T, T)> for Vec4<T>
where
    T: Copy,
{
    #[inline]
    fn from(value: (T, T, T, T)) -> Self {
        Self::new(value.0, value.1, value.2, value.3)
    }
}

impl<T> From<(T, T, T)> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: (T, T, T)) -> Self {
        Self::new(value.0, value.1, value.2, T::ZERO)
    }
}

impl<T> From<(T, T)> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: (T, T)) -> Self {
        Self::new(value.0, value.1, T::ZERO, T::ZERO)
    }
}

impl<T> From<Vec3<T>> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: Vec3<T>) -> Self {
        Self::new(value.x, value.y, value.z, T::ZERO)
    }
}

impl<T> From<Vec2<T>> for Vec4<T>
where
    T: Copy + Zero<T>,
{
    #[inline]
    fn from(value: Vec2<T>) -> Self {
        Self::new(value.x, value.y, T::ZERO, T::ZERO)
    }
}

impl<T: std::ops::Add<Output = T>> std::ops::Add for Vec4<T> {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            w: self.w + other.w,
            z: self.z + other.z,
        }
    }
}

impl<T: std::ops::AddAssign> std::ops::AddAssign for Vec4<T> {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
        self.w += other.w;
    }
}

impl<T: std::ops::Sub<Output = T>> std::ops::Sub for Vec4<T> {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
            w: self.w - other.w,
        }
    }
}

impl<T: std::ops::SubAssign> std::ops::SubAssign for Vec4<T> {
    #[inline]
    fn sub_assign(&mut self, other: Self) {
        self.x -= other.x;
        self.y -= other.y;
        self.z -= other.z;
        self.w -= other.w;
    }
}

impl<T: std::ops::Mul<Output = T> + Copy> std::ops::Mul<T> for Vec4<T> {
    type Output = Self;

    #[inline]
    fn mul(self, other: T) -> Self {
        Self {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
            w: self.w * other,
        }
    }
}

impl<T: std::ops::MulAssign + Copy> std::ops::MulAssign<T> for Vec4<T> {
    #[inline]
    fn mul_assign(&mut self, other: T) {
        self.x *= other;
        self.y *= other;
        self.z *= other;
        self.w *= other;
    }
}

impl<T: std::ops::Div<Output = T> + Copy> std::ops::Div<T> for Vec4<T> {
    type Output = Self;

    #[inline]
    fn div(self, other: T) -> Self {
        Self {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
            w: self.w / other,
        }
    }
}

impl<T: std::ops::DivAssign + Copy> std::ops::DivAssign<T> for Vec4<T> {
    #[inline]
    fn div_assign(&mut self, other: T) {
        self.x /= other;
        self.y /= other;
        self.z /= other;
        self.w /= other;
    }
}

impl<T: std::ops::Neg<Output = T>> std::ops::Neg for Vec4<T> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
            w: -self.w,
        }
    }
}

impl<T, I> std::ops::Index<I> for Vec4<T>
where
    T: Copy,
    I: std::slice::SliceIndex<[T], Output = T>,
{
    type Output = T;

    #[inline]
    fn index(&self, i: I) -> &Self::Output {
        &self.as_array()[i]
    }
}

impl<T, I> std::ops::IndexMut<I> for Vec4<T>
where
    T: Copy,
    I: std::slice::SliceIndex<[T], Output = T>,
{
    #[inline]
    fn index_mut(&mut self, i: I) -> &mut Self::Output {
        &mut self.as_array_mut()[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_assign() {
        let mut a = Vec4::<f32>::default();
        let b = Vec4::<f32>::default();
        a *= b;
    }

    #[test]
    fn idx() {
        Vec4::<f32>::default()[0];
        Vec4::<f32>::default()[0] = 1.0;
    }
}

// TODO generate swizzels
// TODO generate constants like UP DOWN LEFT
