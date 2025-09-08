use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn area(&mut self) -> T {
        self.width * self.height
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
