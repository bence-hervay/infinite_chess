use std::ops::{Add, Mul, Neg, Sub};

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl Coord {
    pub const ORIGIN: Coord = Coord { x: 0, y: 0 };

    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn chebyshev_norm(self) -> i32 {
        self.x.abs().max(self.y.abs())
    }

    pub fn in_box(self, bound: i32) -> bool {
        self.x.abs() <= bound && self.y.abs() <= bound
    }
}

impl Add for Coord {
    type Output = Coord;

    fn add(self, rhs: Coord) -> Self::Output {
        Coord::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Coord {
    type Output = Coord;

    fn sub(self, rhs: Coord) -> Self::Output {
        Coord::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Neg for Coord {
    type Output = Coord;

    fn neg(self) -> Self::Output {
        Coord::new(-self.x, -self.y)
    }
}

impl Mul<i32> for Coord {
    type Output = Coord;

    fn mul(self, rhs: i32) -> Coord {
        Coord {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}
