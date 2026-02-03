use std::ops::{Add, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl Coord {
    pub const ORIGIN: Coord = Coord { x: 0, y: 0 };

    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn chebyshev_norm(self) -> i32 {
        self.x.abs().max(self.y.abs())
    }

    #[inline]
    pub fn in_linf_bound(self, bound: i32) -> bool {
        self.x.abs() <= bound && self.y.abs() <= bound
    }
}

impl Add for Coord {
    type Output = Coord;

    #[inline]
    fn add(self, rhs: Coord) -> Self::Output {
        Coord::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Coord {
    type Output = Coord;

    #[inline]
    fn sub(self, rhs: Coord) -> Self::Output {
        Coord::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Neg for Coord {
    type Output = Coord;

    #[inline]
    fn neg(self) -> Self::Output {
        Coord::new(-self.x, -self.y)
    }
}

impl Mul<i32> for Coord {
    type Output = Coord;

    #[inline]
    fn mul(self, rhs: i32) -> Coord {
        Coord {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

/// The 8 king steps around the origin.
pub const KING_STEPS: [Coord; 8] = [
    Coord { x: -1, y: -1 },
    Coord { x: -1, y: 0 },
    Coord { x: -1, y: 1 },
    Coord { x: 0, y: -1 },
    Coord { x: 0, y: 1 },
    Coord { x: 1, y: -1 },
    Coord { x: 1, y: 0 },
    Coord { x: 1, y: 1 },
];
