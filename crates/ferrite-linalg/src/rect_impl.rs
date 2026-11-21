use crate::{
    Rect, Vec2, max, min,
    traits::{One, SaturatingSub, Two},
};

impl<T> Rect<T> {
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_pos_and_size(pos: impl Into<Vec2<T>>, size: impl Into<Vec2<T>>) -> Self {
        let pos = pos.into();
        let size = size.into();
        Self {
            x: pos.x,
            y: pos.y,
            width: size.x,
            height: size.y,
        }
    }

    pub fn map<O, F: FnMut(T) -> O>(self, mut f: F) -> Rect<O> {
        Rect::new((f)(self.x), (f)(self.y), (f)(self.width), (f)(self.height))
    }

    pub fn try_map<O, E, F: FnMut(T) -> Result<O, E>>(self, mut f: F) -> Result<Rect<O>, E> {
        Ok(Rect::new(
            (f)(self.x)?,
            (f)(self.y)?,
            (f)(self.width)?,
            (f)(self.height)?,
        ))
    }
}

impl<T> Rect<T>
where
    T: Copy,
{
    pub fn position(&self) -> Vec2<T> {
        Vec2::new(self.x, self.y)
    }

    pub fn size(&self) -> Vec2<T> {
        Vec2::new(self.width, self.height)
    }

    pub fn left(&self) -> T {
        self.x
    }

    pub fn top(&self) -> T {
        self.y
    }
}

impl<T> Rect<T>
where
    T: Two<T> + Copy + std::ops::Add<Output = T> + std::ops::Div<Output = T>,
{
    pub fn from_center_size(center: impl Into<Vec2<T>>, size: impl Into<Vec2<T>>) -> Self {
        let center = center.into();
        let size = size.into();
        Self {
            x: center.x + (size.x / T::TWO),
            y: center.y + (size.y / T::TWO),
            width: size.x,
            height: size.y,
        }
    }
}

impl<T> Rect<T>
where
    T: std::ops::Add<Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + Copy,
{
    pub fn intersection(self, other: Self) -> Self {
        let x1 = max(self.x, other.x);
        let y1 = max(self.y, other.y);
        let x2 = min(self.right(), other.right());
        let y2 = min(self.bottom(), other.bottom());

        Self {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
}

impl<T> Rect<T>
where
    T: std::ops::Mul<Output = T> + Copy,
{
    pub fn scale(&self, x: T, y: T) -> Self {
        let mut copy = *self;
        copy.width = copy.width * x;
        copy.height = copy.height * y;
        copy
    }

    pub fn area(&self) -> T {
        self.width * self.height
    }
}

impl<T> Rect<T>
where
    T: std::ops::Add<Output = T> + Copy,
{
    pub fn right(&self) -> T {
        self.x + self.width
    }
    pub fn bottom(&self) -> T {
        self.y + self.height
    }
}

impl<T> Rect<T>
where
    T: std::ops::Add<Output = T> + PartialOrd + Copy,
{
    pub fn intersects(&self, other: &Self) -> bool {
        self.x < self.x + self.width
            && self.x + self.width > other.x
            && self.y < self.y + self.height
            && self.y + self.height > other.y
    }

    pub fn contains(&self, point: Vec2<T>) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }
}

impl<T> Rect<T>
where
    T: std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Mul<Output = T>
        + PartialOrd
        + Copy,
{
    pub fn margin_left(&self, margin: T) -> Self {
        let margin = min(margin, self.width);
        Self {
            x: self.x + margin,
            y: self.y,
            width: self.width - margin,
            height: self.height,
        }
    }

    pub fn margin_right(&self, margin: T) -> Self {
        let margin = min(margin, self.width);
        Self {
            x: self.x,
            y: self.y,
            width: self.width - margin,
            height: self.height,
        }
    }

    pub fn margin_top(&self, margin: T) -> Self {
        let margin = min(margin, self.height);
        Self {
            x: self.x,
            y: self.y + margin,
            width: self.width,
            height: self.height - margin,
        }
    }

    pub fn margin_bottom(&self, margin: T) -> Self {
        let margin = min(margin, self.height);
        Self {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height - margin,
        }
    }
}

impl<T> Rect<T>
where
    T: std::ops::Add<Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Sub<Output = T>
        + SaturatingSub
        + One<T>
        + PartialOrd
        + Copy,
{
    pub fn border(&self) -> [Rect<T>; 4] {
        let left = self.margin_right(self.width.saturating_sub_t(T::ONE));
        let right = self.margin_left(self.width.saturating_sub_t(T::ONE));
        let top = self.margin_bottom(self.height.saturating_sub_t(T::ONE));
        let bottom = self.margin_top(self.height.saturating_sub_t(T::ONE));
        [left, right, top, bottom]
    }
}

impl<T> Rect<T>
where
    T: std::ops::Add<Output = T> + std::ops::Sub<Output = T> + PartialOrd + Copy,
{
    pub fn clamp_within(&self, outer: Self) -> Self {
        let left = max(self.left(), outer.left());
        let right = min(self.right(), outer.right());
        let top = max(self.top(), outer.top());
        let bottom = min(self.bottom(), outer.bottom());
        Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
    }
}

impl<T: std::ops::Mul<Output = T> + Copy> std::ops::Mul<Vec2<T>> for Rect<T> {
    type Output = Self;

    #[inline]
    fn mul(self, other: Vec2<T>) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            width: self.width * other.x,
            height: self.height * other.y,
        }
    }
}

impl<T: std::ops::MulAssign + Copy> std::ops::MulAssign<Vec2<T>> for Rect<T> {
    #[inline]
    fn mul_assign(&mut self, other: Vec2<T>) {
        self.x *= other.x;
        self.y *= other.y;
        self.width *= other.x;
        self.height *= other.y;
    }
}

impl<T: std::ops::Div<Output = T> + Copy> std::ops::Div<Vec2<T>> for Rect<T> {
    type Output = Self;

    #[inline]
    fn div(self, other: Vec2<T>) -> Self {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
            width: self.width / other.x,
            height: self.height / other.y,
        }
    }
}

impl<T: std::ops::DivAssign + Copy> std::ops::DivAssign<Vec2<T>> for Rect<T> {
    #[inline]
    fn div_assign(&mut self, other: Vec2<T>) {
        self.x /= other.x;
        self.y /= other.y;
        self.width /= other.x;
        self.height /= other.y;
    }
}

impl<T> From<&[T; 4]> for Rect<T>
where
    T: Copy,
{
    #[inline]
    fn from(value: &[T; 4]) -> Self {
        Self::new(value[0], value[1], value[2], value[3])
    }
}

impl<T> From<[T; 4]> for Rect<T>
where
    T: Copy,
{
    #[inline]
    fn from(value: [T; 4]) -> Self {
        Self::new(value[0], value[1], value[2], value[3])
    }
}

impl<T> From<(T, T, T, T)> for Rect<T>
where
    T: Copy,
{
    #[inline]
    fn from(value: (T, T, T, T)) -> Self {
        Self::new(value.0, value.1, value.2, value.3)
    }
}

/*impl From<Rect> for tui_core::layout::Rect {
    fn from(rect: Rect) -> tui_core::layout::Rect {
        let x = rect.x.try_into();
        let y = rect.y.try_into();
        let width = rect.width.try_into();
        let height = rect.height.try_into();

        if x.is_err() || y.is_err() || width.is_err() || height.is_err() {
            panic!(
                "x: {} y: {} width: {} height: {}",
                rect.x, rect.y, rect.width, rect.height
            );
        }

        tui_core::layout::Rect {
            x: x.unwrap(),
            y: y.unwrap(),
            width: width.unwrap(),
            height: height.unwrap(),
        }
    }
}*/

/*impl From<tui_core::layout::Rect> for Rect {
    fn from(rect: tui_core::layout::Rect) -> Rect {
        Rect {
            x: rect.x.into(),
            y: rect.y.into(),
            width: rect.width.into(),
            height: rect.height.into(),
        }
    }
}*/
