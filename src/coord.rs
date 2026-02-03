use std::ops::{Add, AddAssign, Sub};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Coord {
    pub x: i16,
    pub y: i16,
}

impl Coord {
    #[inline]
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn l_inf(self) -> i16 {
        self.x.abs().max(self.y.abs())
    }

    #[inline]
    pub fn l1(self) -> i16 {
        self.x.abs() + self.y.abs()
    }
}

impl Add for Coord {
    type Output = Coord;

    #[inline]
    fn add(self, rhs: Coord) -> Coord {
        Coord::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Coord {
    #[inline]
    fn add_assign(&mut self, rhs: Coord) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Coord {
    type Output = Coord;

    #[inline]
    fn sub(self, rhs: Coord) -> Coord {
        Coord::new(self.x - rhs.x, self.y - rhs.y)
    }
}

#[inline]
pub fn signum_i16(v: i16) -> i16 {
    if v > 0 {
        1
    } else if v < 0 {
        -1
    } else {
        0
    }
}
