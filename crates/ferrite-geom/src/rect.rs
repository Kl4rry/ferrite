use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect<T = usize> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

impl<T> Rect<T> {
    pub fn new(x: T, y: T, width: T, height: T) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

impl<T> Rect<T>
where
    T: Add<Output = T> + Mul<Output = T> + PartialOrd + Ord + num_traits::SaturatingSub + Copy,
{
    pub fn intersection(self, other: Self) -> Self {
        let x1 = std::cmp::max(self.x, other.x);
        let y1 = std::cmp::max(self.y, other.y);
        let x2 = std::cmp::min(self.right(), other.right());
        let y2 = std::cmp::min(self.bottom(), other.bottom());

        Self {
            x: x1,
            y: y1,
            width: x2.saturating_sub(&x1),
            height: y2.saturating_sub(&y1),
        }
    }
}

impl<T> Rect<T>
where
    T: Add<Output = T> + Mul<Output = T> + PartialOrd + Copy,
{
    pub fn intersects(&self, other: &Self) -> bool {
        self.x < self.x + self.width
            && self.x + self.width > other.x
            && self.y < self.y + self.height
            && self.y + self.height > other.y
    }

    pub fn scale(&self, x: T, y: T) -> Self {
        let mut copy = *self;
        copy.width = copy.width * x;
        copy.height = copy.height * y;
        copy
    }

    pub fn contains(&self, point: Vec2<T>) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x <= self.x + self.width
            && point.y <= self.y + self.height
    }

    pub fn area(&self) -> T {
        self.width * self.height
    }

    pub fn left(&self) -> T {
        self.x
    }

    pub fn right(&self) -> T {
        self.x + self.width
    }

    pub fn top(&self) -> T {
        self.y
    }

    pub fn bottom(&self) -> T {
        self.y + self.height
    }
}

impl<T> Rect<T>
where
    T: Add<Output = T> + num_traits::int::PrimInt + num_traits::SaturatingSub,
{
    pub fn clamp_within(&self, outer: Self) -> Self {
        let left = self.left().max(outer.left());
        let right = self.right().min(outer.right());
        let top = self.top().max(outer.top());
        let bottom = self.bottom().min(outer.bottom());
        Rect {
            x: left,
            y: top,
            width: right.saturating_sub(left),
            height: bottom.saturating_sub(top),
        }
    }
}

impl From<Rect> for tui::layout::Rect {
    fn from(rect: Rect) -> tui::layout::Rect {
        tui::layout::Rect {
            x: rect.x.try_into().unwrap(),
            y: rect.y.try_into().unwrap(),
            width: rect.width.try_into().unwrap(),
            height: rect.height.try_into().unwrap(),
        }
    }
}

impl From<tui::layout::Rect> for Rect {
    fn from(rect: tui::layout::Rect) -> Rect {
        Rect {
            x: rect.x.into(),
            y: rect.y.into(),
            width: rect.width.into(),
            height: rect.height.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vec2<T = usize> {
    pub x: T,
    pub y: T,
}

impl<T> Vec2<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}
