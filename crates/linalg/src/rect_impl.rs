use super::{Rect, Vec2};

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
    T: Copy,
{
    pub fn position(&self) -> Vec2<T> {
        Vec2::new(self.x, self.y)
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
    T: std::ops::Add<Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Sub<Output = T>
        + PartialOrd
        + Copy,
{
    pub fn intersection(self, other: Self) -> Self {
        let x1 = partial_max(self.x, other.x);
        let y1 = partial_max(self.y, other.y);
        let x2 = partial_min(self.right(), other.right());
        let y2 = partial_min(self.bottom(), other.bottom());

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
        + Copy
        + Ord,
{
    pub fn margin_left(&self, margin: T) -> Self {
        let margin = margin.min(self.width);
        Self {
            x: self.x + margin,
            y: self.y,
            width: self.width - margin,
            height: self.height,
        }
    }

    pub fn margin_right(&self, margin: T) -> Self {
        let margin = margin.min(self.width);
        Self {
            x: self.x,
            y: self.y,
            width: self.width - margin,
            height: self.height,
        }
    }

    pub fn margin_top(&self, margin: T) -> Self {
        let margin = margin.min(self.width);
        Self {
            x: self.x,
            y: self.y + margin,
            width: self.width,
            height: self.height - margin,
        }
    }

    pub fn margin_bottom(&self, margin: T) -> Self {
        let margin = margin.min(self.width);
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
    T: std::ops::Add<Output = T> + std::ops::Sub<Output = T> + PartialOrd + Copy,
{
    pub fn clamp_within(&self, outer: Self) -> Self {
        let left = partial_max(self.left(), outer.left());
        let right = partial_min(self.right(), outer.right());
        let top = partial_max(self.top(), outer.top());
        let bottom = partial_min(self.bottom(), outer.bottom());
        Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        }
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

fn partial_max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b {
        a
    } else if a < b {
        b
    } else {
        a
    }
}

fn partial_min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        a
    } else if a > b {
        b
    } else {
        a
    }
}
