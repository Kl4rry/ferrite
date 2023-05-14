use std::ops::{Add, AddAssign, Sub, SubAssign};

use num_traits::PrimInt;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point<T: PrimInt> {
    // line must be first for the ord derive to work correctly
    pub line: T,
    pub column: T,
}

impl<T: PrimInt> Point<T> {
    pub fn new(column: T, line: T) -> Self {
        Self { column, line }
    }
}

impl<T: PrimInt> Add<Point<T>> for Point<T> {
    type Output = Point<T>;

    fn add(self, rhs: Point<T>) -> Self::Output {
        Self {
            line: self.line + rhs.line,
            column: self.column + rhs.column,
        }
    }
}

impl<T: PrimInt + AddAssign> AddAssign<Point<T>> for Point<T> {
    fn add_assign(&mut self, rhs: Point<T>) {
        self.line += rhs.line;
        self.column += rhs.column;
    }
}

impl<T: PrimInt> Sub<Point<T>> for Point<T> {
    type Output = Point<T>;

    fn sub(self, rhs: Point<T>) -> Self::Output {
        Self {
            line: self.line - rhs.line,
            column: self.column - rhs.column,
        }
    }
}

impl<T: PrimInt + SubAssign> SubAssign<Point<T>> for Point<T> {
    fn sub_assign(&mut self, rhs: Point<T>) {
        self.line -= rhs.line;
        self.column -= rhs.column;
    }
}
